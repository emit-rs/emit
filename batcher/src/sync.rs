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

use crate::{BatchError, Channel, Receiver, Sender, Wait};

/**
Run the receiver synchronously.

This method spawns a background thread and runs [`Receiver::exec`] on it. The handle will join when the [`Sender`] is dropped.

This method will return an error on the `wasm32-unknown-unknown` target.
*/
pub fn spawn<T: Channel + Send + 'static>(
    thread_name: impl Into<String>,
    receiver: Receiver<T>,
    on_batch: impl FnMut(T) -> Result<(), BatchError<T>> + Send + 'static,
) -> io::Result<thread::JoinHandle<()>>
where
    T::Item: Send + 'static,
{
    #![allow(unreachable_code)]

    #[cfg(all(
        target_arch = "wasm32",
        target_vendor = "unknown",
        target_os = "unknown"
    ))]
    {
        let _ = (thread_name, receiver, on_batch);

        return Err(io::Error::new(
            io::ErrorKind::Other,
            "blocking channel spawning is not supported on this platform",
        ));
    }

    let mut on_batch = on_batch;

    thread::Builder::new()
        .name(thread_name.into())
        .spawn(move || {
            let shared = receiver.shared.clone();

            block_on(receiver.exec_inner(
                move |wait, delay| {
                    future::ready(match wait {
                        // Idle waits can be cut short by a sender notification
                        Wait::Idle => {
                            shared.receiver_notifier.sync.wait_timeout(delay);
                        }
                        // Retry waits are backoff on a failing batch; don't cut them short
                        Wait::Retry => thread::sleep(delay),
                    })
                },
                move |batch| future::ready(on_batch(batch)),
            ))
        })
}

/**
Wait for a channel running on a regular OS thread to process all items active at the point this call was made.

This method returns `true` if the flush succeeded, or `false` if it failed or timed out.
This method will always immediately return `false` on the `wasm32-unknown-unknown` target.
*/
pub fn blocking_flush<T: Channel>(sender: &Sender<T>, timeout: Duration) -> bool {
    #![allow(unreachable_code)]

    #[cfg(all(
        target_arch = "wasm32",
        target_vendor = "unknown",
        target_os = "unknown"
    ))]
    {
        let _ = (sender, timeout);

        emit::warn!(rt: emit::runtime::internal(), "blocking flush is not supported on this platform");

        return false;
    }

    let notifier = Trigger::new();

    sender.when_flushed_inner({
        let notifier = notifier.clone();

        move |flushed| {
            notifier.trigger(flushed);
        }
    });

    notifier.wait_timeout(timeout)
}

/**
Wait for a channel to send a message, blocking if the channel is at capacity.

This method will return an error on the `wasm32-unknown-unknown` target.
*/
pub fn blocking_send<T: Channel>(
    sender: &Sender<T>,
    msg: T::Item,
    timeout: Duration,
) -> Result<(), BatchError<T::Item>> {
    #![allow(unreachable_code)]

    #[cfg(all(
        target_arch = "wasm32",
        target_vendor = "unknown",
        target_os = "unknown"
    ))]
    {
        let _ = (sender, msg, timeout);

        return Err(BatchError::no_retry(io::Error::new(
            io::ErrorKind::Other,
            "blocking send is not supported on this platform",
        )));
    }

    let start = Instant::now();

    block_on(sender.send_or_wait(
        msg,
        timeout,
        || start.elapsed(),
        |sender, timeout| {
            let notifier = Trigger::new();

            sender.when_empty({
                let notifier = notifier.clone();

                move || {
                    let _ = notifier.trigger(true);
                }
            });

            notifier.wait_timeout(timeout);

            future::ready(())
        },
    ))
}

#[derive(Clone)]
pub(crate) struct Trigger(Arc<(Mutex<Option<bool>>, Condvar)>);

impl Trigger {
    pub fn new() -> Self {
        Trigger(Arc::new((Mutex::new(None), Condvar::new())))
    }

    pub fn trigger(&self, value: bool) {
        *(self.0).0.lock().unwrap() = Some(value);
        (self.0).1.notify_all();
    }

