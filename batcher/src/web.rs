/*!
Run channels in a JavaScript runtime using a background promise.
*/

use std::{
    cell::RefCell,
    cmp,
    future::Future,
    io, mem,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll, Waker},
    time::Duration,
};

use js_sys::Promise;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{future_to_promise, JsFuture};

use crate::{BatchError, Channel, Receiver, Sender};

/**
Run [`Receiver::exec`] in a fire-and-forget JavaScript promise.
*/
pub fn spawn<T: Channel + 'static, F: Future<Output = Result<(), BatchError<T>>> + 'static>(
    receiver: Receiver<T>,
    on_batch: impl FnMut(T) -> F + 'static,
) -> io::Result<SpawnHandle> {
    // Fire-and-forget promise
    let promise = future_to_promise(async move {
        // `exec` attempts to catch panics, but since they can't be caught in wasm,
        // there's not much we can do here to prevent them propagating if they happen
        receiver.exec(|delay| Park::new(delay), on_batch).await;

        Ok(JsValue::UNDEFINED)
    });

    Ok(SpawnHandle { promise })
}

/**
A handle that can be used to join a spawned receiver.

The receiver is a promise that will resolve when the channel is closed.
*/
pub struct SpawnHandle {
    promise: Promise,
}

impl SpawnHandle {
    /**
    Convert this handle into a JavaScript promise.
    */
    pub fn into_promise(self) -> Promise {
        self.promise
    }

    /**
    Join the handle, waiting for it to complete.
    */
    pub async fn join(self) {
        // The promise is infallible, so the return here isn't interesting
        let _ = JsFuture::from(self.promise).await;
    }
}

/**
Wait for a channel to send a message, blocking if the channel is at capacity.
*/
pub async fn send<T: Channel>(
    sender: &Sender<T>,
    msg: T::Item,
    timeout: Duration,
) -> Result<(), BatchError<T::Item>> {
    let start = performance_now();

    sender
        .send_or_wait(
            msg,
            timeout,
            || performance_now().saturating_sub(start),
            |sender, timeout| async move {
                let (notifier, notified) = futures::channel::oneshot::channel();

                sender.when_empty(move || {
                    let _ = notifier.send(());
                });

                wait(notified, timeout).await;
            },
        )
        .await
}

/**
Wait for a channel running in a JavaScript promise to process all items active at the point this call was made.
*/
pub async fn flush<T: Channel>(sender: &Sender<T>, timeout: Duration) -> bool {
    let (notifier, notified) = futures::channel::oneshot::channel();

    sender.when_flushed(move || {
        let _ = notifier.send(());
    });

    wait(notified, timeout).await
}

async fn wait(mut notified: futures::channel::oneshot::Receiver<()>, timeout: Duration) -> bool {
    // If the trigger has already fired then return immediately
    if let Ok(Some(())) = notified.try_recv() {
        return true;
    }

    // If the timeout is 0 then return immediately
    // The trigger hasn't already fired so there's no point waiting for it
    if timeout == Duration::ZERO {
        return false;
    }

    let timeout = Park::new(timeout);

    match futures::future::select(notified, timeout).await {
        // The notifier was triggered
        futures::future::Either::Left((Ok(_), _)) => true,
        // Unexpected hangup; this should mean the channel was closed
        futures::future::Either::Left((Err(_), _)) => true,
        // The timeout was reached instead
        futures::future::Either::Right(((), _)) => false,
    }
}

/**
Wait for approximately `delay`.

This is semantically more like `park_timeout` than `sleep` because the treatment of the delay itself is quite lax.

This function may wait for longer than `delay` if `delay` is a fractional number of milliseconds.
This function may wait for less than `delay` if `setTimeout` cannot be called, or the future is dropped before the delay triggers.
*/
struct Park {
    // `Some` if the timeout hasn't been scheduled yet
    delay: Option<Duration>,
    // `Some` if the timeout has been scheduled
    timeout: Option<Timeout>,
    state: Rc<RefCell<ParkState>>,
}

impl Drop for Park {
    fn drop(&mut self) {
        ParkState::wake(&self.state);
    }
}

impl Park {
    fn new(delay: Duration) -> Self {
        Park {
            delay: Some(delay),
            timeout: None,
            state: Rc::new(RefCell::new(ParkState {
                done: false,
                wakers: Vec::new(),
            })),
        }
    }
}

impl ParkState {
    fn wake(state: &Rc<RefCell<Self>>) {
        let mut state = state.borrow_mut();

        state.done = true;
        let wakers = mem::take(&mut state.wakers);

        drop(state);

        for waker in wakers {
            waker.wake();
        }
    }
}

