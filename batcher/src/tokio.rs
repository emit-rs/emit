/*!
Run channels in a `tokio` runtime.
*/

use std::{
    future::Future,
    io, thread,
    time::{Duration, Instant},
};

use crate::{sync, BatchError, Channel, Receiver, Sender};

/**
Run [`Receiver::exec`] on a `tokio` runtime in a dedicated background thread.

This function will create a single-threaded `tokio` runtime on a dedicated thread.
*/
pub fn spawn<
    T: Channel + Send + 'static,
    F: Future<Output = Result<(), BatchError<T>>> + Send + 'static,
>(
    thread_name: impl Into<String>,
    receiver: Receiver<T>,
    on_batch: impl FnMut(T) -> F + Send + 'static,
) -> io::Result<thread::JoinHandle<()>>
where
    T::Item: Send + 'static,
{
    let receive = async move {
        receiver
            .exec(|delay| tokio::time::sleep(delay), on_batch)
            .await
    };

    thread::Builder::new()
        .name(thread_name.into())
        .spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(receive);
        })
}

/**
Run [`Receiver::exec`] on the current `tokio` runtime.
*/
pub async fn exec<T: Channel, F: Future<Output = Result<(), BatchError<T>>>>(
    receiver: Receiver<T>,
    on_batch: impl FnMut(T) -> F,
) {
    receiver
        .exec(|delay| tokio::time::sleep(delay), on_batch)
        .await
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
    let start = Instant::now();

    sender
        .send_or_wait(
            msg,
            timeout,
            || start.elapsed(),
            |sender, timeout| async move {
                let (notifier, notified) = tokio::sync::oneshot::channel();

                sender.when_empty(move || {
                    let _ = notifier.send(());
                });

                wait(notified, timeout).await;
            },
        )
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

        let _ = spawn("test_receiver", receiver, {
            let received = received.clone();

            move |batch| {
                let received = received.clone();

                async move {
                    *received.lock().unwrap() += batch.len();

                    Ok(())
                }
            }
        })
        .unwrap();

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
