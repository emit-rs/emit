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
This function may wait for less than `delay` if `window.setTimeout` cannot be called, or the future is dropped before the delay triggers.
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
        // The timeout is done using `window.setTimeout`
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

    use std::sync::{Arc, Mutex};

    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
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
}
