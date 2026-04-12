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
    /// **Property**: Async send and flush work correctly for high-volume message processing.
    ///
    /// **Sequence of events**:
    /// 1. Create a bounded channel with capacity 10
    /// 2. Spawn receiver task that counts processed messages
    /// 3. Send 100 messages using async send (blocks when at capacity)
    /// 4. Call flush to wait for all messages to be processed
    /// 5. Verify all 100 messages were received and counted
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
    /// **Property**: Channel truncates oldest messages when capacity is exceeded (tokio variant).
    ///
    /// **Sequence of events**:
    /// 1. Create a bounded channel with capacity 5
    /// 2. Send 10 messages (exceeding capacity, causing truncation of 0-4)
    /// 3. Spawn receiver with post_process barrier for synchronization
    /// 4. Receiver processes the remaining batch
    /// 5. Wait at barrier for processing to complete
    /// 6. Verify only messages 5-9 were received (first 5 truncated)
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
            receiver.with_test_barriers(
                TestBarriers::new().with_post_process(post_process_barrier.clone()),
            ),
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
    /// **Property**: Async send with blocking ensures all messages are processed even when exceeding capacity.
    ///
    /// **Sequence of events**:
    /// 1. Create a bounded channel with capacity 5
    /// 2. Spawn receiver task
    /// 3. Send 10 messages using async send (blocks when full, waits for capacity)
    /// 4. Call flush to wait for all processing to complete
    /// 5. Verify all 10 messages were processed (no truncation because async_send waits)
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
    /// **Property**: Async send times out when channel remains at capacity.
    ///
    /// **Sequence of events**:
    /// 1. Create a bounded channel with capacity 5
    /// 2. Spawn receiver that takes first batch then blocks indefinitely (using Semaphore(0))
    /// 3. Fill channel with 5 messages
    /// 4. Wait for receiver to take the batch and block
    /// 5. Fill channel again with 5 messages (now at capacity)
    /// 6. Attempt async send with 10ms timeout - should fail (channel full, receiver blocked)
    /// 7. Abort receiver task to clean up
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
    /// **Property**: Flush on an empty channel with zero timeout succeeds immediately (tokio variant).
    ///
    /// **Sequence of events**:
    /// 1. Create a bounded channel with capacity 10
    /// 2. Spawn receiver task
    /// 3. Call flush with zero timeout on empty channel
    /// 4. Flush returns true immediately (nothing to wait for)
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
    /// **Property**: Flush waits for all active and in-flight batches to complete (tokio variant).
    ///
    /// **Sequence of events**:
    /// 1. Create a bounded channel with capacity 10
    /// 2. Spawn receiver that signals when it takes a batch
    /// 3. Send 3 messages to start first batch
    /// 4. Wait for receiver to pick up first batch
    /// 5. Send 3 more messages (second batch, queued while first is processing)
    /// 6. Call flush - should wait for both batches to complete
    /// 7. Verify flush succeeded and at least one batch was processed
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
        assert!(*batch_count.lock().unwrap() >= 1);
    }

    #[tokio::test]
    /// **Property**: Batch processing retries on temporary failures until success or max retries.
    ///
    /// **Sequence of events**:
    /// 1. Create a bounded channel with capacity 10
    /// 2. Spawn receiver that fails twice then succeeds on third attempt
    /// 3. Send a single message
    /// 4. Wait for retry logic to complete (3 attempts with 700ms backoff)
    /// 5. Verify at least 2 attempts were made and message was eventually received
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

        // Wait for retries to complete (700ms delay between retries, need 3 attempts)
        tokio::time::sleep(Duration::from_secs(3)).await;

        // Should have 3 attempts: 1 initial + 2 retries
        let count = *attempt_count.lock().unwrap();
        assert!(count >= 2, "Should have at least 2 attempts, got {}", count);
        assert!(*received.lock().unwrap());
    }

    #[tokio::test]
    /// **Property**: Receiver processes all remaining messages after sender is dropped.
    ///
    /// **Sequence of events**:
    /// 1. Create a bounded channel with capacity 10
    /// 2. Spawn receiver with post_process barrier for synchronization
    /// 3. Send 5 messages to the channel
    /// 4. Drop the sender (signals channel is closed)
    /// 5. Wait at barrier for receiver to finish processing
    /// 6. Verify all 5 messages were processed despite sender being dropped
    async fn processes_remaining_after_drop() {
        let received = Arc::new(Mutex::new(Vec::new()));
        let post_process_barrier = Arc::new(Barrier::new(2));

        let (sender, receiver) = crate::bounded::<Vec<i32>>(10);

        let _ = spawn(
            "test_receiver",
            receiver.with_test_barriers(
                TestBarriers::new().with_post_process(post_process_barrier.clone()),
            ),
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
    /// **Property**: try_send succeeds when under capacity and fails immediately when at capacity.
    ///
    /// **Sequence of events**:
    /// 1. Create a bounded channel with capacity 3
    /// 2. Spawn receiver with post_process barrier for synchronization
    /// 3. Send 3 messages using try_send (should all succeed)
    /// 4. Attempt 4th try_send - should fail immediately (at capacity)
    /// 5. Wait at barrier for receiver to process the batch
    /// 6. Attempt try_send again - should succeed (capacity freed)
    async fn try_send_behavior() {
        let post_process_barrier = Arc::new(Barrier::new(2));

        let (sender, receiver) = crate::bounded::<Vec<i32>>(3);
        let _ = spawn(
            "test_receiver",
            receiver.with_test_barriers(
                TestBarriers::new().with_post_process(post_process_barrier.clone()),
            ),
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
    /// **Property**: try_send returns an error when the channel is closed.
    ///
    /// **Sequence of events**:
    /// 1. Create a bounded channel
    /// 2. Drop the receiver to close the channel from the receiver side
    /// 3. Attempt try_send - should fail with a non-retryable error
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
    /// **Property**: when_empty callback fires when the channel becomes empty (batch is taken).
    ///
    /// **Sequence of events**:
    /// 1. Create a bounded channel with capacity 10
    /// 2. Spawn receiver with post_take barrier for synchronization
    /// 3. Send a single message
    /// 4. Register when_empty callback
    /// 5. Verify callback hasn't fired yet (batch not taken)
    /// 6. Wait at barrier for receiver to take the batch
    /// 7. Verify callback fired after batch was taken (channel empty)
    async fn when_empty_callback() {
        let callback_fired = Arc::new(Mutex::new(false));
        let post_take_barrier = Arc::new(Barrier::new(2));

        let (sender, receiver) = crate::bounded::<Vec<i32>>(10);

        let _ = spawn(
            "test_receiver",
            receiver
                .with_test_barriers(TestBarriers::new().with_post_take(post_take_barrier.clone())),
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
    /// **Property**: when_flushed callback fires when a batch is fully processed.
    ///
    /// **Sequence of events**:
    /// 1. Create a bounded channel with capacity 10
    /// 2. Spawn receiver with post_process barrier for synchronization
    /// 3. Send a single message
    /// 4. Register when_flushed callback
    /// 5. Verify callback hasn't fired yet (batch not processed)
    /// 6. Wait at barrier for receiver to finish processing
    /// 7. Verify callback fired after batch was flushed
    async fn when_flushed_callback() {
        let callback_fired = Arc::new(Mutex::new(false));
        let post_process_barrier = Arc::new(Barrier::new(2));

        let (sender, receiver) = crate::bounded::<Vec<i32>>(10);

        let _ = spawn(
            "test_receiver",
            receiver.with_test_barriers(
                TestBarriers::new().with_post_process(post_process_barrier.clone()),
            ),
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
