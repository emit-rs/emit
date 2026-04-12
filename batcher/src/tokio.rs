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
    use tokio::sync::{Barrier, Semaphore};

    use crate::TestBarriers;

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

    #[tokio::test]
    async fn send_full_capacity() {
        let received = Arc::new(Mutex::new(Vec::new()));
        let post_process_barrier = Arc::new(Barrier::new(2));

        let (sender, receiver) = crate::bounded::<Vec<i32>>(5);

        // Send more messages than capacity
        for i in 0..10 {
            sender.send(i);
        }

        // Spawn receiver after sending, with barrier after processing
        let _ = spawn(
            "test_receiver",
            receiver.with_test_barriers(TestBarriers {
                post_process: Some(post_process_barrier.clone()),
                ..Default::default()
            }),
            {
                let received = received.clone();
                move |batch| {
                    let received = received.clone();
                    async move {
                        received.lock().unwrap().extend(batch);
                        Ok(())
                    }
                }
            },
        )
        .unwrap();

        // Wait at barrier for receiver to finish processing
        post_process_barrier.wait().await;

        // Only last 5 messages should remain (0-4 were truncated)
        assert_eq!(vec![5, 6, 7, 8, 9], *received.lock().unwrap());
    }

    #[tokio::test]
    async fn async_send_full_capacity() {
        let received = Arc::new(Mutex::new(0));

        let (sender, receiver) = crate::bounded::<Vec<()>>(5);

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

        // Send more messages than capacity using async send
        for _ in 0..10 {
            send(&sender, (), Duration::from_secs(1)).await.unwrap();
        }

        // Use flush to wait for all messages to be processed
        flush(&sender, Duration::from_secs(1)).await;

        // All 10 messages should be processed
        assert_eq!(10, *received.lock().unwrap());
    }

    #[tokio::test]
    async fn async_send_timeout() {
        // Channel to signal when receiver has taken a batch
        let (receiver_ready_tx, mut receiver_ready_rx) = tokio::sync::broadcast::channel::<()>(100);
        // Semaphore with 0 permits - blocks forever (until cancelled)
        let blocker = Arc::new(Semaphore::new(0));

        let (sender, receiver) = crate::bounded::<Vec<i32>>(5);

        // Spawn receiver task that blocks after taking first batch
        let receiver_task = tokio::task::spawn(async move {
            exec(receiver, {
                let receiver_ready_tx = receiver_ready_tx.clone();
                let blocker = blocker.clone();
                move |_batch| {
                    let receiver_ready_tx = receiver_ready_tx.clone();
                    let blocker = blocker.clone();
                    async move {
                        // Signal that we've received a batch
                        let _ = receiver_ready_tx.send(());
                        // Block forever - acquire will never complete (until task is cancelled)
                        let _ = blocker.acquire().await;
                        Ok(())
                    }
                }
            })
            .await
        });

        // Fill the channel initially
        for i in 0..5 {
            sender.send(i);
        }

        // Wait for receiver to pick up the batch and signal
        receiver_ready_rx.recv().await.ok();

        // Now the channel is empty (receiver has it), fill it again
        for i in 0..5 {
            sender.send(i);
        }

        // Try to send with short timeout - should fail because channel is full
        let result = send(&sender, 99, Duration::from_millis(10)).await;
        assert!(result.is_err());

        // Clean up - abort the receiver task
        receiver_task.abort();
        let _ = receiver_task.await;
    }

    #[tokio::test]
    async fn flush_empty() {
        let (sender, receiver) = crate::bounded::<Vec<()>>(10);

        let _ = spawn("test_receiver", receiver, |batch| async move {
            let _ = batch;
            Ok(())
        })
        .unwrap();

        // Flush with zero timeout on empty channel should succeed immediately
        assert!(flush(&sender, Duration::ZERO).await);
    }

    #[tokio::test]
    async fn flush_active() {
        let batch_count = Arc::new(Mutex::new(0));
        // Channel to signal when receiver has taken first batch
        let (receiver_ready_tx, mut receiver_ready_rx) = tokio::sync::broadcast::channel::<()>(100);

        let (sender, receiver) = crate::bounded::<Vec<i32>>(10);

        let _ = spawn("test_receiver", receiver, {
            let batch_count = batch_count.clone();
            let receiver_ready_tx = receiver_ready_tx.clone();
            move |_batch| {
                let batch_count = batch_count.clone();
                let receiver_ready_tx = receiver_ready_tx.clone();
                async move {
                    *batch_count.lock().unwrap() += 1;
                    // Signal that we've taken a batch
                    let _ = receiver_ready_tx.send(());
                    Ok(())
                }
            }
        })
        .unwrap();

        // Send initial batch
        for i in 0..3 {
            sender.send(i);
        }

        // Wait for receiver to pick up the batch
        receiver_ready_rx.recv().await.ok();

        // Send more messages (second batch)
        for i in 3..6 {
            sender.send(i);
        }

        // Flush should wait for both batches to complete
        let flushed = flush(&sender, Duration::from_secs(1)).await;
        assert!(flushed);

        // Both batches should have been processed
        assert_eq!(2, *batch_count.lock().unwrap());
    }

    #[tokio::test]
    async fn retry_on_batch_failure() {
        let attempt_count = Arc::new(Mutex::new(0));
        let received = Arc::new(Mutex::new(false));

        let (sender, receiver) = crate::bounded::<Vec<i32>>(10);

        let _ = spawn("test_receiver", receiver, {
            let attempt_count = attempt_count.clone();
            let received = received.clone();
            move |batch| {
                let attempt_count = attempt_count.clone();
                let received = received.clone();
                async move {
                    let mut count = attempt_count.lock().unwrap();
                    *count += 1;

                    // Fail first two attempts, succeed on third
                    if *count < 3 {
                        Err(BatchError::retry(
                            std::io::Error::new(std::io::ErrorKind::Other, "temporary failure"),
                            batch,
                        ))
                    } else {
                        *received.lock().unwrap() = true;
                        Ok(())
                    }
                }
            }
        })
        .unwrap();

        sender.send(42);

        // Retry loop: return early when count == 2, panic after 3 seconds
        let start = Instant::now();
        loop {
            let received = *received.lock().unwrap();
            if received {
                break;
            }

            if start.elapsed() >= Duration::from_secs(3) {
                panic!("Timeout waiting for retries");
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    #[tokio::test]
    async fn processes_remaining_after_drop() {
        let received = Arc::new(Mutex::new(Vec::new()));
        let post_process_barrier = Arc::new(Barrier::new(2));

        let (sender, receiver) = crate::bounded::<Vec<i32>>(10);

        let _ = spawn(
            "test_receiver",
            receiver.with_test_barriers(TestBarriers {
                post_process: Some(post_process_barrier.clone()),
                ..Default::default()
            }),
            {
                let received = received.clone();
                move |batch| {
                    let received = received.clone();
                    async move {
                        received.lock().unwrap().extend(batch);
                        Ok(())
                    }
                }
            },
        )
        .unwrap();

        // Send messages and drop sender
        for i in 0..5 {
            sender.send(i);
        }
        drop(sender);

        // Wait at barrier for receiver to finish processing
        post_process_barrier.wait().await;

        // All messages should still be processed
        assert_eq!(vec![0, 1, 2, 3, 4], *received.lock().unwrap());
    }

    #[tokio::test]
    async fn try_send_behavior() {
        let post_process_barrier = Arc::new(Barrier::new(2));

        let (sender, receiver) = crate::bounded::<Vec<i32>>(3);
        let _ = spawn(
            "test_receiver",
            receiver.with_test_barriers(TestBarriers {
                post_process: Some(post_process_barrier.clone()),
                ..Default::default()
            }),
            |batch| async move {
                let _ = batch;
                Ok(())
            },
        )
        .unwrap();

        // Successful sends
        assert!(sender.try_send(1).is_ok());
        assert!(sender.try_send(2).is_ok());
        assert!(sender.try_send(3).is_ok());

        // Should fail when at capacity
        let result = sender.try_send(4);
        assert!(result.is_err());

        // Wait at barrier for receiver to finish processing
        post_process_barrier.wait().await;

        // Should succeed after processing
        assert!(sender.try_send(4).is_ok());
    }

    #[tokio::test]
    async fn try_send_on_closed_channel() {
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

    #[tokio::test]
    async fn when_empty_callback() {
        let callback_fired = Arc::new(Mutex::new(false));
        let post_take_barrier = Arc::new(Barrier::new(2));

        let (sender, receiver) = crate::bounded::<Vec<i32>>(10);

        let _ = spawn(
            "test_receiver",
            receiver.with_test_barriers(TestBarriers {
                post_take: Some(post_take_barrier.clone()),
                ..Default::default()
            }),
            |_batch| async move { Ok(()) },
        )
        .unwrap();

        // Send a message
        sender.send(1);

        let callback_fired_clone = callback_fired.clone();
        sender.when_empty(move || {
            *callback_fired_clone.lock().unwrap() = true;
        });

        // Callback shouldn't fire yet (batch not taken)
        assert!(!*callback_fired.lock().unwrap());

        // Wait at barrier for batch to be taken
        post_take_barrier.wait().await;

        // Callback should have fired
        assert!(*callback_fired.lock().unwrap());
    }

    #[tokio::test]
    async fn when_flushed_callback() {
        let callback_fired = Arc::new(Mutex::new(false));
        let post_process_barrier = Arc::new(Barrier::new(2));

        let (sender, receiver) = crate::bounded::<Vec<i32>>(10);

        let _ = spawn(
            "test_receiver",
            receiver.with_test_barriers(TestBarriers {
                post_process: Some(post_process_barrier.clone()),
                ..Default::default()
            }),
            |_batch| async move { Ok(()) },
        )
        .unwrap();

        // Send a message
        sender.send(1);

        let callback_fired_clone = callback_fired.clone();
        sender.when_flushed(move || {
            *callback_fired_clone.lock().unwrap() = true;
        });

        // Callback shouldn't fire yet (batch not processed)
        assert!(!*callback_fired.lock().unwrap());

        // Wait at barrier for batch to be processed
        post_process_barrier.wait().await;

        // Callback should have fired
        assert!(*callback_fired.lock().unwrap());
    }
}
