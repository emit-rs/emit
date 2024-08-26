/*!
Run channels in a `tokio` runtime.
*/

use std::{
    cmp,
    future::Future,
    time::{Duration, Instant},
};

use crate::{BatchError, Channel, Receiver, Sender};

/**
Spawn a worker to run the [`Receiver`] on a `tokio` runtime.

If the current thread is a `tokio` thread, then the worker will be spawned onto its runtime. If the current thread is not a `tokio` thread, then a single-threaded `tokio` runtime will be set up in a dedicated thread to run it.
*/
pub fn spawn<
    T: Channel + Send + 'static,
    F: Future<Output = Result<(), BatchError<T>>> + Send + 'static,
>(
    receiver: Receiver<T>,
    on_batch: impl FnMut(T) -> F + Send + 'static,
) where
    T::Item: Send + 'static,
{
    let receive = async move {
        receiver
            .exec(|delay| tokio::time::sleep(delay), on_batch)
            .await
    };

    match tokio::runtime::Handle::try_current() {
        // If we're on a `tokio` thread then spawn on it
        Ok(handle) => {
            handle.spawn(receive);
        }
        // If we're not on a `tokio` thread then spawn a
        // background thread and run the work there
        Err(_) => {
            std::thread::spawn(move || {
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap()
                    .block_on(receive)
                    .unwrap();
            });
        }
    }
}

/**
Wait for a channel potentially running on a `tokio` thread to process all items active at the point this call was made.

If the current thread is a `tokio` thread then this call will be executed using [`tokio::task::block_in_place`] to avoid starving other work.
*/
pub fn blocking_flush<T: Channel>(sender: &Sender<T>, timeout: Duration) -> bool {
    tokio::task::block_in_place(|| {
        let (notifier, notified) = tokio::sync::oneshot::channel();

        sender.when_flushed(move || {
            let _ = notifier.send(());
        });

        wait(notified, timeout)
    })
}

/**
Wait for a channel to send a message, blocking if the channel is at capacity.
*/
pub fn blocking_send<T: Channel>(
    sender: &Sender<T>,
    timeout: Duration,
    msg: T::Item,
) -> Result<(), BatchError<T::Item>> {
    crate::blocking_send(sender, timeout, msg, |timeout| {
        tokio::task::block_in_place(|| {
            let (notifier, notified) = tokio::sync::oneshot::channel();

            sender.when_empty(move || {
                let _ = notifier.send(());
            });

            wait(notified, timeout);
        });
    })
}

fn wait(mut notified: tokio::sync::oneshot::Receiver<()>, timeout: Duration) -> bool {
    // If the trigger has already fired then return immediately
    if notified.try_recv().is_ok() {
        return true;
    }

    // If the timeout is 0 then return immediately
    // The trigger hasn't already fired so there's no point waiting for it
    if timeout == Duration::ZERO {
        return false;
    }

    match tokio::runtime::Handle::try_current() {
        // If we're on a `tokio` thread then await the receiver
        Ok(handle) => handle.block_on(async {
            match tokio::time::timeout(timeout, notified).await {
                // The notifier was triggered
                Ok(Ok(())) => true,
                // Unexpected hangup; this should mean the channel was closed
                Ok(Err(_)) => true,
                // The timeout was reached instead
                Err(_) => false,
            }
        }),
        // If we're not on a `tokio` thread then wait for
        // a notification
        Err(_) => {
            let now = Instant::now();
            let mut wait = Duration::from_micros(1);
            let max_wait_step = cmp::max(timeout / 3, Duration::from_micros(1));

            while now.elapsed() < timeout {
                if notified.try_recv().is_ok() {
                    return true;
                }

                // Apply some exponential backoff to avoid spinning
                // Chances are if we're not called immediately that
                // it'll be waiting on some network or file IO and could
                // be a while
                std::thread::sleep(wait);
                wait += cmp::min(wait * 2, max_wait_step);
            }

            false
        }
    }
}
