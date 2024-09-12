/*!
The [`Emitter`] type.

Emitters are the receivers of diagnostic data in the form of [`Event`]s. A typical emitter will translate and forward those events to some outside observer. That could be a file containing newline JSON, a remote observability system via OTLP, or anything else.

Emitters are asynchronous, so emitted diagnostics are not guaranteed to have been fully processed until a call to [`Emitter::blocking_flush`].
*/

use core::time::Duration;

use crate::{
    and::And,
    empty::Empty,
    event::{Event, ToEvent},
    props::ErasedProps,
};

/**
An asynchronous destination for diagnostic data.

Once [`Event`]s are emitted through [`Emitter::emit`], a call to [`Emitter::blocking_flush`] must be made to ensure they're fully processed. This should be done once before the emitter is disposed, but may be more frequent for auditing.
*/
pub trait Emitter {
    /**
    Emit an [`Event`].
    */
    fn emit<E: ToEvent>(&self, evt: E);

    /**
    Block for up to `timeout`, waiting for all diagnostic data emitted up to this point to be fully processed.

    This method returns `true` if the flush completed, and `false` if it timed out.

    If an emitter doesn't need to flush, this method should immediately return `true`. If an emitted doesn't support flushing, this method should immediately return `false`.
    */
    fn blocking_flush(&self, timeout: Duration) -> bool;

    /**
    Emit events to both `self` and `other`.
    */
    fn and_to<U>(self, other: U) -> And<Self, U>
    where
        Self: Sized,
    {
        And::new(self, other)
    }

    /**
    Wrap the emitter, transforming or filtering [`Event`]s before it receives them.

    Flushing defers to the wrapped emitter.
    */
    fn wrap_emitter<W: wrapping::Wrapping>(self, wrapping: W) -> Wrap<Self, W>
    where
        Self: Sized,
    {
        Wrap {
            emitter: self,
            wrapping,
        }
    }
}

impl<'a, T: Emitter + ?Sized> Emitter for &'a T {
    fn emit<E: ToEvent>(&self, evt: E) {
        (**self).emit(evt)
    }

    fn blocking_flush(&self, timeout: Duration) -> bool {
        (**self).blocking_flush(timeout)
    }
}

#[cfg(feature = "alloc")]
impl<'a, T: Emitter + ?Sized + 'a> Emitter for alloc::boxed::Box<T> {
    fn emit<E: ToEvent>(&self, evt: E) {
        (**self).emit(evt)
    }

    fn blocking_flush(&self, timeout: Duration) -> bool {
        (**self).blocking_flush(timeout)
    }
}

#[cfg(feature = "alloc")]
impl<'a, T: Emitter + ?Sized + 'a> Emitter for alloc::sync::Arc<T> {
    fn emit<E: ToEvent>(&self, evt: E) {
        (**self).emit(evt)
    }

    fn blocking_flush(&self, timeout: Duration) -> bool {
        (**self).blocking_flush(timeout)
    }
}

impl<T: Emitter> Emitter for Option<T> {
    fn emit<E: ToEvent>(&self, evt: E) {
        match self {
            Some(target) => target.emit(evt),
            None => Empty.emit(evt),
        }
    }

    fn blocking_flush(&self, timeout: Duration) -> bool {
        match self {
            Some(target) => target.blocking_flush(timeout),
            None => Empty.blocking_flush(timeout),
        }
    }
}

impl Emitter for Empty {
    fn emit<E: ToEvent>(&self, _: E) {}

    fn blocking_flush(&self, _: Duration) -> bool {
        true
    }
}

impl Emitter for fn(&Event<&dyn ErasedProps>) {
    fn emit<E: ToEvent>(&self, evt: E) {
        (self)(&evt.to_event().erase())
    }

    fn blocking_flush(&self, _: Duration) -> bool {
        true
    }
}

/**
An [`Emitter`] from a function.

This type can be created directly, or via [`from_fn`].
*/
pub struct FromFn<F = fn(&Event<&dyn ErasedProps>)>(F);

impl<F> FromFn<F> {
    /**
    Wrap the given emitter function.
    */
    pub const fn new(emitter: F) -> FromFn<F> {
        FromFn(emitter)
    }
}

impl<F: Fn(&Event<&dyn ErasedProps>)> Emitter for FromFn<F> {
    fn emit<E: ToEvent>(&self, evt: E) {
        (self.0)(&evt.to_event().erase())
    }

    fn blocking_flush(&self, _: Duration) -> bool {
        true
    }
}

