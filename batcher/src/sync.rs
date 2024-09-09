/*!
Run channels on regular OS threads.
*/

use std::{
    future::{self, Future},
    io,
    pin::pin,
    sync::{Arc, Condvar, Mutex, OnceLock},
    task, thread,
    time::{Duration, Instant},
};

use crate::{BatchError, Channel, Receiver, Sender};

/**
Run the receiver synchronously.

This method spawns a background thread and runs [`Receiver::exec`] on it. The handle will join when the [`Sender`] is dropped.
*/
pub fn spawn<T: Channel + Send + 'static>(
    thread_name: impl Into<String>,
    receiver: Receiver<T>,
    mut on_batch: impl FnMut(T) -> Result<(), BatchError<T>> + Send + 'static,
) -> io::Result<thread::JoinHandle<()>>
where
    T::Item: Send + 'static,
{
    thread::Builder::new()
        .name(thread_name.into())
        .spawn(move || {
            block_on(receiver.exec(
                |delay| future::ready(thread::sleep(delay)),
                move |batch| future::ready(on_batch(batch)),
            ))
        })
}

/**
Wait for a channel running on a regular OS thread to process all items active at the point this call was made.
*/
pub fn blocking_flush<T: Channel>(sender: &Sender<T>, timeout: Duration) -> bool {
    let notifier = Trigger::new();

    sender.when_flushed({
        let notifier = notifier.clone();

        move || {
            notifier.trigger();
        }
    });

    notifier.wait_timeout(timeout)
}

/**
Wait for a channel to send a message, blocking if the channel is at capacity.
*/
pub fn blocking_send<T: Channel>(
    sender: &Sender<T>,
    msg: T::Item,
    timeout: Duration,
) -> Result<(), BatchError<T::Item>> {
    block_on(sender.send_or_wait(msg, timeout, |sender, timeout| {
        let notifier = Trigger::new();

        sender.when_empty({
            let notifier = notifier.clone();

            move || {
                let _ = notifier.trigger();
            }
        });

        notifier.wait_timeout(timeout);

        future::ready(())
    }))
}

#[derive(Clone)]
struct Trigger(Arc<(Mutex<bool>, Condvar)>);

impl Trigger {
    pub fn new() -> Self {
        Trigger(Arc::new((Mutex::new(false), Condvar::new())))
    }

    pub fn trigger(self) {
        *(self.0).0.lock().unwrap() = true;
        (self.0).1.notify_all();
    }

    pub fn wait_timeout(&self, mut timeout: Duration) -> bool {
        let mut flushed_slot = (self.0).0.lock().unwrap();
        loop {
            // If we flushed then return
            // This condition may already be set before we start waiting
            if *flushed_slot {
                return true;
            }

            // If the timeout is 0 then return
            // There's no point waiting for the condition
            if timeout == Duration::ZERO {
                return false;
            }

            let now = Instant::now();
            match (self.0).1.wait_timeout(flushed_slot, timeout).unwrap() {
                (flushed, r) if !r.timed_out() => {
                    flushed_slot = flushed;

                    // Reduce the remaining timeout just in case we didn't time out,
                    // but woke up spuriously for some reason
                    timeout = match timeout.checked_sub(now.elapsed()) {
                        Some(timeout) => timeout,
                        // We didn't time out, but got close enough that we should now anyways
                        None => {
                            return *flushed_slot;
                        }
                    };

                    continue;
                }
                // Timed out
                (flushed, _) => {
                    return *flushed;
                }
            }
        }
    }
}

