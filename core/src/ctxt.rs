/*!
The [`Ctxt`] type.

Context is a shared place to store and retrieve data from the environment. It can be used to enrich [`crate::event::Event`]s with additional [`Props`], without needing to explicitly thread those properties through to them.

Context is modeled like a stack. Pushing properties returns a frame which can be entered and exited to make those properties active on the current thread. Accessing the current context includes the properties for all active frames. This approach makes it possible to isolate context on different threads, as well is in different futures cooperatively executing on the same thread.
*/

use crate::{empty::Empty, props::Props};

/**
Storage for ambient [`Props`].
*/
pub trait Ctxt {
    /**
    The type of [`Props`] used in [`Ctxt::with_current`].
    */
    type Current: Props + ?Sized;

    /**
    The type of frame returned by [`Ctxt::open_root`] and [`Ctxt::open_push`].
    */
    type Frame;

    /**
    Create a frame that will set the context to just the properties in `P`.

    This method can be used to delete properties from the context, by pushing a frame that includes the current set with unwanted properties removed.

    Once a frame is created, it can be entered to make its properties live by passing it to [`Ctxt::enter`]. The frame needs to be exited on the same thread by a call to [`Ctxt::exit`]. Once it's done, it should be disposed by a call to [`Ctxt::close`].
    */
    fn open_root<P: Props>(&self, props: P) -> Self::Frame;

    /**
    Create a frame that will set the context to its current set, plus the properties in `P`.

    Once a frame is created, it can be entered to make its properties live by passing it to [`Ctxt::enter`]. The frame needs to be exited on the same thread by a call to [`Ctxt::exit`]. Once it's done, it should be disposed by a call to [`Ctxt::close`].
    */
    fn open_push<P: Props>(&self, props: P) -> Self::Frame {
        self.with_current(|current| self.open_root(props.and_props(current)))
    }

    /**
    Create a disabled frame.

    The properties in `P` will not be made live when the frame is entered but may still be tracked by the underlying context using the returned frame. This method can be used to inform a context about properties that would have been used under some other conditions.
    */
    fn open_disabled<P: Props>(&self, props: P) -> Self::Frame {
        let _ = props;

        self.open_push(Empty)
    }

    /**
    Make the properties in a frame active.

    Once a frame is entered, it must be exited by a call to [`Ctxt::exit`] on the same thread.
    */
    fn enter(&self, frame: &mut Self::Frame);

    /**
    Access the current context.

    The properties passed to `with` are those from the most recently entered frame.

    This method must call `with` exactly once, even if the current context is empty.
    */
    fn with_current<R, F: FnOnce(&Self::Current) -> R>(&self, with: F) -> R;

    /**
    Make the properties in a frame inactive.

    Once a frame is exited, it can be entered again with a new call to [`Ctxt::enter`], potentially on another thread if [`Ctxt::Frame`] allows it.
    */
    fn exit(&self, frame: &mut Self::Frame);

    /**
    Close a frame, performing any shared cleanup.

    This method should be called whenever a frame is finished. Failing to do so may leak.
    */
    fn close(&self, frame: Self::Frame);
}

impl<'a, C: Ctxt + ?Sized> Ctxt for &'a C {
    type Current = C::Current;
    type Frame = C::Frame;

    fn open_root<P: Props>(&self, props: P) -> Self::Frame {
        (**self).open_root(props)
    }

    fn open_push<P: Props>(&self, props: P) -> Self::Frame {
        (**self).open_push(props)
    }

    fn open_disabled<P: Props>(&self, props: P) -> Self::Frame {
        (**self).open_disabled(props)
    }

    fn enter(&self, frame: &mut Self::Frame) {
        (**self).enter(frame)
    }

    fn with_current<R, F: FnOnce(&Self::Current) -> R>(&self, with: F) -> R {
        (**self).with_current(with)
    }

    fn exit(&self, frame: &mut Self::Frame) {
        (**self).exit(frame)
    }

    fn close(&self, frame: Self::Frame) {
        (**self).close(frame)
    }
}

impl<C: Ctxt> Ctxt for Option<C> {
    type Current = Option<internal::Slot<C::Current>>;
    type Frame = Option<C::Frame>;

