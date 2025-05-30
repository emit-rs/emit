/*!
The [`Filter`] type.

Filters reduce the burden of diagnostics by limiting the volume of data generated. A typical filter will only match events with a certain level or higher, or it may exclude all events for a particularly noisy module.
*/

use crate::{
    and::And,
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

impl Filter for fn(Event<&dyn ErasedProps>) -> bool {
    fn matches<E: ToEvent>(&self, evt: E) -> bool {
        (self)(evt.to_event().erase())
    }
}

/**
A [`Filter`] from a function.

This type can be created directly, or via [`from_fn`].
*/
pub struct FromFn<F = fn(Event<&dyn ErasedProps>) -> bool>(F);

impl<F> FromFn<F> {
    /**
    Wrap the given filter function.
    */
    pub const fn new(filter: F) -> FromFn<F> {
        FromFn(filter)
    }
}

impl<F: Fn(Event<&dyn ErasedProps>) -> bool> Filter for FromFn<F> {
    fn matches<E: ToEvent>(&self, evt: E) -> bool {
        (self.0)(evt.to_event().erase())
    }
}

/**
Create a [`Filter`] from a function.
*/
pub const fn from_fn<F: Fn(Event<&dyn ErasedProps>) -> bool>(f: F) -> FromFn<F> {
    FromFn(f)
}

/**
A [`Filter`] that always matches any event.
*/
pub struct Always {}

impl Always {
    /**
    Create a filter that matches any event.
    */
    pub const fn new() -> Always {
        Always {}
    }
}

impl Filter for Always {
    fn matches<E: ToEvent>(&self, _: E) -> bool {
        true
    }
}

/**
Create a [`Filter`] that matches any event.
*/
pub const fn always() -> Always {
    Always::new()
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

#[cfg(test)]
mod tests {
    use crate::{path::Path, template::Template};

    use super::*;

    struct MyFilter {
        matches: bool,
    }

    impl Filter for MyFilter {
        fn matches<E: ToEvent>(&self, _: E) -> bool {
            self.matches
        }
    }

    #[test]
    fn option_filter() {
        for (case, matches) in [(Some(MyFilter { matches: false }), false), (None, true)] {
            assert_eq!(
                matches,
                case.matches(Event::new(
                    Path::new_raw("module"),
                    Template::literal("Event"),
                    Empty,
                    Empty,
                ))
            );
        }
    }

    #[test]
    fn and_filter() {
        for a in [true, false] {
            for b in [true, false] {
                let fa = MyFilter { matches: a };
                let fb = MyFilter { matches: b };

                assert_eq!(
                    a && b,
                    fa.and_when(fb).matches(Event::new(
                        Path::new_raw("module"),
                        Template::literal("Event"),
                        Empty,
                        Empty,
                    ))
                );
            }
        }
    }

    #[test]
    fn or_filter() {
        for a in [true, false] {
            for b in [true, false] {
                let fa = MyFilter { matches: a };
                let fb = MyFilter { matches: b };

                assert_eq!(
                    a || b,
                    fa.or_when(fb).matches(Event::new(
                        Path::new_raw("module"),
                        Template::literal("Event"),
                        Empty,
                        Empty,
                    ))
                );
            }
        }
    }

    #[test]
    fn from_fn_filter() {
        let f = from_fn(|evt| evt.mdl() == Path::new_raw("module"));

        assert!(f.matches(Event::new(
            Path::new_raw("module"),
            Template::literal("Event"),
            Empty,
            Empty,
        )));

        assert!(!f.matches(Event::new(
            Path::new_raw("not_module"),
            Template::literal("Event"),
            Empty,
            Empty,
        )));
    }

    #[test]
    fn always_filter() {
        let f = always();

        assert!(f.matches(Event::new(
            Path::new_raw("module"),
            Template::literal("Event"),
            Empty,
            Empty,
        )));
    }

    #[test]
    fn erased_filter() {
        let f = MyFilter { matches: true };
        let f = &f as &dyn ErasedFilter;

        assert!(f.matches(Event::new(
            Path::new_raw("module"),
            Template::literal("Event"),
            Empty,
            Empty,
        )));

        let f = MyFilter { matches: false };
        let f = &f as &dyn ErasedFilter;

        assert!(!f.matches(Event::new(
            Path::new_raw("module"),
            Template::literal("Event"),
            Empty,
            Empty,
        )));
    }
}