fn block_on<R>(fut: impl Future<Output = R>) -> R {
    static WAKER: OnceLock<Arc<NeverWake>> = OnceLock::new();

    // A waker that does nothing; the tasks it runs are fully
    // synchronous so there's never any notifications to issue
    struct NeverWake;

    impl task::Wake for NeverWake {
        fn wake(self: Arc<Self>) {}
    }

    // The future is polled to completion here, so we can pin
    // it directly on the stack
    let mut fut = pin!(fut);

    // Get a context for our synchronous task
    let waker = WAKER.get_or_init(|| Arc::new(NeverWake)).clone().into();
    let mut cx = task::Context::from_waker(&waker);

    // Drive the task to completion; it should complete in one go,
    // but may eagerly return as soon as it hits an await point, so
    // just to be sure we continuously poll it
    loop {
        match fut.as_mut().poll(&mut cx) {
            task::Poll::Ready(r) => return r,
            task::Poll::Pending => continue,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::Receiver;

    use std::{sync::mpsc, thread};

    enum SenderCommand<T> {
        BlockingSend(T, Duration),
        Stop,
    }

    impl<T> SenderCommand<T> {
        fn blocking_send(msg: T, timeout: Duration) -> Self {
            SenderCommand::BlockingSend(msg, timeout)
        }

        fn stop() -> Self {
            SenderCommand::Stop
        }
    }

    enum ReceiverCommand<T> {
        ProcessBatch(Box<dyn FnOnce(Vec<T>) -> Result<(), BatchError<Vec<T>>> + Send>),
    }

    impl<T> ReceiverCommand<T> {
        fn process_batch(
            f: impl FnOnce(Vec<T>) -> Result<(), BatchError<Vec<T>>> + Send + 'static,
        ) -> Self {
            ReceiverCommand::ProcessBatch(Box::new(f))
        }
    }

    fn spawn_sender<T: Send + 'static>(
        sender: Sender<Vec<T>>,
    ) -> (mpsc::Sender<SenderCommand<T>>, thread::JoinHandle<()>) {
        let (tx, rx) = mpsc::channel();

        let handle = thread::spawn(move || loop {
            match rx.recv().unwrap() {
                SenderCommand::BlockingSend(msg, timeout) => {
                    let _ = blocking_send(&sender, msg, timeout);
                }
                SenderCommand::Stop => return,
            }
        });

        (tx, handle)
    }

    fn spawn_receiver<T: Send + 'static>(
        receiver: Receiver<Vec<T>>,
    ) -> (mpsc::Sender<ReceiverCommand<T>>, thread::JoinHandle<()>) {
        let (tx, rx) = mpsc::channel();

        let handle = spawn("test_receiver", receiver, move |batch| match rx.recv() {
            Ok(ReceiverCommand::ProcessBatch(p)) => p(batch),
            _ => Ok(()),
        })
        .unwrap();

        (tx, handle)
    }

    #[test]
    fn send_recv() {
        let received = Arc::new(Mutex::new(0));

        let (sender, receiver) = crate::bounded(10);

        let (receiver, receiver_handle) = spawn_receiver(receiver);

        // Send some messages
        for _ in 0..10 {
            sender.send(());
        }

        // Process the messages
        // This should be done in a single batch, but may be at most 2
        for _ in 0..2 {
            receiver
                .send(ReceiverCommand::process_batch({
                    let received = received.clone();

                    move |batch| {
                        *received.lock().unwrap() += batch.len();

                        Ok(())
                    }
                }))
                .unwrap();
        }

        // Wait for the receiver to process the batches
        while { *received.lock().unwrap() } != 10 {}

        // Shutdown
        drop(sender);
        receiver_handle.join().unwrap();
    }

    #[test]
    fn send_full_capacity() {
        let received = Arc::new(Mutex::new(Vec::new()));

        let (sender, receiver) = crate::bounded(5);

        // Send some messages
        for i in 0..10 {
            sender.send(i);
        }

        // Spawn the receiver after attempting to send all messages
        let (receiver, receiver_handle) = spawn_receiver(receiver);

        // Everything should be processed in a single batch
        receiver
            .send(ReceiverCommand::process_batch({
                let received = received.clone();

                move |batch| {
                    received.lock().unwrap().extend(batch);

                    Ok(())
                }
            }))
            .unwrap();

        // Only the last 5 messages should be processed
        // The others were truncated
        while { received.lock().unwrap().len() } == 0 {}
        assert_eq!(vec![5, 6, 7, 8, 9], *received.lock().unwrap());

        // Shutdown
        drop(sender);
        receiver_handle.join().unwrap();
    }

    #[test]
    fn blocking_send_full_capacity() {
        let received = Arc::new(Mutex::new(0));

        let (sender, receiver) = crate::bounded(5);

        let (sender, sender_handle) = spawn_sender(sender);
        let (receiver, receiver_handle) = spawn_receiver(receiver);

        // Send some messages
        for _ in 0..10 {
            sender
                .send(SenderCommand::blocking_send((), Duration::from_secs(1)))
                .unwrap();
        }

        // The receiver may process in (up to) 10 batches
        for _ in 0..10 {
            receiver
                .send(ReceiverCommand::process_batch({
                    let received = received.clone();

                    move |batch| {
                        *received.lock().unwrap() += batch.len();

                        Ok(())
                    }
                }))
                .unwrap();
        }

        // Wait for the receiver to process the batches
        while { *received.lock().unwrap() } != 10 {}

        // Shutdown
        sender.send(SenderCommand::stop()).unwrap();
        sender_handle.join().unwrap();
        receiver_handle.join().unwrap();
    }

    #[test]
    fn blocking_send_full_capacity_timeout() {
        let received = Arc::new(Mutex::new(Vec::new()));

        let (sender, receiver) = crate::bounded(5);

        let (sender, sender_handle) = spawn_sender(sender);
        let (receiver, _) = spawn_receiver(receiver);

        // Send some messages
        for i in 0..10 {
            sender
                .send(SenderCommand::blocking_send(i, Duration::from_millis(1)))
                .unwrap();
        }

        // Only process a single batch
        receiver
            .send(ReceiverCommand::process_batch({
                let received = received.clone();

                move |batch| {
                    received.lock().unwrap().extend(batch);

                    Ok(())
                }
            }))
            .unwrap();

        // Wait for the receiver to process the batch
        while { received.lock().unwrap().len() } == 0 {}

        // Shutdown
        // The blocking sends will time out
        sender.send(SenderCommand::stop()).unwrap();
        sender_handle.join().unwrap();
    }

    #[test]
    fn flush_empty() {
        let (sender, receiver) = crate::bounded(10);

        let (_, receiver_handle) = spawn_receiver::<()>(receiver);

        // There's nothing to flush; should return immediately
        assert!(blocking_flush(&sender, Duration::ZERO));

        // Shutdown
        drop(sender);
        receiver_handle.join().unwrap();
    }

    #[test]
    fn flush_active() {
        let (sender, receiver) = crate::bounded(10);

        let (receiver, receiver_handle) = spawn_receiver::<()>(receiver);

        // Start a batch
        for _ in 0..3 {
            sender.send(());
        }

        // Wait for the receiver to start processing a batch
        while !sender.shared.state.lock().unwrap().is_in_batch {}

        // Start another batch
        for _ in 0..3 {
            sender.send(());
        }

        thread::scope(|s| {
            // Start the flush
            let handle = s.spawn(|| blocking_flush(&sender, Duration::from_secs(1)));

            // Process both batches
            for _ in 0..2 {
                receiver
                    .send(ReceiverCommand::process_batch(|_| Ok(())))
                    .unwrap();
                receiver
                    .send(ReceiverCommand::process_batch(|_| Ok(())))
                    .unwrap();
            }

            // Wait for the flush to complete
            handle.join().unwrap();

            assert_eq!(
                0,
                sender.shared.state.lock().unwrap().next_batch.channel.len()
            );
        });

        // Shutdown
        drop(sender);
        receiver_handle.join().unwrap();
    }
}