/**
Create an [`Emitter`] from a function.

The input function is assumed not to perform any background work that needs flushing.
*/
pub fn from_fn<F: Fn(&Event<&dyn ErasedProps>)>(f: F) -> FromFn<F> {
    FromFn::new(f)
}

impl<T: Emitter, U: Emitter> Emitter for And<T, U> {
    fn emit<E: ToEvent>(&self, evt: E) {
        let evt = evt.to_event();

        self.left().emit(&evt);
        self.right().emit(&evt);
    }

    fn blocking_flush(&self, timeout: Duration) -> bool {
        // Approximate; give each target an equal
        // time to flush. With a monotonic clock
        // we could measure the time each takes
        // to flush and track in our timeout
        let timeout = timeout / 2;

        let lhs = self.left().blocking_flush(timeout);
        let rhs = self.right().blocking_flush(timeout);

        lhs && rhs
    }
}

/**
An [`Emitter`] that can transform or filter events before forwarding them through.

This type is returned by [`Emitter::wrap_emitter`].
*/
pub struct Wrap<E, W> {
    emitter: E,
    wrapping: W,
}

impl<E, W> Wrap<E, W> {
    /**
    Get a reference to the underlying [`Emitter`].
    */
    pub const fn emitter(&self) -> &E {
        &self.emitter
    }

    /**
    Get a reference to the underlying [`wrapping::Wrapping`].
    */
    pub const fn wrapping(&self) -> &W {
        &self.wrapping
    }
}

impl<E: Emitter, W: wrapping::Wrapping> Emitter for Wrap<E, W> {
    fn emit<T: ToEvent>(&self, evt: T) {
        self.wrapping.wrap(&self.emitter, evt.to_event())
    }

    fn blocking_flush(&self, timeout: Duration) -> bool {
        self.emitter.blocking_flush(timeout)
    }
}

/**
Wrap an [`Emitter`] in a [`wrapping::Wrapping`], transforming or filtering [`Event`]s before it receives them.

Flushing defers to the wrapped emitter.
*/
pub fn wrap<E: Emitter, W: wrapping::Wrapping>(emitter: E, wrapping: W) -> Wrap<E, W> {
    emitter.wrap_emitter(wrapping)
}

pub mod wrapping {
    /*!
    The [`Wrapping`] type.

    This module defines a middleware API for [`Emitter`]s. An [`Emitter`] can be wrapped through [`Emitter::wrap_emitter`] in a [`Wrapping`] that can manipulate an [`Event`] before forwarding it to the wrapped emitter.
    */

    use super::*;

    use crate::filter::Filter;

    /**
    A transformation or filter applied to an [`Event`] before emitting it through an [`Emitter`].
    */
    pub trait Wrapping {
        /**
        Wrap the given emitter.
        */
        fn wrap<O: Emitter, E: ToEvent>(&self, output: O, evt: E);
    }

    impl<'a, T: Wrapping + ?Sized> Wrapping for &'a T {
        fn wrap<O: Emitter, E: ToEvent>(&self, output: O, evt: E) {
            (**self).wrap(output, evt)
        }
    }

    /**
    A [`Wrapping`] from a function.

    This type can be created directly, or via [`from_fn`].
    */
    pub struct FromFn<F = fn(&dyn ErasedEmitter, Event<&dyn ErasedProps>)>(F);

    impl<F> FromFn<F> {
        /**
        Wrap the given function.
        */
        pub const fn new(wrapping: F) -> FromFn<F> {
            FromFn(wrapping)
        }
    }

    impl<F: Fn(&dyn ErasedEmitter, Event<&dyn ErasedProps>)> Wrapping for FromFn<F> {
        fn wrap<O: Emitter, E: ToEvent>(&self, output: O, evt: E) {
            (self.0)(&output, evt.to_event().erase())
        }
    }

    /**
    Create a [`Wrapping`] from a function.
    */
    pub fn from_fn<F: Fn(&dyn ErasedEmitter, Event<&dyn ErasedProps>)>(f: F) -> FromFn<F> {
        FromFn::new(f)
    }

    /**
    A [`Wrapping`] from a filter.

    The filter will be applied to incoming [`Event`]s and only passed to the output [`Emitter`] when they match.
    */
    pub struct FromFilter<F>(F);

    impl<F> FromFilter<F> {
        /**
        Wrap the given filter.
        */
        pub const fn new(filter: F) -> FromFilter<F> {
            FromFilter(filter)
        }
    }