    pub fn wait_timeout(&self, mut timeout: Duration) -> bool {
        let mut triggered_slot = (self.0).0.lock().unwrap();
        loop {
            // If we were triggered then return the value we were triggered with
            // This may already be set before we start waiting
            if let Some(triggered) = triggered_slot.take() {
                return triggered;
            }

            // If the timeout is 0 then return
            // There's no point waiting for the condition
            if timeout == Duration::ZERO {
                return false;
            }

            let now = Instant::now();
            match (self.0).1.wait_timeout(triggered_slot, timeout).unwrap() {
                (triggered, r) if !r.timed_out() => {
                    triggered_slot = triggered;

                    // Reduce the remaining timeout just in case we didn't time out,
                    // but woke up spuriously for some reason
                    timeout = match timeout.checked_sub(now.elapsed()) {
                        Some(timeout) => timeout,
                        // We didn't time out, but got close enough that we should now anyways
                        None => {
                            return triggered_slot.take().unwrap_or(false);
                        }
                    };

                    continue;
                }
                // Timed out
                (mut triggered_slot, _) => {
                    return triggered_slot.take().unwrap_or(false);
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

    use std::{sync::mpsc, thread};

    use crate::Receiver;

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

        let handle = thread::spawn(move || {
            loop {
                match rx.recv().unwrap() {
                    SenderCommand::BlockingSend(msg, timeout) => {
                        let _ = blocking_send(&sender, msg, timeout);
                    }
                    SenderCommand::Stop => return,
                }
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
    fn try_send_on_closed_channel() {
        let (sender, receiver) = crate::bounded::<Vec<i32>>(10);

        // Drop the receiver to close the channel
        drop(receiver);

        // try_send should fail with a non-retryable error
        let result = sender.try_send(1);
        assert!(result.is_err());

        // Verify the error is non-retryable (no messages to retry)
        let err = result.err().unwrap();
        assert!(err.into_retryable().is_none());
    }

    #[test]
    fn flush_reports_failed_batch() {
        let (sender, receiver) = crate::bounded::<Vec<i32>>(10);

        let handle = spawn("test_receiver", receiver, |_| {
            Err(BatchError::no_retry(io::Error::new(
                io::ErrorKind::Other,
                "explicit failure",
            )))
        })
        .unwrap();

        sender.send(1);

        // The batch is dropped after failing, so the flush must not report success
        assert!(!blocking_flush(&sender, Duration::from_secs(5)));

        drop(sender);
        handle.join().unwrap();
    }

    #[test]
    fn flush_reports_closed_channel() {
        let (sender, receiver) = crate::bounded::<Vec<i32>>(10);

        sender.send(1);

        // Drop the receiver without processing anything; the pending item
        // will never be flushed
        drop(receiver);

        assert!(!blocking_flush(&sender, Duration::from_secs(5)));
    }

    #[test]
    fn flush_reports_target_batch_outcome() {
        let (started_tx, started_rx) = mpsc::channel();
        let (continue_tx, continue_rx) = mpsc::channel();

        let (sender, receiver) = crate::bounded::<Vec<i32>>(10);

        let handle = spawn("test_receiver", receiver, move |_| {
            started_tx.send(()).unwrap();

            // Hold the batch in flight until the test says to continue
            continue_rx.recv().unwrap();

            Err(BatchError::no_retry(io::Error::new(
                io::ErrorKind::Other,
                "explicit failure",
            )))
        })
        .unwrap();

        sender.send(1);

        // Wait until the batch is being processed
        started_rx.recv().unwrap();

        // Ask to be notified while the batch is in flight; the flush outcome
        // must cover that batch even though nothing else is pending
        let (flushed_tx, flushed_rx) = mpsc::channel();
        sender.when_flushed_inner(move |flushed| {
            flushed_tx.send(flushed).unwrap();
        });

        // Let the batch fail
        continue_tx.send(()).unwrap();

        assert!(!flushed_rx.recv().unwrap());

        drop(sender);
        handle.join().unwrap();
    }

    #[test]
    fn flush_wakes_idle_receiver() {
        let received = Arc::new(Mutex::new(0));

        let (sender, receiver) = crate::bounded(10);

        let handle = spawn("test_receiver", receiver, {
            let received = received.clone();

            move |batch: Vec<()>| {
                *received.lock().unwrap() += batch.len();

                Ok(())
            }
        })
        .unwrap();

        // Let the receiver's idle backoff grow towards its maximum (500ms);
        // by 550ms in it's asleep inside a ~500ms delay
        thread::sleep(Duration::from_millis(550));

        sender.send(());

        // Without a wake the flush would have to wait out the remainder of the
        // receiver's idle delay, which is longer than this timeout
        assert!(blocking_flush(&sender, Duration::from_millis(200)));
        assert_eq!(1, *received.lock().unwrap());

        drop(sender);
        handle.join().unwrap();
    }

    #[test]
    fn drop_wakes_idle_receiver() {
        let (sender, receiver) = crate::bounded::<Vec<()>>(10);

        let handle = spawn("test_receiver", receiver, |_| Ok(())).unwrap();

        // Let the receiver's idle backoff grow towards its maximum (500ms)
        thread::sleep(Duration::from_millis(550));

        let dropped = Instant::now();
        drop(sender);

        // Without a wake the receiver would sleep out the remainder of its
        // idle delay before noticing the channel is closed
        handle.join().unwrap();
        assert!(dropped.elapsed() < Duration::from_millis(250));
    }

    #[test]
    fn channel_pressure_wakes_idle_receiver() {
        let received = Arc::new(Mutex::new(0));

        let (sender, receiver) = crate::bounded(10);

        let handle = spawn("test_receiver", receiver, {
            let received = received.clone();

            move |batch: Vec<()>| {
                *received.lock().unwrap() += batch.len();

                Ok(())
            }
        })
        .unwrap();

        // Let the receiver's idle backoff grow towards its maximum (500ms)
        thread::sleep(Duration::from_millis(550));

        // Crossing half the channel's capacity wakes the receiver
        for _ in 0..5 {
            sender.send(());
        }

        // Without a wake the receiver would sleep for longer than this deadline
        let deadline = Instant::now() + Duration::from_millis(200);
        while { *received.lock().unwrap() } < 5 {
            assert!(
                Instant::now() < deadline,
                "receiver didn't wake on pressure"
            );

            thread::sleep(Duration::from_millis(1));
        }

        drop(sender);
        handle.join().unwrap();
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