    fn with_current<R, F: FnOnce(&Self::Current) -> R>(&self, with: F) -> R {
        match self {
            Some(ctxt) => {
                ctxt.with_current(|props| unsafe { with(&Some(internal::Slot::new(props))) })
            }
            None => with(&None),
        }
    }

    fn open_root<P: Props>(&self, props: P) -> Self::Frame {
        self.as_ref().map(|ctxt| ctxt.open_root(props))
    }

    fn open_push<P: Props>(&self, props: P) -> Self::Frame {
        self.as_ref().map(|ctxt| ctxt.open_push(props))
    }

    fn open_disabled<P: Props>(&self, props: P) -> Self::Frame {
        self.as_ref().map(|ctxt| ctxt.open_disabled(props))
    }

    fn enter(&self, frame: &mut Self::Frame) {
        if let (Some(ctxt), Some(span)) = (self, frame) {
            ctxt.enter(span)
        }
    }

    fn exit(&self, frame: &mut Self::Frame) {
        if let (Some(ctxt), Some(span)) = (self, frame) {
            ctxt.exit(span)
        }
    }

    fn close(&self, frame: Self::Frame) {
        if let (Some(ctxt), Some(span)) = (self, frame) {
            ctxt.close(span)
        }
    }
}

#[cfg(feature = "alloc")]
impl<'a, C: Ctxt + ?Sized + 'a> Ctxt for alloc::boxed::Box<C> {
    type Current = C::Current;
    type Frame = C::Frame;

    fn with_current<R, F: FnOnce(&Self::Current) -> R>(&self, with: F) -> R {
        (**self).with_current(with)
    }

    fn open_root<P: Props>(&self, props: P) -> Self::Frame {
        (**self).open_root(props)
    }

    fn open_push<P: Props>(&self, props: P) -> Self::Frame {
        (**self).open_push(props)
    }

    fn open_disabled<P: Props>(&self, props: P) -> Self::Frame {
        (**self).open_disabled(props)
    }

    fn enter(&self, frame: &mut Self::Frame) {
        (**self).enter(frame)
    }

    fn exit(&self, frame: &mut Self::Frame) {
        (**self).exit(frame)
    }

    fn close(&self, frame: Self::Frame) {
        (**self).close(frame)
    }
}

#[cfg(feature = "alloc")]
impl<'a, C: Ctxt + ?Sized + 'a> Ctxt for alloc::sync::Arc<C> {
    type Current = C::Current;
    type Frame = C::Frame;

    fn with_current<R, F: FnOnce(&Self::Current) -> R>(&self, with: F) -> R {
        (**self).with_current(with)
    }

    fn open_root<P: Props>(&self, props: P) -> Self::Frame {
        (**self).open_root(props)
    }

    fn open_push<P: Props>(&self, props: P) -> Self::Frame {
        (**self).open_push(props)
    }

    fn open_disabled<P: Props>(&self, props: P) -> Self::Frame {
        (**self).open_disabled(props)
    }

    fn enter(&self, frame: &mut Self::Frame) {
        (**self).enter(frame)
    }

    fn exit(&self, frame: &mut Self::Frame) {
        (**self).exit(frame)
    }

    fn close(&self, frame: Self::Frame) {
        (**self).close(frame)
    }
}

impl Ctxt for Empty {
    type Current = Empty;
    type Frame = Empty;

    fn with_current<R, F: FnOnce(&Self::Current) -> R>(&self, with: F) -> R {
        with(&Empty)
    }

    fn open_root<P: Props>(&self, _: P) -> Self::Frame {
        Empty
    }

    fn open_push<P: Props>(&self, _: P) -> Self::Frame {
        Empty
    }

    fn open_disabled<P: Props>(&self, _: P) -> Self::Frame {
        Empty
    }

    fn enter(&self, _: &mut Self::Frame) {}

    fn exit(&self, _: &mut Self::Frame) {}

    fn close(&self, _: Self::Frame) {}
}

mod internal {
    use core::{marker::PhantomData, ops::ControlFlow};

    use crate::{props::Props, str::Str, value::Value};

