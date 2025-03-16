/*!
Run channels in a JavaScript runtime using a background promise.
*/

#![allow(missing_docs)]

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

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;

use crate::{BatchError, Channel, Receiver};

pub fn spawn<
    T: Channel + Send + 'static,
    F: Future<Output = Result<(), BatchError<T>>> + Send + 'static,
>(
    receiver: Receiver<T>,
    on_batch: impl FnMut(T) -> F + Send + 'static,
) -> io::Result<()>
where
    T::Item: Send + 'static,
{
    // Fire-and-forget promise
    let _ = future_to_promise(async move {
        // `exec` does not panic
        receiver
            .exec(|delay| Park::new(delay).await, on_batch)
            .await;

        Ok(JsValue::UNDEFINED)
    });

    Ok(())
}

/**
Wait for approximately `delay`.

This is semantically more like `park_timeout` than `sleep` because the treatment of the delay itself is quite lax.

This function may wait for longer than `delay` if `delay` is a fractional number of milliseconds.
This function may wait for less than `delay` if `window.setTimeout` cannot be called, or the future is dropped before the delay triggers.
*/
// NOTE: We'll want to be able to re-use this for `flush` timeout somehow
struct Park {
    // `Some` if the timeout hasn't been scheduled yet
    delay: Option<Duration>,
    // `Some` if the timeout has been scheduled
    complete: Option<Closure<dyn Fn()>>,
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
            complete: None,
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

            let complete = Closure::<dyn Fn()>::new(move || {
                ParkState::wake(&state);
            });

            let Some(window) = web_sys::window() else {
                ParkState::wake(&unpinned.state);

                return Poll::Ready(());
            };

            // Set a timeout for at least 1ms that will trigger the wakeup
            let Ok(_) = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                complete.as_ref().unchecked_ref(),
                cmp::max(1, delay.as_millis() as i32),
            ) else {
                ParkState::wake(&unpinned.state);

                return Poll::Ready(());
            };

            unpinned.complete = Some(complete);
        }

        Poll::Pending
    }
}
