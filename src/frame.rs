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
    ctxt: C,
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
        let scope = mem::ManuallyDrop::new(ctxt.open_push(props));

        Frame { ctxt, scope }
    }

    /**
    Get a frame for just the properties in `props`.
    */
    #[track_caller]
    #[must_use = "call `enter`, `call`, or `in_future` to make the properties active"]
    pub fn root(ctxt: C, props: impl Props) -> Self {
        let scope = mem::ManuallyDrop::new(ctxt.open_root(props));

        Frame { ctxt, scope }
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

impl<C: Ctxt> Drop for Frame<C> {
    fn drop(&mut self) {
        // SAFETY: We're being dropped, so won't access `scope` again
        self.ctxt
            .close(unsafe { mem::ManuallyDrop::take(&mut self.scope) })
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