    // A lifetime-erased borrowed value
    // This type is used to work around the lifetime relationship between
    // `Ctxt::Frame` and the borrowed reference used by `Ctxt::with_current`
    // I looked at using GATs for this, but it wasn't quite capable enough
    pub struct Slot<T: ?Sized>(*const T, PhantomData<*mut fn()>);

    impl<T: ?Sized> Slot<T> {
        // SAFETY: `Slot<T>` must not outlive `&T`
        pub(super) unsafe fn new(v: &T) -> Slot<T> {
            Slot(v as *const T, PhantomData)
        }

        pub(super) fn get(&self) -> &T {
            // SAFETY: `Slot<T>` must not outlive `&T`, as per `Slot::new`
            unsafe { &*self.0 }
        }
    }

    impl<T: Props + ?Sized> Props for Slot<T> {
        fn for_each<'a, F: FnMut(Str<'a>, Value<'a>) -> ControlFlow<()>>(
            &'a self,
            for_each: F,
        ) -> ControlFlow<()> {
            self.get().for_each(for_each)
        }
    }
}

#[cfg(feature = "alloc")]
mod alloc_support {
    use crate::props::ErasedProps;

    use super::*;

    mod internal {
        use alloc::boxed::Box;
        use core::{marker::PhantomData, mem, mem::MaybeUninit, ops::ControlFlow, ptr};

        use crate::{
            props::{ErasedProps, Props},
            str::Str,
            value::Value,
        };

        pub trait DispatchCtxt {
            fn dispatch_with_current(&self, with: &mut dyn FnMut(&ErasedCurrent));

            fn dispatch_open_root(&self, props: &dyn ErasedProps) -> ErasedFrame;
            fn dispatch_open_push(&self, props: &dyn ErasedProps) -> ErasedFrame;
            fn dispatch_open_disabled(&self, props: &dyn ErasedProps) -> ErasedFrame;
            fn dispatch_enter(&self, frame: &mut ErasedFrame);
            fn dispatch_exit(&self, frame: &mut ErasedFrame);
            fn dispatch_close(&self, frame: ErasedFrame);
        }

        pub trait SealedCtxt {
            fn erase_ctxt(&self) -> crate::internal::Erased<&dyn DispatchCtxt>;
        }

        pub struct ErasedCurrent(
            *const dyn ErasedProps,
            PhantomData<fn(&mut dyn ErasedProps)>,
        );

        impl ErasedCurrent {
            // SAFETY: `ErasedCurrent` must not outlive `&v`
            pub(super) unsafe fn new<'a>(v: &'a impl Props) -> Self {
                let v: &'a dyn ErasedProps = v;
                let v: &'a (dyn ErasedProps + 'static) =
                    mem::transmute::<&'a dyn ErasedProps, &'a (dyn ErasedProps + 'static)>(v);

                ErasedCurrent(v as *const dyn ErasedProps, PhantomData)
            }

