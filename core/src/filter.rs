/*!
The [`Filter`] type.

Filters reduce the burden of diagnostics by limiting the volume of data generated. A typical filter will only match events with a certain level or higher, or it may exclude all events for a particularly noisy module.
*/

use core::time::Duration;

use crate::{
    and::And,
    emitter::Emitter,
    empty::Empty,
    event::{Event, ToEvent},
    or::Or,
    props::ErasedProps,
};

/**
A filter over [`Event`]s.

Filters can be evaluated with a call to [`Filter::matches`].
*/
pub trait Filter {
    /**
    Evaluate an event against the filter.

    If this method return `true` then the event has passed the filter. If this method returns `false` then the event has failed the filter.
    */
    fn matches<E: ToEvent>(&self, evt: E) -> bool;

    /**
    `self && other`.

    If `self` evaluates to `true` then `other` will be evaluated.
    */
    fn and_when<U>(self, other: U) -> And<Self, U>
    where
        Self: Sized,
    {
        And::new(self, other)
    }

    /**
    `self || other`.

    If `self` evaluates to `false` then `other` will be evaluated.
    */
    fn or_when<U>(self, other: U) -> Or<Self, U>
    where
        Self: Sized,
    {
        Or::new(self, other)
    }

    /**
    Wrap an [`Emitter`], only emitting events if they pass the filter.
    */
    fn wrap_emitter<E>(self, emitter: E) -> FilteredEmitter<Self, E>
    where
        Self: Sized,
    {
        FilteredEmitter::new(self, emitter)
    }
}

impl<'a, F: Filter + ?Sized> Filter for &'a F {
    fn matches<E: ToEvent>(&self, evt: E) -> bool {
        (**self).matches(evt)
    }
}

#[cfg(feature = "alloc")]
impl<'a, F: Filter + ?Sized + 'a> Filter for alloc::boxed::Box<F> {
    fn matches<E: ToEvent>(&self, evt: E) -> bool {
        (**self).matches(evt)
    }
}

#[cfg(feature = "alloc")]
impl<'a, F: Filter + ?Sized + 'a> Filter for alloc::sync::Arc<F> {
    fn matches<E: ToEvent>(&self, evt: E) -> bool {
        (**self).matches(evt)
    }
}

impl<F: Filter> Filter for Option<F> {
    fn matches<E: ToEvent>(&self, evt: E) -> bool {
        match self {
            Some(filter) => filter.matches(evt),
            None => Empty.matches(evt),
        }
    }
}

impl Filter for Empty {
    fn matches<E: ToEvent>(&self, _: E) -> bool {
        true
    }
}

impl Filter for fn(&Event<&dyn ErasedProps>) -> bool {
    fn matches<E: ToEvent>(&self, evt: E) -> bool {
        (self)(&evt.to_event().erase())
    }
}

/**
A [`Filter`] from a function.

This type can be created directly, or via [`from_fn`].
*/
pub struct FromFn<F>(F);

impl<F> FromFn<F> {
    /**
    Wrap the given filter function.
    */
    pub const fn new(filter: F) -> FromFn<F> {
        FromFn(filter)
    }
}

impl<F: Fn(&Event<&dyn ErasedProps>) -> bool> Filter for FromFn<F> {
    fn matches<E: ToEvent>(&self, evt: E) -> bool {
        (self.0)(&evt.to_event().erase())
    }
}

/**
Create a [`Filter`] from a function.
*/
pub fn from_fn<F: Fn(&Event<&dyn ErasedProps>) -> bool>(f: F) -> FromFn<F> {
    FromFn(f)
}

/**
An [`Emitter`] protected by a [`Filter`].
*/
pub struct FilteredEmitter<F, E> {
    filter: F,
    emitter: E,
}

impl<F, E> FilteredEmitter<F, E> {
    /**
    Create a new filtered emitter with the given filter `F` and emitter `E`.
    */
    pub const fn new(filter: F, emitter: E) -> Self {
        FilteredEmitter { filter, emitter }
    }
}

impl<F: Filter, E: Emitter> Emitter for FilteredEmitter<F, E> {
    fn emit<T: ToEvent>(&self, evt: T) {
        let evt = evt.to_event();

        if self.filter.matches(&evt) {
            self.emitter.emit(evt);
        }
    }

    fn blocking_flush(&self, timeout: Duration) -> bool {
        self.emitter.blocking_flush(timeout)
    }
}

/**
Wrap an [`Emitter`] in a [`Filter`].

Only events that pass [`Filter::matches`] will be emitted through [`Emitter::emit`].
*/
pub fn wrap<F: Filter, E: Emitter>(filter: F, emitter: E) -> FilteredEmitter<F, E> {
    filter.wrap_emitter(emitter)
}

impl<T: Filter, U: Filter> Filter for And<T, U> {
    fn matches<E: ToEvent>(&self, evt: E) -> bool {
        let evt = evt.to_event();

        self.left().matches(&evt) && self.right().matches(&evt)
    }
}

impl<T: Filter, U: Filter> Filter for Or<T, U> {
    fn matches<E: ToEvent>(&self, evt: E) -> bool {
        let evt = evt.to_event();

        self.left().matches(&evt) || self.right().matches(&evt)
    }
}

/**
A [`Filter`] that always evaluates to `true`.
*/
pub fn always() -> Empty {
    Empty
}

mod internal {
    use crate::{event::Event, props::ErasedProps};

    pub trait DispatchFilter {
        fn dispatch_matches(&self, evt: &Event<&dyn ErasedProps>) -> bool;
    }

    pub trait SealedFilter {
        fn erase_filter(&self) -> crate::internal::Erased<&dyn DispatchFilter>;
    }
}

/**
An object-safe [`Filter`].

A `dyn ErasedFilter` can be treated as `impl Filter`.
*/
pub trait ErasedFilter: internal::SealedFilter {}

impl<T: Filter> ErasedFilter for T {}

impl<T: Filter> internal::SealedFilter for T {
    fn erase_filter(&self) -> crate::internal::Erased<&dyn internal::DispatchFilter> {
        crate::internal::Erased(self)
    }
}

impl<T: Filter> internal::DispatchFilter for T {
    fn dispatch_matches(&self, evt: &Event<&dyn ErasedProps>) -> bool {
        self.matches(evt)
    }
}

impl<'a> Filter for dyn ErasedFilter + 'a {
    fn matches<E: ToEvent>(&self, evt: E) -> bool {
        self.erase_filter()
            .0
            .dispatch_matches(&evt.to_event().erase())
    }
}

impl<'a> Filter for dyn ErasedFilter + Send + Sync + 'a {
    fn matches<E: ToEvent>(&self, evt: E) -> bool {
        (self as &(dyn ErasedFilter + 'a)).matches(evt)
    }
}
