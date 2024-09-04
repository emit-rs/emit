/*!
The [`Event`] type.

Events are the timestamped combination of [`Template`] and [`Props`] that describe what the event was and any ambient state surrounding it.

Events are a generic abstraction. They only directly define the concepts that are common to all diagnostics. Specific kinds of diagnostic events, like logs, traces, or metric samples, are modeled on top of events using [crate::well_known] key-values in their props.

Events can be constructed directly, or generically through the [`ToEvent`] trait.
*/

use core::{fmt, ops::ControlFlow};

use crate::{
    extent::{Extent, ToExtent},
    path::Path,
    props::{ErasedProps, Props},
    template::{Render, Template},
    timestamp::Timestamp,
};

/**
A captured record of something significant that occurred during the operation of a system.
*/
#[derive(Clone)]
pub struct Event<'a, P> {
    // Fields that identify an event callsite
    // "where"
    mdl: Path<'a>,
    // "what"
    tpl: Template<'a>,
    // Fields that vary per instance of an event callsite
    // "when"
    extent: Option<Extent>,
    // "why"
    props: P,
}

impl<'a, P> Event<'a, P> {
    /**
    Construct an event from its parts.

    Events are composed of:

    - `mdl`: A [`Path`] to the module that produced the event. This will typically be a [`module_path!`].
    - `tpl`: The [`Template`] of the event. This is the user-facing description of the event that can be rendered into a readable form.
    - `extent`: The [`Extent`] of the event. This is the point in time at which it occurred, or the timespan for which it was active.
    - `props`: The [`Props`] attached to the event, captured from the surrounding environment.
    */
    pub fn new(
        mdl: impl Into<Path<'a>>,
        tpl: impl Into<Template<'a>>,
        extent: impl ToExtent,
        props: P,
    ) -> Self {
        Event {
            mdl: mdl.into(),
            tpl: tpl.into(),
            extent: extent.to_extent(),
            props,
        }
    }

    /**
    Get a reference to the module that produced the event.
    */
    pub fn mdl(&self) -> &Path<'a> {
        &self.mdl
    }

    /**
    Set the module of the event, returning a new one.
    */
    pub fn with_mdl(mut self, mdl: impl Into<Path<'a>>) -> Self {
        self.mdl = mdl.into();
        self
    }

    /**
    Get a reference to the extent of the event, if there is one.

    An event won't have an extent if it was never constructed with one. This can happen in environments without access to a realtime clock.
    */
    pub fn extent(&self) -> Option<&Extent> {
        self.extent.as_ref()
    }

    /**
    Set the extent of the event, returning a new one.
    */
    pub fn with_extent(mut self, extent: impl ToExtent) -> Self {
        self.extent = extent.to_extent();
        self
    }

    /**
    Get the extent of the event as a point in time.

    If the event has an extent then this method will return `Some`, with the result of [`Extent::as_point`]. If the event doesn't have an extent then this method will return `None`.
    */
    pub fn ts(&self) -> Option<&Timestamp> {
        self.extent.as_ref().map(|extent| extent.as_point())
    }

    /**
    Get the start point of the extent of the event.

    If the event has an extent, and that extent covers a timespan then this method will return `Some`. Otherwise this method will return `None`.
    */
    pub fn ts_start(&self) -> Option<&Timestamp> {
        self.extent
            .as_ref()
            .and_then(|extent| extent.as_span())
            .map(|span| &span.start)
    }

    /**
    Get a reference to the template of the event.
    */
    pub fn tpl(&self) -> &Template<'a> {
        &self.tpl
    }

    /**
    Set the template of the event, returning a new one.
    */
    pub fn with_tpl(mut self, tpl: impl Into<Template<'a>>) -> Self {
        self.tpl = tpl.into();
        self
    }

    /**
    Get a reference to the properties of the event.
    */
    pub fn props(&self) -> &P {
        &self.props
    }

    /**
    Set the properties of the event, returning a new one.
    */
    pub fn with_props<U>(self, props: U) -> Event<'a, U> {
        Event {
            mdl: self.mdl,
            extent: self.extent,
            tpl: self.tpl,
            props,
        }
    }

    /**
    Map the properties of the event, returning a new one.
    */
    pub fn map_props<U>(self, map: impl FnOnce(P) -> U) -> Event<'a, U> {
        Event {
            mdl: self.mdl,
            extent: self.extent,
            tpl: self.tpl,
            props: map(self.props),
        }
    }
}