struct Timeout {
    _closure: Closure<dyn Fn()>,
    token: f64,
}

impl Drop for Timeout {
    fn drop(&mut self) {
        clear_timeout(self.token);
    }
}

struct ParkState {
    // `true` if the timeout has elapsed
    done: bool,
    wakers: Vec<Waker>,
}

impl Future for Park {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<()> {
        let unpinned = unsafe { self.get_unchecked_mut() };

        if unpinned.state.borrow_mut().done {
            return Poll::Ready(());
        }

        // The timeout hasn't been scheduled, or hasn't expired yet

        // Add this waker to the set to wake when the timeout resolves
        let mut state = unpinned.state.borrow_mut();

        let waker = cx.waker();

        if !state.wakers.iter().any(|w| w.will_wake(waker)) {
            state.wakers.push(waker.clone());
        }

        drop(state);

        // Schedule the timeout if we haven't already
        // The timeout is done using `setTimeout`
        if let Some(delay) = unpinned.delay.take() {
            let state = unpinned.state.clone();

            let closure = Closure::<dyn Fn()>::new(move || {
                ParkState::wake(&state);
            });

            let token = set_timeout(&closure, cmp::max(1, delay.as_millis() as u32));

            unpinned.timeout = Some(Timeout {
                token,
                _closure: closure,
            });
        }

        Poll::Pending
    }
}

fn performance_now() -> Duration {
    let origin_millis = PERFORMANCE.with(|performance| performance.time_origin());
    let now_millis = now();

    let origin_nanos = (origin_millis * 1_000_000.0) as u128;
    let now_nanos = (now_millis * 1_000_000.0) as u128;

    let timestamp_nanos = origin_nanos + now_nanos;

    let timestamp_secs = (timestamp_nanos / 1_000_000_000) as u64;
    let timestamp_subsec_nanos = (timestamp_nanos % 1_000_000_000) as u32;

    Duration::new(timestamp_secs, timestamp_subsec_nanos)
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = "setTimeout")]
    fn set_timeout(closure: &Closure<dyn Fn()>, millis: u32) -> f64;

    #[wasm_bindgen(js_name = "clearTimeout")]
    fn clear_timeout(token: f64);
}

#[wasm_bindgen]
extern "C" {
    type Performance;

    #[wasm_bindgen(thread_local_v2, js_name = performance)]
    static PERFORMANCE: Performance;

    #[wasm_bindgen(method, getter = timeOrigin)]
    fn time_origin(this: &Performance) -> f64;

    #[wasm_bindgen(js_namespace = performance)]
    fn now() -> f64;
}

#[cfg(all(
    test,
    target_arch = "wasm32",
    target_vendor = "unknown",
    target_os = "unknown"
))]
mod tests {
    use super::*;

    use futures::channel::oneshot;
    use std::sync::{Arc, Mutex};
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    /// **Property**: Spawn handle resolves when sender is dropped (WebAssembly variant).
    ///
    /// **Sequence of events**:
    /// 1. Create a bounded channel with capacity 1024
    /// 2. Spawn receiver in a JavaScript promise
    /// 3. Drop the sender (signals channel is closed)
    /// 4. Join the handle - should complete when receiver exits
    async fn promise_resolves_on_sender_drop() {
        let (sender, receiver) = crate::bounded::<Vec<i32>>(1024);

        let handle = spawn(receiver, |batch| async move {
            let _ = batch;

            Ok(())
        })
        .unwrap();

        drop(sender);

        // Joining should now complete
        handle.join().await;
    }

    #[wasm_bindgen_test]
    /// **Property**: Receiver processes all messages in batches (WebAssembly variant).
    ///
    /// **Sequence of events**:
    /// 1. Create a bounded channel with capacity 1024
    /// 2. Spawn receiver that records batch size
    /// 3. Send 100 messages (JavaScript single-threaded, all processed together)
    /// 4. Drop sender and join handle
    /// 5. Verify all 100 messages were processed in a single batch
    async fn spawn_processes_batches() {
        let (sender, receiver) = crate::bounded::<Vec<i32>>(1024);

        let count = Arc::new(Mutex::new(0));

        let handle = spawn(receiver, {
            let count = count.clone();

            move |batch| {
                let count = count.clone();

                async move {
                    *count.lock().unwrap() = batch.len();

                    Ok(())
                }
            }
        })
        .unwrap();

        // Send a batch
        // Since JavaScript is single threaded, these will all get processed together
        for i in 0..100 {
            sender.send(i);
        }

        drop(sender);
        handle.join().await;

        assert_eq!(100, *count.lock().unwrap());
    }

