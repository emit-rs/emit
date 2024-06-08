/*!
The [`Kind`] type.
*/

use core::{fmt, str::FromStr};

use emit_core::{
    event::ToEvent,
    filter::Filter,
    props::Props,
    value::{FromValue, ToValue, Value},
    well_known::{EVENT_KIND_METRIC, EVENT_KIND_SPAN, KEY_EVENT_KIND},
};

/**
A kind of specialized diagnostic event.

If a [`crate::Event`] has a kind associated with it, it can be pulled from its props:

```
# use emit::{Event, Props};
# fn with_event(evt: impl emit::event::ToEvent) {
# let evt = evt.to_event();
match evt.props().pull::<emit::Kind, _>(emit::well_known::KEY_EVENT_KIND) {
    Some(emit::Kind::Span) => {
        // The event is a span
    }
    Some(emit::Kind::Metric) => {
        // The event is a metric
    }
    Some(_) => {
        // The event is an unknown kind
    }
    None => {
        // The event doesn't have a specific kind
    }
}
# }
```
*/
#[non_exhaustive]
#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub enum Kind {
    /**
    The event is a span in a distributed trace.

    This variant is equal to [`EVENT_KIND_SPAN`]. See the [`mod@crate::span`] module for details.
    */
    Span,
    /**
    The event is a metric sample.

    This variant is equal to [`EVENT_KIND_METRIC`]. See the [`mod@crate::metric`] module for details.
    */
    Metric,
}

impl fmt::Debug for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"{}\"", self)
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Kind::Span => f.write_str(EVENT_KIND_SPAN),
            Kind::Metric => f.write_str(EVENT_KIND_METRIC),
        }
    }
}

impl ToValue for Kind {
    fn to_value(&self) -> Value {
        Value::capture_display(self)
    }
}

impl<'v> FromValue<'v> for Kind {
    fn from_value(value: Value<'v>) -> Option<Self> {
        value
            .downcast_ref::<Kind>()
            .copied()
            .or_else(|| value.parse())
    }
}

impl FromStr for Kind {
    type Err = ParseKindError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case(EVENT_KIND_SPAN) {
            return Ok(Kind::Span);
        }

        if s.eq_ignore_ascii_case(EVENT_KIND_METRIC) {
            return Ok(Kind::Metric);
        }

        Err(ParseKindError {})
    }
}

/**
An error attempting to parse a [`Kind`] from text.
*/
#[derive(Debug)]
pub struct ParseKindError {}

impl fmt::Display for ParseKindError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the input was not a valid kind")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ParseKindError {}

/**
A [`Filter`] that matches events with a specific [`Kind`].

The kind to match is pulled from the [`KEY_EVENT_KIND`] well-known property. Events that don't carry any kind are not matched.
*/
pub struct KindFilter(Kind);

impl KindFilter {
    /**
    Create a filter that only matches events carrying the same kind.
    */
    pub fn new(kind: Kind) -> Self {
        KindFilter(kind)
    }
}

/**
Only match events that are spans in a distributed trace.

Events that match must carry a [`Kind::Span`].
*/
pub fn is_span_filter() -> KindFilter {
    KindFilter::new(Kind::Span)
}

/**
Only match events that are metric samples.

Events that match must carry a [`Kind::Metric`].
*/
pub fn is_metric_filter() -> KindFilter {
    KindFilter::new(Kind::Metric)
}

impl Filter for KindFilter {
    fn matches<E: ToEvent>(&self, evt: E) -> bool {
        evt.to_event().props().pull::<Kind, _>(KEY_EVENT_KIND) == Some(self.0)
    }
}
