/*!
Run channels in a JavaScript runtime using a background promise.
*/

#![allow(missing_docs)]

use std::{
    cell::RefCell,
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
    let _ = future_to_promise(async move {
        // `exec` does not panic
        receiver.exec(|delay| sleep(delay), on_batch).await;

        Ok(JsValue::UNDEFINED)
    });

    Ok(())
}

async fn sleep(delay: Duration) {
    let f = Timeout {
        delay: Some(delay),
        complete: None,
        state: Rc::new(RefCell::new(TimeoutState {
            done: false,
            wakers: Vec::new(),
        })),
    };

    f.await
}

struct Timeout {
    delay: Option<Duration>,
    complete: Option<Closure<dyn Fn()>>,
    state: Rc<RefCell<TimeoutState>>,
}

struct TimeoutState {
    done: bool,
    wakers: Vec<Waker>,
}

impl Future for Timeout {
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
                let mut state = state.borrow_mut();

                state.done = true;
                let wakers = mem::take(&mut state.wakers);

                drop(state);

                for waker in wakers {
                    waker.wake();
                }
            });

            web_sys::window()
                .unwrap()
                .set_timeout_with_callback_and_timeout_and_arguments_0(
                    complete.as_ref().unchecked_ref(),
                    delay.as_millis() as i32,
                )
                .unwrap();

            unpinned.complete = Some(complete);
        }

        Poll::Pending
    }
}
