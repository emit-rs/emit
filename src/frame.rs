/*!
The [`Frame`] type.
*/

use core::{
    future::Future,
    marker::PhantomData,
    mem,
    pin::Pin,
    task::{Context, Poll},
};
use emit_core::{ctxt::Ctxt, props::Props};

/**
A set of ambient properties that are cleaned up automatically.

This type is a wrapper around a [`Ctxt`] that simplifies ambient property management. A frame containing ambient properties can be created through [`Frame::push`] or [`Frame::root`]. Those properties can be activated by calling [`Frame::enter`]. The returned [`EnterGuard`] will automatically deactivate those properties when dropped.

A frame can be converted into a future through [`Frame::in_future`] that enters and exits on each call to [`Future::poll`] so ambient properties can follow a future as it executes in an async runtime.
*/
pub struct Frame<C: Ctxt> {
    scope: mem::ManuallyDrop<C::Frame>,
    ctxt: mem::ManuallyDrop<C>,
}

impl<C: Ctxt> Frame<C> {
    /**
    Get a frame with the current set of ambient properties.
    */
    #[track_caller]
    #[must_use = "call `enter`, `call`, or `in_future` to make the pushed properties active"]
    pub fn current(ctxt: C) -> Self {
        Self::push(ctxt, crate::empty::Empty)
    }

    /**
    Get a frame with the given `props` pushed to the current set.
    */
    #[track_caller]
    #[must_use = "call `enter`, `call`, or `in_future` to make the pushed properties active"]
    pub fn push(ctxt: C, props: impl Props) -> Self {
        let scope = ctxt.open_push(props);

        Self::from_parts(ctxt, scope)
    }

    /**
    Get a frame for just the properties in `props`.
    */
    #[track_caller]
    #[must_use = "call `enter`, `call`, or `in_future` to make the properties active"]
    pub fn root(ctxt: C, props: impl Props) -> Self {
        let scope = ctxt.open_root(props);

        Self::from_parts(ctxt, scope)
    }

    /**
    Get a disabled frame.

    The properties in `props` will not be visible when the frame is entered. This method should be used when `props` could have been pushed, but were filtered out.
    */
    #[track_caller]
    #[must_use = "call `enter`, `call`, or `in_future` to make the properties active"]
    pub fn disabled(ctxt: C, props: impl Props) -> Self {
        let scope = ctxt.open_disabled(props);

        Self::from_parts(ctxt, scope)
    }

    /**
    Access the properties in this frame.
    */
    #[track_caller]
    pub fn with<R>(&mut self, with: impl FnOnce(&C::Current) -> R) -> R {
        self.enter().with(with)
    }

    /**
    Activate this frame.

    The properties in this frame will be visible until the returned [`EnterGuard`] is dropped.
    */
    #[track_caller]
    pub fn enter(&mut self) -> EnterGuard<C> {
        self.ctxt.enter(&mut self.scope);

        EnterGuard {
            scope: self,
            _marker: PhantomData,
        }
    }

    /**
    Activate this frame for the duration of `scope`.

    The properties in this frame will be visible while `scope` is executing.
    */
    #[track_caller]
    pub fn call<R>(mut self, scope: impl FnOnce() -> R) -> R {
        let __guard = self.enter();
        scope()
    }

    /**
    Get a future that will activate this frame on each call to [`Future::poll`].

    The properties in this frame will be visible while the inner future is executing.
    */
    #[track_caller]
    #[must_use = "futures do nothing unless polled"]
    pub fn in_future<F>(self, future: F) -> FrameFuture<C, F> {
        FrameFuture {
            frame: self,
            future,
        }
    }

    /**
    Get a reference to the underlying context frame.
    */
    pub fn inner(&self) -> &C::Frame {
        &self.scope
    }

    /**
    Get an exclusive reference to the underlying context frame.
    */
    pub fn inner_mut(&mut self) -> &mut C::Frame {
        &mut self.scope
    }