    impl<F: Filter> Wrapping for FromFilter<F> {
        fn wrap<O: Emitter, E: ToEvent>(&self, output: O, evt: E) {
            if self.0.matches(&evt) {
                output.emit(evt);
            }
        }
    }

    /**
    Create a [`Wrapping`] from a filter.
    */
    pub fn from_filter<F: Filter>(filter: F) -> FromFilter<F> {
        FromFilter::new(filter)
    }

    mod internal {
        use crate::{emitter::ErasedEmitter, event::Event, props::ErasedProps};

        pub trait DispatchWrapping {
            fn dispatch_wrap(&self, emitter: &dyn ErasedEmitter, evt: Event<&dyn ErasedProps>);
        }

        pub trait SealedWrapping {
            fn erase_wrapping(&self) -> crate::internal::Erased<&dyn DispatchWrapping>;
        }
    }

    /**
    An object-safe [`Wrapping`].

    A `dyn ErasedWrapping` can be treated as `impl Wrapping`.
    */
    pub trait ErasedWrapping: internal::SealedWrapping {}

    impl<T: Wrapping> ErasedWrapping for T {}

    impl<T: Wrapping> internal::SealedWrapping for T {
        fn erase_wrapping(&self) -> crate::internal::Erased<&dyn internal::DispatchWrapping> {
            crate::internal::Erased(self)
        }
    }

    impl<T: Wrapping> internal::DispatchWrapping for T {
        fn dispatch_wrap(&self, emitter: &dyn ErasedEmitter, evt: Event<&dyn ErasedProps>) {
            self.wrap(emitter, evt)
        }
    }

    impl<'a> Wrapping for dyn ErasedWrapping + 'a {
        fn wrap<O: Emitter, E: ToEvent>(&self, output: O, evt: E) {
            self.erase_wrapping()
                .0
                .dispatch_wrap(&output, evt.to_event().erase())
        }
    }