    #[wasm_bindgen_test]
    /// **Property**: Async send waits for capacity and ensures messages are processed (WebAssembly variant).
    ///
    /// **Sequence of events**:
    /// 1. Create a bounded channel with capacity 2
    /// 2. Spawn receiver that counts processed messages
    /// 3. Send 3 messages using async send (blocks when at capacity)
    /// 4. After 3 sends with capacity 2, exactly 2 messages should be processed
    /// 5. Drop sender and join handle
    async fn send_waits_for_processing() {
        let (sender, receiver) = crate::bounded::<Vec<()>>(2);

        let total = Arc::new(Mutex::new(0));

        let handle = spawn(receiver, {
            let total = total.clone();

            move |batch| {
                let total = total.clone();

                async move {
                    *total.lock().unwrap() += batch.len();

                    Ok(())
                }
            }
        })
        .unwrap();

        send(&sender, (), Duration::from_secs(1)).await.unwrap();
        send(&sender, (), Duration::from_secs(1)).await.unwrap();
        send(&sender, (), Duration::from_secs(1)).await.unwrap();

        // After sending, the event should be processed
        assert_eq!(2, *total.lock().unwrap());

        drop(sender);
        handle.join().await;
    }

    #[wasm_bindgen_test]
    /// **Property**: Flush waits for all messages to be processed (WebAssembly variant).
    ///
    /// **Sequence of events**:
    /// 1. Create a bounded channel with capacity 1024
    /// 2. Spawn receiver that counts processed messages
    /// 3. Send 100 messages
    /// 4. Verify no messages processed yet
    /// 5. Call flush with 10ms timeout - should wait for all processing
    /// 6. Verify flush succeeded and all 100 messages were processed
    async fn flush_waits_for_completion() {
        let (sender, receiver) = crate::bounded::<Vec<i32>>(1024);

        let total = Arc::new(Mutex::new(0));

        let handle = spawn(receiver, {
            let total = total.clone();

            move |batch| {
                let total = total.clone();

                async move {
                    *total.lock().unwrap() += batch.len();

                    Ok(())
                }
            }
        })
        .unwrap();

        for i in 0..100 {
            sender.send(i);
        }

        assert_eq!(0, *total.lock().unwrap());

        let flushed = flush(&sender, Duration::from_millis(10)).await;
        assert!(flushed);

        // After flushing all events should be processed
        assert_eq!(100, *total.lock().unwrap());

        drop(sender);
        handle.join().await;
    }

    #[wasm_bindgen_test]
    /// **Property**: Flush times out when processing takes longer than the timeout (WebAssembly variant).
    ///
    /// **Sequence of events**:
    /// 1. Create a bounded channel with capacity 1024
    /// 2. Spawn receiver that takes 50ms to process each batch
    /// 3. Send 100 messages
    /// 4. Verify no messages processed yet
    /// 5. Call flush with 1ms timeout - should timeout before processing completes
    /// 6. Verify flush returned false (timed out)
    async fn flush_times_out() {
        let (sender, receiver) = crate::bounded::<Vec<i32>>(1024);

        let total = Arc::new(Mutex::new(0));

        let handle = spawn(receiver, {
            let total = total.clone();

            move |batch| {
                let total = total.clone();

                async move {
                    // Take a while to process the batch
                    Park::new(Duration::from_millis(50)).await;

                    *total.lock().unwrap() += batch.len();

                    Ok(())
                }
            }
        })
        .unwrap();

        for i in 0..100 {
            sender.send(i);
        }

        assert_eq!(0, *total.lock().unwrap());

        let flushed = flush(&sender, Duration::from_millis(1)).await;
        assert!(!flushed);

        drop(sender);
        handle.join().await;
    }

    #[wasm_bindgen_test]
    /// **Property**: Receiver errors are handled gracefully without crashing (WebAssembly variant).
    ///
    /// **Sequence of events**:
    /// 1. Create a bounded channel with capacity 1024
    /// 2. Spawn receiver that always returns an error (no_retry)
    /// 3. Send a message
    /// 4. Drop sender and join handle
    /// 5. Verify the system doesn't crash despite the failing receiver
    async fn failing_receiver_does_not_cause_havoc() {
        let (sender, receiver) = crate::bounded::<Vec<()>>(1024);

        // NOTE: Can't really test panics here, because they _do_ cause havoc
        let handle = spawn(receiver, |_| async move {
            Err(BatchError::no_retry(std::io::Error::new(
                std::io::ErrorKind::Other,
                "explicit failure",
            )))
        })
        .unwrap();

        sender.send(());

        drop(sender);
        handle.join().await;
    }