    /**
    Create a frame from its constituent parts.

    In order to be correct, this method requires:

    1. That `scope` was created by `ctxt`.
    2. That `scope` is not currently entered.
    */
    pub const fn from_parts(ctxt: C, scope: C::Frame) -> Self {
        let scope = mem::ManuallyDrop::new(scope);
        let ctxt = mem::ManuallyDrop::new(ctxt);

        Frame { ctxt, scope }
    }

    /**
    Split the frame into its raw parts.

    The original frame can be re-constituted by calling [`Frame::from_parts`].
    */
    pub fn into_parts(mut self) -> (C, C::Frame) {
        // SAFETY: We're moving ownership out of `Frame` without running its `Drop`
        let ctxt = unsafe { mem::ManuallyDrop::take(&mut self.ctxt) };
        let scope = unsafe { mem::ManuallyDrop::take(&mut self.scope) };

        mem::forget(self);

        (ctxt, scope)
    }
}

impl<C: Ctxt> Drop for Frame<C> {
    fn drop(&mut self) {
        // SAFETY: We're being dropped, so won't access fields again
        let ctxt = unsafe { mem::ManuallyDrop::take(&mut self.ctxt) };
        let scope = unsafe { mem::ManuallyDrop::take(&mut self.scope) };

        ctxt.close(scope)
    }
}

/**
The result of calling [`Frame::enter`].

The guard will de-activate the properties in its protected frame on drop.
*/
pub struct EnterGuard<'a, C: Ctxt> {
    scope: &'a mut Frame<C>,
    _marker: PhantomData<*mut fn()>,
}

impl<'a, C: Ctxt> EnterGuard<'a, C> {
    /**
    Access the properties in this frame.
    */
    #[track_caller]
    pub fn with<R>(&mut self, with: impl FnOnce(&C::Current) -> R) -> R {
        self.scope.ctxt.with_current(with)
    }
}

impl<'a, C: Ctxt> Drop for EnterGuard<'a, C> {
    fn drop(&mut self) {
        self.scope.ctxt.exit(&mut self.scope.scope);
    }
}

/**
The result of calling [`Frame::in_future`].
*/
pub struct FrameFuture<C: Ctxt, F> {
    frame: Frame<C>,
    future: F,
}

impl<C: Ctxt, F: Future> Future for FrameFuture<C, F> {
    type Output = F::Output;

    #[track_caller]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // SAFETY: The fields of `FrameFuture` remain pinned
        let unpinned = unsafe { Pin::get_unchecked_mut(self) };

        let __guard = unpinned.frame.enter();

        // SAFETY: `FrameFuture::future` is pinned
        unsafe { Pin::new_unchecked(&mut unpinned.future) }.poll(cx)
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "std")]
    use super::*;

    #[cfg(feature = "std")]
    #[test]
    fn frame_manual() {
        let ctxt = crate::platform::thread_local_ctxt::ThreadLocalCtxt::new();

        let frame = Frame::push(&ctxt, ("a", 1));

        let (ctxt, mut inner) = frame.into_parts();
        ctxt.enter(&mut inner);

        ctxt.with_current(|props| {
            assert_eq!(1, props.pull::<i32, _>("a").unwrap());
        });

        ctxt.exit(&mut inner);

        let frame = Frame::from_parts(ctxt, inner);

        drop(frame);
    }

    #[cfg(feature = "std")]
    #[test]
    fn frame_exec() {
        let ctxt = crate::platform::thread_local_ctxt::ThreadLocalCtxt::new();

        let mut frame = Frame::push(&ctxt, ("a", 1));

        frame.with(|props| {
            assert_eq!(1, props.pull::<i32, _>("a").unwrap());
        });

        drop(frame);
    }

    #[cfg(feature = "std")]
    #[tokio::test]
    async fn frame_in_future() {
        let ctxt = crate::platform::thread_local_ctxt::ThreadLocalCtxt::new();

        let frame = Frame::push(&ctxt, ("a", 1));

        frame
            .in_future(async {
                Frame::current(&ctxt).with(|props| {
                    assert_eq!(1, props.pull::<i32, _>("a").unwrap());
                })
            })
            .await;
    }
}