    impl<'a> Wrapping for dyn ErasedWrapping + Send + Sync + 'a {
        fn wrap<O: Emitter, E: ToEvent>(&self, output: O, evt: E) {
            (self as &(dyn ErasedWrapping + 'a)).wrap(output, evt)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        use crate::{path::Path, template::Template};

        use std::cell::Cell;

        struct MyWrapping(Cell<usize>);

        impl Wrapping for MyWrapping {
            fn wrap<O: Emitter, E: ToEvent>(&self, _: O, _: E) {
                self.0.set(self.0.get() + 1);
            }
        }

        #[test]
        fn erased_wrapping() {
            let wrapping = MyWrapping(Cell::new(0));

            {
                let wrapping = &wrapping as &dyn ErasedWrapping;

                wrapping.wrap(
                    Empty,
                    Event::new(
                        Path::new_unchecked("a"),
                        Template::literal("test"),
                        Empty,
                        Empty,
                    ),
                );
            }

            assert_eq!(1, wrapping.0.get());
        }

        #[test]
        fn from_fn_wrapping() {
            let calls = Cell::new(0);

            let wrapping = from_fn(|_, _| {
                calls.set(calls.get() + 1);
            });

            wrapping.wrap(
                Empty,
                Event::new(
                    Path::new_unchecked("a"),
                    Template::literal("test"),
                    Empty,
                    Empty,
                ),
            );

            assert_eq!(1, calls.get());
        }
    }
}

mod internal {
    use core::time::Duration;

    use crate::{event::Event, props::ErasedProps};

    pub trait DispatchEmitter {
        fn dispatch_emit(&self, evt: &Event<&dyn ErasedProps>);
        fn dispatch_blocking_flush(&self, timeout: Duration) -> bool;
    }

    pub trait SealedEmitter {
        fn erase_emitter(&self) -> crate::internal::Erased<&dyn DispatchEmitter>;
    }
}

/**
An object-safe [`Emitter`].

A `dyn ErasedEmitter` can be treated as `impl Emitter`.
*/
pub trait ErasedEmitter: internal::SealedEmitter {}

impl<T: Emitter> ErasedEmitter for T {}

impl<T: Emitter> internal::SealedEmitter for T {
    fn erase_emitter(&self) -> crate::internal::Erased<&dyn internal::DispatchEmitter> {
        crate::internal::Erased(self)
    }
}

impl<T: Emitter> internal::DispatchEmitter for T {
    fn dispatch_emit(&self, evt: &Event<&dyn ErasedProps>) {
        self.emit(evt)
    }

    fn dispatch_blocking_flush(&self, timeout: Duration) -> bool {
        self.blocking_flush(timeout)
    }
}

impl<'a> Emitter for dyn ErasedEmitter + 'a {
    fn emit<E: ToEvent>(&self, evt: E) {
        self.erase_emitter()
            .0
            .dispatch_emit(&evt.to_event().erase())
    }

    fn blocking_flush(&self, timeout: Duration) -> bool {
        self.erase_emitter().0.dispatch_blocking_flush(timeout)
    }
}

impl<'a> Emitter for dyn ErasedEmitter + Send + Sync + 'a {
    fn emit<E: ToEvent>(&self, evt: E) {
        (self as &(dyn ErasedEmitter + 'a)).emit(evt)
    }

    fn blocking_flush(&self, timeout: Duration) -> bool {
        (self as &(dyn ErasedEmitter + 'a)).blocking_flush(timeout)
    }
}

#[cfg(test)]
mod tests {
    use crate::{path::Path, props::Props, template::Template};

    use super::*;

    use std::{cell::Cell, sync::Mutex};

    struct MyEmitter {
        pending: Mutex<Vec<String>>,
        emitted: Mutex<Vec<String>>,
    }

    impl MyEmitter {
        fn new() -> Self {
            MyEmitter {
                pending: Mutex::new(Vec::new()),
                emitted: Mutex::new(Vec::new()),
            }
        }

        fn emitted(&self) -> Vec<String> {
            (*self.emitted.lock().unwrap()).clone()
        }
    }

    impl Emitter for MyEmitter {
        fn emit<E: ToEvent>(&self, evt: E) {
            let rendered = evt.to_event().msg().to_string();
            self.pending.lock().unwrap().push(rendered);
        }

        fn blocking_flush(&self, _: Duration) -> bool {
            let mut pending = self.pending.lock().unwrap();
            let mut emitted = self.emitted.lock().unwrap();

            emitted.extend(pending.drain(..));

            true
        }
    }

    #[test]
    fn erased_emitter() {
        let emitter = MyEmitter::new();

        {
            let emitter = &emitter as &dyn ErasedEmitter;

            emitter.emit(Event::new(
                Path::new_unchecked("a"),
                Template::literal("event 1"),
                Empty,
                Empty,
            ));
            emitter.blocking_flush(Duration::from_secs(0));
        }

        assert_eq!(vec![String::from("event 1")], emitter.emitted());
    }

    #[test]
    fn option_emitter() {
        for (emitter, expected) in [
            (Some(MyEmitter::new()), vec![String::from("event 1")]),
            (None, vec![]),
        ] {
            emitter.emit(Event::new(
                Path::new_unchecked("a"),
                Template::literal("event 1"),
                Empty,
                Empty,
            ));
            emitter.blocking_flush(Duration::from_secs(0));

            let emitted = emitter.map(|emitter| emitter.emitted()).unwrap_or_default();

            assert_eq!(expected, emitted);
        }
    }

    #[test]
    fn from_fn_emitter() {
        let count = Cell::new(0);

        let emitter = from_fn(|evt| {
            assert_eq!(Path::new_unchecked("a"), evt.mdl());

            count.set(count.get() + 1);
        });

        emitter.emit(Event::new(
            Path::new_unchecked("a"),
            Template::literal("event 1"),
            Empty,
            Empty,
        ));

        assert_eq!(1, count.get());
    }

    #[test]
    fn and_emitter() {
        let emitter = MyEmitter::new().and_to(MyEmitter::new());

        emitter.emit(Event::new(
            Path::new_unchecked("a"),
            Template::literal("event 1"),
            Empty,
            Empty,
        ));
        emitter.blocking_flush(Duration::from_secs(0));

        assert_eq!(vec![String::from("event 1")], emitter.left().emitted());
        assert_eq!(vec![String::from("event 1")], emitter.right().emitted());
    }

    #[test]
    fn wrap_emitter() {
        let count = Cell::new(0);
        let emitter = from_fn(|evt| {
            assert_eq!(1, evt.props().pull::<i32, _>("appended").unwrap());

            count.set(count.get() + 1);
        })
        .wrap_emitter(wrapping::from_fn(|output, evt| {
            output.emit(evt.map_props(|props| props.and_props(("appended", 1))));
        }));

        emitter.emit(Event::new(
            Path::new_unchecked("a"),
            Template::literal("event 1"),
            Empty,
            Empty,
        ));

        assert_eq!(1, count.get());
    }
}