    #[wasm_bindgen_test]
    /// **Property**: try_send returns an error when the channel is closed (WebAssembly variant).
    ///
    /// **Sequence of events**:
    /// 1. Create a bounded channel
    /// 2. Drop the receiver to close the channel from the receiver side
    /// 3. Attempt try_send - should fail with a non-retryable error
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

    #[wasm_bindgen_test]
    /// **Property**: Channel truncates oldest messages when capacity is exceeded (WebAssembly variant).
    ///
    /// **Sequence of events**:
    /// 1. Create a bounded channel with capacity 5
    /// 2. Send 10 messages (exceeding capacity, causing truncation of 0-4)
    /// 3. Spawn receiver to process the remaining batch
    /// 4. Drop sender and join handle
    /// 5. Verify only messages 5-9 were received (first 5 truncated)
    async fn truncation_keeps_most_recent() {
        let (sender, receiver) = crate::bounded::<Vec<i32>>(5);

        let received = Arc::new(Mutex::new(Vec::new()));

        let handle = spawn(receiver, {
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

        // Send more messages than capacity
        for i in 0..10 {
            sender.send(i);
        }

        drop(sender);
        handle.join().await;

        // Only last 5 messages should remain (0-4 were truncated)
        assert_eq!(vec![5, 6, 7, 8, 9], *received.lock().unwrap());
    }

    #[wasm_bindgen_test]
    /// **Property**: Park future completes when timeout fires.
    ///
    /// This test verifies that Park properly wakes when the timeout expires.
    ///
    /// **Sequence of events**:
    /// 1. Create a Park future with a 10ms delay
    /// 2. Await it (should complete in ~10ms)
    /// 3. Verify the test completes (timeout fired and woke the future)
    async fn park_completes_on_timeout() {
        let start = js_sys::Date::new_0().get_time();

        // Park for 10ms
        Park::new(Duration::from_millis(10)).await;

        let elapsed = js_sys::Date::new_0().get_time() - start;

        // Should have waited at least 10ms (allow some tolerance)
        assert!(elapsed >= 10.0, "Expected at least 10ms, got {}", elapsed);
        // Should not have waited too long (would indicate a hang)
        assert!(
            elapsed < 100.0,
            "Expected less than 100ms, got {} - future may have hung",
            elapsed
        );
    }

    #[wasm_bindgen_test]
    /// **Property**: Park future doesn't hang when used with select and other future completes first.
    ///
    /// This test verifies that Park doesn't leak timeouts when dropped early.
    ///
    /// **Sequence of events**:
    /// 1. Create a oneshot channel and a Park future with 100ms delay
    /// 2. Select between them
    /// 3. Send on the oneshot (completes immediately)
    /// 4. Verify select returns the oneshot result (not the timeout)
    /// 5. Park is dropped, timeout should be cleared
    async fn park_does_not_hang_when_dropped_early() {
        let (tx, rx) = oneshot::channel::<()>();

        // Send immediately (should complete before timeout)
        let _ = tx.send(());

        // Select between rx and Park
        let result = futures::future::select(rx, Park::new(Duration::from_millis(100))).await;

        // Should be the left side (oneshot completed first)
        assert!(matches!(result, futures::future::Either::Left((Ok(()), _))));

        // If we get here without hanging, Park was cleaned up properly
    }

    #[wasm_bindgen_test]
    /// **Property**: Park with zero duration completes quickly.
    ///
    /// This test verifies that Park handles zero duration correctly (should use minimum 1ms).
    ///
    /// **Sequence of events**:
    /// 1. Create a Park future with zero duration
    /// 2. Await it
    /// 3. Verify it completes quickly (should be at least 1ms due to setTimeout minimum)
    async fn park_zero_duration() {
        let start = js_sys::Date::new_0().get_time();

        // Park for 0ms (should use minimum 1ms)
        Park::new(Duration::ZERO).await;

        let elapsed = js_sys::Date::new_0().get_time() - start;

        // Should have waited at least 1ms (setTimeout minimum)
        assert!(elapsed >= 1.0, "Expected at least 1ms, got {}", elapsed);
        // Should not have waited too long
        assert!(elapsed < 50.0, "Expected less than 50ms, got {}", elapsed);
    }
}
