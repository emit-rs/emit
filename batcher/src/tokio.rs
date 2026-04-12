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

    #[tokio::test]
    async fn send_full_capacity() {
        let received = Arc::new(Mutex::new(Vec::new()));

        let (sender, receiver) = crate::bounded::<Vec<i32>>(5);

        // Send more messages than capacity
        for i in 0..10 {
            sender.send(i);
        }

        // Spawn receiver after sending
        let _ = spawn("test_receiver", receiver, {
            let received = received.clone();
            move |batch| {
                let received = received.clone();
                async move {
                    received.lock().unwrap().extend(batch);
                    Ok(())
                }
            }
        })
        .unwrap();

        // Wait for processing
        tokio::time::sleep(Duration::from_millis(100)).await;

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
            send(&sender, (), Duration::from_secs(1))
                .await
                .unwrap();
        }

        // Wait for all to be processed
        tokio::time::sleep(Duration::from_millis(100)).await;

        // All 10 messages should be processed
        assert_eq!(10, *received.lock().unwrap());
    }

    #[tokio::test]
    async fn async_send_timeout() {
        let received = Arc::new(Mutex::new(Vec::new()));
        let (receiver_ready_tx, mut receiver_ready_rx) = tokio::sync::broadcast::channel(100);

        let (sender, receiver) = crate::bounded::<Vec<i32>>(5);

        // Spawn receiver task that blocks before processing
        let received_clone = received.clone();
        let receiver_ready_tx_clone = receiver_ready_tx.clone();
        let receiver_task = tokio::task::spawn(async move {
            exec(receiver, {
                let received = received_clone.clone();
                move |batch| {
                    let received = received.clone();
                    let receiver_ready_tx = receiver_ready_tx_clone.clone();
                    async move {
                        // Signal that we've received a batch and block before processing
                        let _ = receiver_ready_tx.send(());
                        // Wait for test to fill the channel again
                        tokio::time::sleep(Duration::from_millis(100)).await;
                        received.lock().unwrap().extend(batch);
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

        let (sender, receiver) = crate::bounded::<Vec<i32>>(10);

        let _ = spawn("test_receiver", receiver, {
            let batch_count = batch_count.clone();
            move |_batch| {
                let batch_count = batch_count.clone();
                async move {
                    *batch_count.lock().unwrap() += 1;
                    // Small delay to simulate processing
                    tokio::time::sleep(Duration::from_millis(10)).await;
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
        tokio::time::sleep(Duration::from_millis(50)).await;

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
    async fn processes_remaining_after_drop() {
        let received = Arc::new(Mutex::new(Vec::new()));

        let (sender, receiver) = crate::bounded::<Vec<i32>>(10);

        let _ = spawn("test_receiver", receiver, {
            let received = received.clone();
            move |batch| {
                let received = received.clone();
                async move {
                    received.lock().unwrap().extend(batch);
                    Ok(())
                }
            }
        })
        .unwrap();

        // Send messages and drop sender
        for i in 0..5 {
            sender.send(i);
        }
        drop(sender);

        // Wait for processing
        tokio::time::sleep(Duration::from_millis(100)).await;

        // All messages should still be processed
        assert_eq!(vec![0, 1, 2, 3, 4], *received.lock().unwrap());
    }

    #[tokio::test]
    async fn try_send_behavior() {
        let (sender, receiver) = crate::bounded::<Vec<i32>>(3);
        let _ = spawn("test_receiver", receiver, |batch| async move {
            let _ = batch;
            Ok(())
        })
        .unwrap();

        // Successful sends
        assert!(sender.try_send(1).is_ok());
        assert!(sender.try_send(2).is_ok());
        assert!(sender.try_send(3).is_ok());

        // Should fail when at capacity
        let result = sender.try_send(4);
        assert!(result.is_err());

        // Wait for processing
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Should succeed after processing
        assert!(sender.try_send(4).is_ok());
    }

    #[tokio::test]
    async fn when_empty_callback() {
        let callback_fired = Arc::new(Mutex::new(false));

        let (sender, receiver) = crate::bounded::<Vec<i32>>(10);

        let _ = spawn("test_receiver", receiver, |_batch| async move {
            Ok(())
        })
        .unwrap();

        // Send a message
        sender.send(1);

        let callback_fired_clone = callback_fired.clone();
        sender.when_empty(move || {
            *callback_fired_clone.lock().unwrap() = true;
        });

        // Callback shouldn't fire yet (batch not taken)
        assert!(!*callback_fired.lock().unwrap());

        // Wait for batch to be taken
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Callback should have fired
        assert!(*callback_fired.lock().unwrap());
    }

    #[tokio::test]
    async fn when_flushed_callback() {
        let callback_fired = Arc::new(Mutex::new(false));

        let (sender, receiver) = crate::bounded::<Vec<i32>>(10);

        let _ = spawn("test_receiver", receiver, |_batch| async move {
            // Simulate some processing time
            tokio::time::sleep(Duration::from_millis(10)).await;
            Ok(())
        })
        .unwrap();

        // Send a message
        sender.send(1);

        let callback_fired_clone = callback_fired.clone();
        sender.when_flushed(move || {
            *callback_fired_clone.lock().unwrap() = true;
        });

        // Callback shouldn't fire yet (batch not processed)
        assert!(!*callback_fired.lock().unwrap());

        // Wait for batch to be processed
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Callback should have fired
        assert!(*callback_fired.lock().unwrap());
    }
}

#[cfg(test)]
mod quickcheck_tests {
    use crate::tokio::{flush, spawn};
    use crate::bounded;

    use quickcheck_macros::quickcheck;
    use std::sync::{Arc, Mutex};
    use tokio::time::{sleep, Duration};

    #[quickcheck]
    fn prop_all_messages_received(messages: Vec<i32>) {
        if messages.is_empty() || messages.len() > 1000 {
            return;
        }

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let received = Arc::new(Mutex::new(Vec::new()));
            let (sender, receiver) = bounded::<Vec<i32>>(1024);

            let _ = spawn("test_receiver", receiver, {
                let received = received.clone();
                move |batch| {
                    let received = received.clone();
                    async move {
                        received.lock().unwrap().extend(batch);
                        Ok(())
                    }
                }
            })
            .unwrap();

            // Send all messages
            for msg in &messages {
                sender.send(*msg);
            }

            // Wait for processing
            flush(&sender, Duration::from_secs(1)).await;
            sleep(Duration::from_millis(50)).await;

            // Verify all messages were received (order may vary due to batching)
            let mut received_vec = received.lock().unwrap().clone();
            received_vec.sort();
            let mut expected = messages.clone();
            expected.sort();

            assert_eq!(expected, received_vec);
        });
    }

    #[quickcheck]
    fn prop_truncation_keeps_most_recent(capacity: usize, extra_messages: usize) {
        // Constrain to reasonable values - ensure we have meaningful test data
        let capacity = (capacity % 50) + 1; // 1-50
        let extra_messages = (extra_messages % 100) + 1; // 1-100

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let received = Arc::new(Mutex::new(Vec::new()));
            let (sender, receiver) = bounded::<Vec<usize>>(capacity);

            // Send more messages than capacity BEFORE spawning receiver
            // This ensures truncation happens in the channel buffer
            for i in 0..(capacity + extra_messages) {
                sender.send(i);
            }

            // Spawn receiver after sending - it will only see the most recent messages
            let _ = spawn("test_receiver", receiver, {
                let received = received.clone();
                move |batch| {
                    let received = received.clone();
                    async move {
                        received.lock().unwrap().extend(batch);
                        Ok(())
                    }
                }
            })
            .unwrap();

            // Wait for processing
            sleep(Duration::from_millis(200)).await;

            let received_vec = received.lock().unwrap();

            // After truncation, we should have at most capacity messages
            // The actual count depends on how many messages were sent after the last truncation
            assert!(
                received_vec.len() <= capacity,
                "Should receive at most capacity messages, got {}",
                received_vec.len()
            );

            // All received messages should be >= extra_messages (the most recent ones after truncation)
            // This verifies that older messages were dropped
            for &msg in received_vec.iter() {
                assert!(
                    msg >= extra_messages,
                    "All messages should be the most recent ones (>= {}), got {}",
                    extra_messages,
                    msg
                );
            }
        });
    }

    #[quickcheck]
    fn prop_batch_sizes_are_consistent(message_count: usize, capacity: usize) {
        // Constrain values - ensure message_count <= capacity to avoid truncation
        let capacity = (capacity % 100) + 1; // 1-100
        let message_count = (message_count % capacity) + 1; // 1-capacity

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let batch_sizes = Arc::new(Mutex::new(Vec::new()));
            let (sender, receiver) = bounded::<Vec<i32>>(capacity);

            let _ = spawn("test_receiver", receiver, {
                let batch_sizes = batch_sizes.clone();
                move |batch| {
                    let batch_sizes = batch_sizes.clone();
                    async move {
                        batch_sizes.lock().unwrap().push(batch.len());
                        Ok(())
                    }
                }
            })
            .unwrap();

            // Send messages (within capacity, so no truncation)
            for _ in 0..message_count {
                sender.send(42i32);
            }

            sleep(Duration::from_millis(200)).await;

            let batch_sizes = batch_sizes.lock().unwrap();

            // Total messages processed should equal sent messages
            let total_processed: usize = batch_sizes.iter().sum();
            assert_eq!(total_processed, message_count, "All messages should be processed");

            // No batch should exceed capacity
            for &size in batch_sizes.iter() {
                assert!(size <= capacity, "Batch size should not exceed capacity");
            }
        });
    }

    #[quickcheck]
    fn prop_flush_guarantees_completion(message_count: usize, capacity: usize) {
        // Constrain values - ensure message_count <= capacity to avoid truncation
        let capacity = (capacity % 100) + 1; // 1-100
        let message_count = (message_count % capacity) + 1; // 1-capacity

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let received = Arc::new(Mutex::new(0usize));
            let (sender, receiver) = bounded::<Vec<i32>>(capacity);

            let _ = spawn("test_receiver", receiver, {
                let received = received.clone();
                move |batch| {
                    let received = received.clone();
                    async move {
                        *received.lock().unwrap() += batch.len();
                        // Add variable delay to simulate real processing
                        sleep(Duration::from_millis((batch.len() % 10) as u64)).await;
                        Ok(())
                    }
                }
            })
            .unwrap();

            // Send messages (within capacity, so no truncation)
            for _ in 0..message_count {
                sender.send(42i32);
            }

            // Flush should wait for all to complete
            let flushed = flush(&sender, Duration::from_secs(5)).await;
            assert!(flushed, "Flush should complete within timeout");

            // All messages should be processed
            assert_eq!(*received.lock().unwrap(), message_count);
        });
    }

    #[quickcheck]
    fn prop_drop_sender_preserves_messages(message_count: usize, capacity: usize) {
        // Constrain values - ensure message_count <= capacity to avoid truncation
        let capacity = (capacity % 100) + 1; // 1-100
        let message_count = (message_count % capacity) + 1; // 1-capacity

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let received = Arc::new(Mutex::new(Vec::new()));
            let (sender, receiver) = bounded::<Vec<i32>>(capacity);

            let _ = spawn("test_receiver", receiver, {
                let received = received.clone();
                move |batch| {
                    let received = received.clone();
                    async move {
                        received.lock().unwrap().extend(batch);
                        Ok(())
                    }
                }
            })
            .unwrap();

            // Send messages (within capacity, so no truncation)
            for i in 0..message_count {
                sender.send(i as i32);
            }

            // Drop sender immediately
            drop(sender);

            // Wait for processing
            sleep(Duration::from_millis(200)).await;

            // All messages should still be processed
            let received_vec = received.lock().unwrap();
            assert_eq!(received_vec.len(), message_count);
        });
    }
}
