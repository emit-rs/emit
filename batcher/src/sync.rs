/*!
Run channels on regular OS threads.
*/

use std::{
    future::{self, Future},
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
    receiver: Receiver<T>,
    mut on_batch: impl FnMut(T) -> Result<(), BatchError<T>> + Send + 'static,
) -> thread::JoinHandle<()>
where
    T::Item: Send + 'static,
{
    static WAKER: OnceLock<Arc<NeverWake>> = OnceLock::new();

    // A waker that does nothing; the tasks it runs are fully
    // synchronous so there's never any notifications to issue
    struct NeverWake;

    impl task::Wake for NeverWake {
        fn wake(self: Arc<Self>) {}
    }

    thread::spawn(move || {
        // The future is polled to completion here, so we can pin
        // it directly on the stack
        let mut fut = pin!(receiver.exec(
            |delay| future::ready(thread::sleep(delay)),
            move |batch| future::ready(on_batch(batch)),
        ));

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
    crate::blocking_send(sender, timeout, msg, |timeout| {
        let notifier = Trigger::new();

        sender.when_empty({
            let notifier = notifier.clone();

            move || {
                let _ = notifier.trigger();
            }
        });

        notifier.wait_timeout(timeout);
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    use crate::Receiver;

    use std::{sync::mpsc, thread};

    enum SenderCommand<T> {
        Send(T),
        BlockingSend(T, Duration),
        BlockingFlush(Duration),
        Stop,
    }

    enum ReciverCommand<T> {
        ProcessBatch(Box<dyn FnOnce(Vec<T>) -> Result<(), BatchError<Vec<T>>> + Send>),
    }

    fn spawn_sender<T: Send + 'static>(
        sender: Sender<Vec<T>>,
    ) -> (mpsc::Sender<SenderCommand<T>>, thread::JoinHandle<()>) {
        let (tx, rx) = mpsc::channel();

        let handle = thread::spawn(move || loop {
            match rx.recv().unwrap() {
                SenderCommand::Send(msg) => {
                    sender.send(msg);
                }
                SenderCommand::BlockingSend(msg, timeout) => {
                    blocking_send(&sender, msg, timeout);
                }
                SenderCommand::BlockingFlush(timeout) => {
                    todo!()
                }
                SenderCommand::Stop => return,
            }
        });

        (tx, handle)
    }

    fn spawn_receiver<T: Send + 'static>(
        receiver: Receiver<Vec<T>>,
    ) -> mpsc::Sender<ReciverCommand<T>> {
        let (tx, rx) = mpsc::channel();

        spawn(receiver, move |batch| match rx.recv().unwrap() {
            ReciverCommand::ProcessBatch(p) => p(batch),
        });

        tx
    }

    #[test]
    fn send_recv() {
        todo!()
    }

    #[test]
    fn recv_panic() {
        todo!()
    }

    #[test]
    fn send_full_capacity() {
        todo!()
    }

    #[test]
    fn blocking_send_recv() {
        todo!()
    }

    #[test]
    fn blocking_send_full_capacity() {
        todo!()
    }

    #[test]
    fn blocking_send_full_capacity_timeout() {
        todo!()
    }

    #[test]
    fn flush_empty() {
        todo!()
    }

    #[test]
    fn flush_active_batch_non_empty_next() {
        // Flush while a batch is active; should wait for the next batch too
        todo!()
    }

    #[test]
    fn flush_active_batch_empty_next() {
        // Flush while a batch is active; should not wait for the next batch too
        todo!()
    }

    #[test]
    fn flush_timeout() {
        todo!()
    }
}
