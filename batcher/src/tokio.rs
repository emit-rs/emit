/*!
Run channels in a `tokio` runtime.
*/

use std::{future::Future, time::Duration};

use crate::{sync, BatchError, Channel, Receiver, Sender};

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
                    .block_on(receive);
            });
        }
    }
}

/**
Wait for a channel potentially running on a `tokio` thread to process all items active at the point this call was made.

If the current thread is a `tokio` thread then this call will be executed using [`tokio::task::block_in_place`] to avoid starving other work.
*/
pub fn blocking_flush<T: Channel>(sender: &Sender<T>, timeout: Duration) -> bool {
    match tokio::runtime::Handle::try_current() {
        // If we're on a `tokio` thread then await
        Ok(handle) => handle.block_on(flush(sender, timeout)),
        // If we're not on a `tokio` thread then run a regular blocking variant
        Err(_) => sync::blocking_flush(sender, timeout),
    }
}

/**
Wait for a channel potentially running on a `tokio` thread to process all items active at the point this call was made.

This function is an asynchronous variant of [`blocking_send`].
*/
pub async fn flush<T: Channel>(sender: &Sender<T>, timeout: Duration) -> bool {
    let (notifier, notified) = tokio::sync::oneshot::channel();

    sender.when_flushed(move || {
        let _ = notifier.send(());
    });

    wait(notified, timeout).await
}

/**
Wait for a channel to send a message, blocking if the channel is at capacity.
*/
pub fn blocking_send<T: Channel>(
    sender: &Sender<T>,
    msg: T::Item,
    timeout: Duration,
) -> Result<(), BatchError<T::Item>> {
    match tokio::runtime::Handle::try_current() {
        // If we're on a `tokio` thread then await
        Ok(handle) => handle.block_on(send(sender, msg, timeout)),
        // If we're not on a `tokio` thread then run a regular blocking variant
        Err(_) => sync::blocking_send(sender, msg, timeout),
    }
}

/**
Wait for a channel to send a message, blocking if the channel is at capacity.

This function is an asynchronous variant of [`blocking_send`].
*/
pub async fn send<T: Channel>(
    sender: &Sender<T>,
    msg: T::Item,
    timeout: Duration,
) -> Result<(), BatchError<T::Item>> {
    sender
        .send_or_wait(msg, timeout, |sender, timeout| async move {
            let (notifier, notified) = tokio::sync::oneshot::channel();

            sender.when_empty(move || {
                let _ = notifier.send(());
            });

            wait(notified, timeout).await;
        })
        .await
}

async fn wait(mut notified: tokio::sync::oneshot::Receiver<()>, timeout: Duration) -> bool {
    // If the trigger has already fired then return immediately
    if notified.try_recv().is_ok() {
        return true;
    }

    // If the timeout is 0 then return immediately
    // The trigger hasn't already fired so there's no point waiting for it
    if timeout == Duration::ZERO {
        return false;
    }

    match tokio::time::timeout(timeout, notified).await {
        // The notifier was triggered
        Ok(Ok(())) => true,
        // Unexpected hangup; this should mean the channel was closed
        Ok(Err(_)) => true,
        // The timeout was reached instead
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::{Arc, Mutex};

    #[tokio::test]
    async fn async_send_recv_flush() {
        let received = Arc::new(Mutex::new(0));

        let (sender, receiver) = crate::bounded::<Vec<()>>(10);

        spawn(receiver, {
            let received = received.clone();

            move |batch| {
                let received = received.clone();

                async move {
                    *received.lock().unwrap() += batch.len();

                    Ok(())
                }
            }
        });

        for _ in 0..100 {
            send(&sender, (), Duration::from_secs(1))
                .await
                .map_err(|_| "failed to send")
                .unwrap();
        }

        flush(&sender, Duration::from_secs(1)).await;

        assert_eq!(100, *received.lock().unwrap());
    }
}