            pub(super) fn get<'a>(&'a self) -> &'a (dyn ErasedProps + 'a) {
                // SAFETY: `ErasedCurrent` does not outlive `&v`, as per `ErasedCurrent::new`
                unsafe { &*self.0 }
            }
        }

        impl Props for ErasedCurrent {
            fn for_each<'a, F: FnMut(Str<'a>, Value<'a>) -> ControlFlow<()>>(
                &'a self,
                for_each: F,
            ) -> ControlFlow<()> {
                self.get().for_each(for_each)
            }
        }

        // NOTE: This type uses the same approach as `erased-serde` does for erasing
        // small values without needing to allocate for them. We have enough local space
        // to store a value up to 16 bytes, with an alignment up to 8 bytes, inline.
        //
        // The variant here is a bit simpler than `erased-serde`'s, because we already
        // constraint `T` to be `Send + 'static`.

        pub struct ErasedFrame {
            data: RawErasedFrame,
            vtable: RawErasedFrameVTable,
        }

        // SAFETY: `ErasedFrame` can only be constructed from `T: Send`
        unsafe impl Send for ErasedFrame {}

        impl Drop for ErasedFrame {
            fn drop(&mut self) {
                // SAFETY: This frame was created from `T`
                unsafe { (self.vtable.drop)(&mut self.data) }
            }
        }

        union RawErasedFrame {
            boxed: *mut (),
            inline: MaybeUninit<[usize; 2]>,
        }

        struct RawErasedFrameVTable {
            drop: unsafe fn(&mut RawErasedFrame),
        }

        impl ErasedFrame {
            /**
            Whether a value of type `T` can be stored inline.
            */
            pub(super) fn inline<T: Send + 'static>() -> bool {
                mem::size_of::<T>() <= mem::size_of::<RawErasedFrame>()
                    && mem::align_of::<T>() <= mem::align_of::<RawErasedFrame>()
            }

            /**
            Erase a value, storing it inline if it's small enough.
            */
            pub(super) fn new<T: Send + 'static>(value: T) -> Self {
                if Self::inline::<T>() {
                    let mut data = RawErasedFrame {
                        inline: MaybeUninit::uninit(),
                    };

                    unsafe { ptr::write(data.inline.as_mut_ptr() as *mut T, value) };

                    unsafe fn vdrop<T>(data: &mut RawErasedFrame) {
                        // SAFETY: This frame is storing `T` inline
                        ptr::drop_in_place(data.inline.as_mut_ptr() as *mut T)
                    }

                    let vtable = RawErasedFrameVTable { drop: vdrop::<T> };

                    ErasedFrame { data, vtable }
                } else {
                    let data = RawErasedFrame {
                        boxed: Box::into_raw(Box::new(value)) as *mut (),
                    };

                    unsafe fn vdrop<T>(data: &mut RawErasedFrame) {
                        // SAFETY: This frame is storing a boxed `T`
                        drop(unsafe { Box::from_raw(data.boxed as *mut T) });
                    }

                    let vtable = RawErasedFrameVTable { drop: vdrop::<T> };

                    ErasedFrame { data, vtable }
                }
            }

            // SAFETY: This frame must have been created from `T`
            pub(super) unsafe fn get_mut<T: Send + 'static>(&mut self) -> &mut T {
                if Self::inline::<T>() {
                    &mut *(self.data.inline.as_mut_ptr() as *mut T)
                } else {
                    &mut *(self.data.boxed as *mut T)
                }
            }

            // SAFETY: This frame must have been created from `T`
            pub(super) unsafe fn into_inner<T: Send + 'static>(mut self) -> T {
                if Self::inline::<T>() {
                    let data = ptr::read(self.data.inline.as_mut_ptr() as *mut T);
                    mem::forget(self);

                    data
                } else {
                    let data = Box::from_raw(self.data.boxed as *mut T);
                    mem::forget(self);

                    *data
                }
            }
        }
    }

    /**
    An object-safe [`Ctxt`].

    A `dyn ErasedCtxt` can be treated as `impl Ctxt`.
    */
    pub trait ErasedCtxt: internal::SealedCtxt {}

    impl<C: Ctxt> ErasedCtxt for C where C::Frame: Send + 'static {}

    impl<C: Ctxt> internal::SealedCtxt for C
    where
        C::Frame: Send + 'static,
    {
        fn erase_ctxt(&self) -> crate::internal::Erased<&dyn internal::DispatchCtxt> {
            crate::internal::Erased(self)
        }
    }

    impl<C: Ctxt> internal::DispatchCtxt for C
    where
        C::Frame: Send + 'static,
    {
        fn dispatch_with_current(&self, with: &mut dyn FnMut(&internal::ErasedCurrent)) {
            // SAFETY: The borrow passed to `with` is arbitarily short, so `internal::ErasedCurrent::get`
            // cannot outlive `props`
            self.with_current(move |props| with(&unsafe { internal::ErasedCurrent::new(&props) }))
        }

        fn dispatch_open_root(&self, props: &dyn ErasedProps) -> internal::ErasedFrame {
            internal::ErasedFrame::new(self.open_root(props))
        }

        fn dispatch_open_push(&self, props: &dyn ErasedProps) -> internal::ErasedFrame {
            internal::ErasedFrame::new(self.open_push(props))
        }

        fn dispatch_open_disabled(&self, props: &dyn ErasedProps) -> internal::ErasedFrame {
            internal::ErasedFrame::new(self.open_disabled(props))
        }

        fn dispatch_enter(&self, span: &mut internal::ErasedFrame) {
            self.enter(unsafe { span.get_mut::<C::Frame>() })
        }

        fn dispatch_exit(&self, span: &mut internal::ErasedFrame) {
            self.exit(unsafe { span.get_mut::<C::Frame>() })
        }

        fn dispatch_close(&self, span: internal::ErasedFrame) {
            self.close(unsafe { span.into_inner::<C::Frame>() })
        }
    }

    impl<'a> Ctxt for dyn ErasedCtxt + 'a {
        type Current = internal::ErasedCurrent;
        type Frame = internal::ErasedFrame;

        fn with_current<R, F: FnOnce(&Self::Current) -> R>(&self, with: F) -> R {
            let mut f = Some(with);
            let mut r = None;

            self.erase_ctxt().0.dispatch_with_current(&mut |props| {
                r = Some(f.take().expect("called multiple times")(&props));
            });

            r.expect("ctxt didn't call `with`")
        }

        fn open_root<P: Props>(&self, props: P) -> Self::Frame {
            self.erase_ctxt().0.dispatch_open_root(&props)
        }

        fn open_push<P: Props>(&self, props: P) -> Self::Frame {
            self.erase_ctxt().0.dispatch_open_push(&props)
        }

        fn open_disabled<P: Props>(&self, props: P) -> Self::Frame {
            self.erase_ctxt().0.dispatch_open_disabled(&props)
        }

        fn enter(&self, span: &mut Self::Frame) {
            self.erase_ctxt().0.dispatch_enter(span)
        }

        fn exit(&self, span: &mut Self::Frame) {
            self.erase_ctxt().0.dispatch_exit(span)
        }

        fn close(&self, span: Self::Frame) {
            self.erase_ctxt().0.dispatch_close(span)
        }
    }

    impl<'a> Ctxt for dyn ErasedCtxt + Send + Sync + 'a {
        type Current = <dyn ErasedCtxt + 'a as Ctxt>::Current;
        type Frame = <dyn ErasedCtxt + 'a as Ctxt>::Frame;

        fn with_current<R, F: FnOnce(&Self::Current) -> R>(&self, with: F) -> R {
            (self as &(dyn ErasedCtxt + 'a)).with_current(with)
        }

        fn open_root<P: Props>(&self, props: P) -> Self::Frame {
            (self as &(dyn ErasedCtxt + 'a)).open_root(props)
        }

        fn open_push<P: Props>(&self, props: P) -> Self::Frame {
            (self as &(dyn ErasedCtxt + 'a)).open_push(props)
        }

        fn open_disabled<P: Props>(&self, props: P) -> Self::Frame {
            (self as &(dyn ErasedCtxt + 'a)).open_disabled(props)
        }

        fn enter(&self, span: &mut Self::Frame) {
            (self as &(dyn ErasedCtxt + 'a)).enter(span)
        }

        fn exit(&self, span: &mut Self::Frame) {
            (self as &(dyn ErasedCtxt + 'a)).exit(span)
        }

        fn close(&self, span: Self::Frame) {
            (self as &(dyn ErasedCtxt + 'a)).close(span)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn erased_ctxt() {
            struct MyCtxt<'a> {
                a: &'a str,
            }

            struct MyFrame {
                a: String,
            }

            impl<'a> Ctxt for MyCtxt<'a> {
                type Current = (&'a str, &'a str);
                type Frame = MyFrame;

                fn open_root<P: Props>(&self, _: P) -> Self::Frame {
                    MyFrame {
                        a: self.a.to_owned(),
                    }
                }

                fn enter(&self, frame: &mut Self::Frame) {
                    assert_eq!(self.a, frame.a);
                }

                fn exit(&self, frame: &mut Self::Frame) {
                    assert_eq!(self.a, frame.a);
                }

                fn close(&self, frame: Self::Frame) {
                    assert_eq!(self.a, frame.a);
                }

                fn with_current<R, F: FnOnce(&Self::Current) -> R>(&self, with: F) -> R {
                    with(&("a", self.a))
                }
            }

            let borrowed = String::from("value");

            let ctxt = MyCtxt { a: &borrowed };

            let ctxt = &ctxt as &dyn ErasedCtxt;

            let mut frame = ctxt.open_root(Empty);

            ctxt.enter(&mut frame);

            ctxt.with_current(|props| {
                assert_eq!("value", props.pull::<crate::str::Str, _>("a").unwrap());
            });

            ctxt.exit(&mut frame);

            ctxt.close(frame);
        }
    }

    #[test]
    fn erased_frame_zero_sized() {
        #[derive(PartialEq, Eq, Debug)]
        struct Data;

        assert!(internal::ErasedFrame::inline::<Data>());
        let mut frame = internal::ErasedFrame::new(Data);

        assert_eq!(Data, *unsafe { frame.get_mut::<Data>() });

        let data = unsafe { frame.into_inner::<Data>() };

        assert_eq!(Data, data);
    }

    #[test]
    fn erased_frame_inline() {
        struct Data(usize);

        impl Drop for Data {
            fn drop(&mut self) {}
        }

        assert!(internal::ErasedFrame::inline::<Data>());
        let mut frame = internal::ErasedFrame::new(Data(42));

        assert_eq!(42, unsafe { frame.get_mut::<Data>() }.0);

        let data = unsafe { frame.into_inner::<Data>() };

        assert_eq!(42, data.0);
    }

    #[test]
    fn erased_frame_boxed() {
        assert!(!internal::ErasedFrame::inline::<String>());
        let mut frame = internal::ErasedFrame::new(String::from("some data"));

        assert_eq!("some data", unsafe { frame.get_mut::<String>() });

        let data = unsafe { frame.into_inner::<String>() };

        assert_eq!("some data", data);
    }
}

#[cfg(feature = "alloc")]
pub use alloc_support::*;

#[cfg(test)]
mod tests {
    use super::*;

    use core::{cell::Cell, ops::ControlFlow};

    #[test]
    fn open_push_precedence() {
        struct MyCtxt;

        impl Ctxt for MyCtxt {
            type Current = (&'static str, usize);
            type Frame = ();

            fn open_root<P: Props>(&self, props: P) -> Self::Frame {
                assert_eq!(2, props.pull::<i32, _>("prop").unwrap());
            }

            fn enter(&self, _: &mut Self::Frame) {}

            fn exit(&self, _: &mut Self::Frame) {}

            fn close(&self, _: Self::Frame) {}

            fn with_current<R, F: FnOnce(&Self::Current) -> R>(&self, with: F) -> R {
                with(&("prop", 1))
            }
        }

        MyCtxt.open_push(("prop", 2));
    }

    #[test]
    fn option_ctxt() {
        struct MyCtxt {
            count: Cell<usize>,
        }

        struct MyFrame {
            count: usize,
        }

        impl Ctxt for MyCtxt {
            type Current = (&'static str, usize);
            type Frame = MyFrame;

            fn open_root<P: Props>(&self, props: P) -> Self::Frame {
                let mut count = 0;

                let _ = props.for_each(|_, _| {
                    count += 1;
                    ControlFlow::Continue(())
                });

                MyFrame { count }
            }

            fn enter(&self, frame: &mut Self::Frame) {
                self.count.set(self.count.get() + frame.count);
            }

            fn exit(&self, frame: &mut Self::Frame) {
                self.count.set(self.count.get() - frame.count);
            }

            fn close(&self, _: Self::Frame) {}

            fn with_current<R, F: FnOnce(&Self::Current) -> R>(&self, with: F) -> R {
                with(&("count", self.count.get()))
            }
        }

        for (ctxt, expected) in [
            (
                Some(MyCtxt {
                    count: Cell::new(0),
                }),
                Some(5),
            ),
            (None, None),
        ] {
            let mut frame = ctxt.open_root([("a", 1), ("b", 2), ("c", 3), ("d", 4), ("e", 5)]);

            ctxt.enter(&mut frame);

            ctxt.with_current(|props| {
                assert_eq!(expected, props.pull::<usize, _>("count"));
            });

            ctxt.exit(&mut frame);

            ctxt.close(frame);
        }
    }
}