impl<'a, P: Props> Event<'a, P> {
    /**
    Get a lazily-evaluated formatting of the event's template.
    */
    pub fn msg(&self) -> Render<&P> {
        self.tpl.render(&self.props)
    }

    /**
    Get a new event, borrowing data from this one.
    */
    pub fn by_ref<'b>(&'b self) -> Event<'b, &'b P> {
        Event {
            mdl: self.mdl.by_ref(),
            extent: self.extent.clone(),
            tpl: self.tpl.by_ref(),
            props: &self.props,
        }
    }

    /**
    Get a type-erased event, borrowing data from this one.
    */
    pub fn erase<'b>(&'b self) -> Event<'b, &'b dyn ErasedProps> {
        Event {
            mdl: self.mdl.by_ref(),
            extent: self.extent.clone(),
            tpl: self.tpl.by_ref(),
            props: &self.props,
        }
    }
}

impl<'a, P: Props> fmt::Debug for Event<'a, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct AsDebug<T>(T);

        impl<T: Props> fmt::Debug for AsDebug<T> {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                let mut f = f.debug_map();

                self.0.for_each(|k, v| {
                    f.key(&k.get());
                    f.value(&v);

                    ControlFlow::Continue(())
                });

                f.finish()
            }
        }

        let mut f = f.debug_struct("Event");

        f.field("mdl", &self.mdl);
        f.field("tpl", &self.tpl);
        f.field("extent", &self.extent);
        f.field("props", &AsDebug(&self.props));

        f.finish()
    }
}

/**
Convert a value into an [`Event`].
*/
pub trait ToEvent {
    /**
    The kind of [`Props`] on the resulting value.
    */
    type Props<'a>: Props
    where
        Self: 'a;

    /**
    Perform the conversion.
    */
    fn to_event<'a>(&'a self) -> Event<'a, Self::Props<'a>>;
}

impl<'a, T: ToEvent + ?Sized> ToEvent for &'a T {
    type Props<'b> = T::Props<'b> where Self: 'b;

    fn to_event<'b>(&'b self) -> Event<'b, Self::Props<'b>> {
        (**self).to_event()
    }
}

impl<'a, P: Props> ToEvent for Event<'a, P> {
    type Props<'b> = &'b P where Self: 'b;

    fn to_event<'b>(&'b self) -> Event<'b, Self::Props<'b>> {
        self.by_ref()
    }
}

#[cfg(test)]
mod tests {
    use crate::{str::Str, value::Value};

    use super::*;

    #[test]
    fn event_new() {
        let evt = Event::new(
            Path::new_unchecked("module"),
            Template::literal("An event"),
            Extent::span(Timestamp::MIN..Timestamp::MAX),
            [
                ("a", Value::from(true)),
                ("b", Value::from(1)),
                ("c", Value::from("string")),
            ],
        );

        fn assert(evt: &Event<impl Props>) {
            assert_eq!(Path::new_unchecked("module"), evt.mdl());
            assert_eq!(
                Timestamp::MIN..Timestamp::MAX,
                evt.extent().unwrap().as_span().unwrap().clone()
            );
            assert_eq!("An event", evt.tpl().as_literal().unwrap());

            assert_eq!(true, evt.props().pull::<bool, _>("a").unwrap());
            assert_eq!(1, evt.props().pull::<i32, _>("b").unwrap());
            assert_eq!("string", evt.props().pull::<Str, _>("c").unwrap());
        }

        assert(&evt);
        assert(&evt.by_ref());
        assert(&evt.erase());
    }
}
