/*!
The [`Span`] type.
*/

/*
Parts of this file are adapted from other libraries:

uuid:
https://github.com/uuid-rs/uuid/blob/main/src/parser.rs
Licensed under Apache 2.0
*/

use emit_core::{
    clock::Clock,
    ctxt::Ctxt,
    event::{Event, ToEvent},
    extent::{Extent, ToExtent},
    path::Path,
    props::Props,
    rng::Rng,
    str::{Str, ToStr},
    template::{self, Template},
    timestamp::Timestamp,
    value::FromValue,
    well_known::{KEY_EVT_KIND, KEY_SPAN_ID, KEY_SPAN_NAME, KEY_SPAN_PARENT, KEY_TRACE_ID},
};

use crate::{
    kind::Kind,
    value::{ToValue, Value},
    Frame, Timer,
};
use core::{
    fmt,
    num::{NonZeroU128, NonZeroU64},
    ops::ControlFlow,
    str::{self, FromStr},
};

/**
A [W3C Trace Id](https://www.w3.org/TR/trace-context/#trace-id).
*/
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct TraceId(NonZeroU128);

impl fmt::Debug for TraceId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(str::from_utf8(&self.to_hex()).unwrap(), f)
    }
}

impl fmt::Display for TraceId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(str::from_utf8(&self.to_hex()).unwrap())
    }
}

impl FromStr for TraceId {
    type Err = ParseIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from_hex_slice(s.as_bytes())
    }
}

impl ToValue for TraceId {
    fn to_value(&self) -> Value {
        Value::capture_display(self)
    }
}

impl<'v> FromValue<'v> for TraceId {
    fn from_value(value: Value<'v>) -> Option<Self> {
        value
            .downcast_ref::<TraceId>()
            .copied()
            .or_else(|| TraceId::try_from_hex(value).ok())
    }
}

impl TraceId {
    /**
    Create a random trace id.

    This method will return `None` if the given [`Rng`] fails to produce a random value, or if it produces the value `0`.
    */
    pub fn random<R: Rng>(rng: R) -> Option<Self> {
        Some(TraceId::new(NonZeroU128::new(rng.gen_u128()?)?))
    }

    /**
    Create a trace id from a non-zero integer.
    */
    pub const fn new(v: NonZeroU128) -> Self {
        TraceId(v)
    }

    /**
    Try create a trace id from an integer.

    This method will return `None` if `v` is `0`.
    */
    pub fn from_u128(v: u128) -> Option<Self> {
        Some(TraceId(NonZeroU128::new(v)?))
    }

    /**
    Get the value of the trace id as an integer.
    */
    pub const fn to_u128(&self) -> u128 {
        self.0.get()
    }

    /**
    Get a trace id from a 16 byte big-endian array.
    */
    pub fn from_bytes(v: [u8; 16]) -> Option<Self> {
        Self::from_u128(u128::from_be_bytes(v))
    }

    /**
    Convert the trace id into a 16 byte big-endian array.
    */
    pub fn to_bytes(&self) -> [u8; 16] {
        self.0.get().to_be_bytes()
    }

    /**
    Convert the trace id into a 32 byte ASCII-compatible hex string, like `4bf92f3577b34da6a3ce929d0e0e4736`.
    */
    pub fn to_hex(&self) -> [u8; 32] {
        let mut dst = [0; 32];
        let src: [u8; 16] = self.0.get().to_be_bytes();

        for i in 0..src.len() {
            let b = src[i];

            dst[i * 2] = HEX_ENCODE_TABLE[(b >> 4) as usize];
            dst[i * 2 + 1] = HEX_ENCODE_TABLE[(b & 0x0f) as usize];
        }

        dst
    }

    /**
    Try parse a slice of ASCII hex bytes into a trace id.

    If `hex` is not a 32 byte array of valid hex characters (`[a-fA-F0-9]`) then this method will fail.
    */
    pub fn try_from_hex_slice(hex: &[u8]) -> Result<Self, ParseIdError> {
        let hex: &[u8; 32] = hex.try_into().map_err(|_| ParseIdError {})?;

        let mut dst = [0; 16];

        let mut i = 0;
        while i < 16 {
            // Convert a two-char hex value (like `A8`)
            // into a byte (like `10101000`)
            let h1 = HEX_DECODE_TABLE[hex[i * 2] as usize];
            let h2 = HEX_DECODE_TABLE[hex[i * 2 + 1] as usize];

            // We use `0xff` as a sentinel value to indicate
            // an invalid hex character sequence (like the letter `G`)
            if h1 | h2 == 0xff {
                return Err(ParseIdError {});
            }

            // The upper nibble needs to be shifted into position
            // to produce the final byte value
            dst[i] = SHL4_TABLE[h1 as usize] | h2;
            i += 1;
        }

        Ok(TraceId::new(
            NonZeroU128::new(u128::from_be_bytes(dst)).ok_or_else(|| ParseIdError {})?,
        ))
    }

    /**
    Try parse ASCII hex characters into a trace id.

    If `hex` is not exactly 32 valid hex characters (`[a-fA-F0-9]`) then this method will fail.
    */
    pub fn try_from_hex(hex: impl fmt::Display) -> Result<Self, ParseIdError> {
        let mut buf = Buffer::<32>::new();

        Self::try_from_hex_slice(buf.buffer(hex)?)
    }
}

/**
A [W3C Span Id](https://www.w3.org/TR/trace-context/#parent-id).
*/
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpanId(NonZeroU64);

impl fmt::Debug for SpanId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(str::from_utf8(&self.to_hex()).unwrap(), f)
    }
}

impl fmt::Display for SpanId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(str::from_utf8(&self.to_hex()).unwrap())
    }
}

impl FromStr for SpanId {
    type Err = ParseIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from_hex_slice(s.as_bytes())
    }
}

impl ToValue for SpanId {
    fn to_value(&self) -> Value {
        Value::capture_display(self)
    }
}

impl<'v> FromValue<'v> for SpanId {
    fn from_value(value: Value<'v>) -> Option<Self> {
        value
            .downcast_ref::<SpanId>()
            .copied()
            .or_else(|| SpanId::try_from_hex(value).ok())
    }
}

impl SpanId {
    /**
    Create a new random span id.

    This method will return `None` if the given [`Rng`] fails to produce a random value, or if it produces the value `0`.
    */
    pub fn random<R: Rng>(rng: R) -> Option<Self> {
        Some(SpanId::new(NonZeroU64::new(rng.gen_u64()?)?))
    }

    /**
    Create a span id from a non-zero integer.
    */
    pub const fn new(v: NonZeroU64) -> Self {
        SpanId(v)
    }

    /**
    Create a span id from an integer.

    This method will return `None` if `v` is `0`.
    */
    pub fn from_u64(v: u64) -> Option<Self> {
        Some(SpanId(NonZeroU64::new(v)?))
    }

    /**
    Get the value of the span id as an integer.
    */
    pub const fn to_u64(&self) -> u64 {
        self.0.get()
    }

    /**
    Get a span id from an 8 byte big-endian array.
    */
    pub fn from_bytes(v: [u8; 8]) -> Option<Self> {
        Self::from_u64(u64::from_be_bytes(v))
    }

    /**
    Convert the span id into an 8 byte big-endian array.
    */
    pub fn to_bytes(&self) -> [u8; 8] {
        self.0.get().to_be_bytes()
    }

    /**
    Convert the span id into a 16 byte ASCII-compatible hex string, like `00f067aa0ba902b7`.
    */
    pub fn to_hex(&self) -> [u8; 16] {
        let mut dst = [0; 16];
        let src: [u8; 8] = self.0.get().to_be_bytes();

        for i in 0..src.len() {
            let b = src[i];

            dst[i * 2] = HEX_ENCODE_TABLE[(b >> 4) as usize];
            dst[i * 2 + 1] = HEX_ENCODE_TABLE[(b & 0x0f) as usize];
        }

        dst
    }

    /**
    Try parse a slice of ASCII hex bytes into a span id.

    If `hex` is not a 16 byte array of valid hex characters (`[a-fA-F0-9]`) then this method will fail.
    */
    pub fn try_from_hex_slice(hex: &[u8]) -> Result<Self, ParseIdError> {
        let hex: &[u8; 16] = hex.try_into().map_err(|_| ParseIdError {})?;

        let mut dst = [0; 8];

        let mut i = 0;
        while i < 8 {
            // Convert a two-char hex value (like `A8`)
            // into a byte (like `10101000`)
            let h1 = HEX_DECODE_TABLE[hex[i * 2] as usize];
            let h2 = HEX_DECODE_TABLE[hex[i * 2 + 1] as usize];

            // We use `0xff` as a sentinel value to indicate
            // an invalid hex character sequence (like the letter `G`)
            if h1 | h2 == 0xff {
                return Err(ParseIdError {});
            }

            // The upper nibble needs to be shifted into position
            // to produce the final byte value
            dst[i] = SHL4_TABLE[h1 as usize] | h2;
            i += 1;
        }

        Ok(SpanId::new(
            NonZeroU64::new(u64::from_be_bytes(dst)).ok_or_else(|| ParseIdError {})?,
        ))
    }

    /**
    Try parse ASCII hex characters into a span id.

    If `hex` is not exactly 16 valid hex characters (`[a-fA-F0-9]`) then this method will fail.
    */
    pub fn try_from_hex(hex: impl fmt::Display) -> Result<Self, ParseIdError> {
        let mut buf = Buffer::<16>::new();

        Self::try_from_hex_slice(buf.buffer(hex)?)
    }
}

/*
Original implementation: https://github.com/uuid-rs/uuid/blob/main/src/parser.rs

Licensed under Apache 2.0
*/

const HEX_ENCODE_TABLE: [u8; 16] = [
    b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'a', b'b', b'c', b'd', b'e', b'f',
];

const HEX_DECODE_TABLE: &[u8; 256] = &{
    let mut buf = [0; 256];
    let mut i: u8 = 0;

    loop {
        buf[i as usize] = match i {
            b'0'..=b'9' => i - b'0',
            b'a'..=b'f' => i - b'a' + 10,
            b'A'..=b'F' => i - b'A' + 10,
            _ => 0xff,
        };

        if i == 255 {
            break buf;
        }

        i += 1
    }
};

const SHL4_TABLE: &[u8; 256] = &{
    let mut buf = [0; 256];
    let mut i: u8 = 0;

    loop {
        buf[i as usize] = i.wrapping_shl(4);

        if i == 255 {
            break buf;
        }

        i += 1;
    }
};

/**
An error encountered attempting to parse a [`TraceId`] or [`SpanId`].
*/
#[derive(Debug)]
pub struct ParseIdError {}

impl fmt::Display for ParseIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the input was not a valid id")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ParseIdError {}

struct Buffer<const N: usize> {
    value: [u8; N],
    idx: usize,
}

impl<const N: usize> Buffer<N> {
    fn new() -> Self {
        Buffer {
            value: [0; N],
            idx: 0,
        }
    }

    fn buffer(&mut self, value: impl fmt::Display) -> Result<&[u8], ParseIdError> {
        use fmt::Write as _;

        self.idx = 0;

        write!(self, "{}", value).map_err(|_| ParseIdError {})?;

        Ok(&self.value[..self.idx])
    }
}

impl<const N: usize> fmt::Write for Buffer<N> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let s = s.as_bytes();
        let next_idx = self.idx + s.len();

        if next_idx <= self.value.len() {
            self.value[self.idx..next_idx].copy_from_slice(s);
            self.idx = next_idx;

            Ok(())
        } else {
            Err(fmt::Error)
        }
    }
}

/**
An active span in a distributed trace.

This type is created by the [`macro@crate::span!`] macro with the `guard` control parameter. See the [`mod@crate::span`] module for details on creating spans.

Call [`SpanGuard::complete_with`], or just drop the guard to complete it, emitting a [`Span`] for its execution.
*/
pub struct SpanGuard<'a, C: Clock, P: Props, F: FnOnce(Span<'a, P>)> {
    state: Option<SpanGuardState<'a, C, P>>,
    on_drop: Option<F>,
}

struct SpanGuardState<'a, C: Clock, P: Props> {
    mdl: Path<'a>,
    timer: Timer<C>,
    name: Str<'a>,
    ctxt: SpanCtxt,
    props: P,
}

impl<'a, C: Clock, P: Props> SpanGuardState<'a, C, P> {
    fn complete(self) -> Span<'a, P> {
        Span::new(self.mdl, self.name, self.timer, self.props)
    }
}

/**
A diagnostic event that represents a span in a distributed trace.

Spans are an extension of [`Event`]s that explicitly take the well-known properties that signal an event as being a span. See the [`mod@crate::span`] module for details.

A `SpanEvent` can be converted into an [`Event`] through its [`ToEvent`] implemenation, or passed directly to a [`crate::Emitter`] to emit it.
*/
pub struct Span<'a, P> {
    mdl: Path<'a>,
    name: Str<'a>,
    extent: Option<Extent>,
    props: P,
}

impl<'a, P: Props> Span<'a, P> {
    /**
    Create a new span event from its parts.

    Each span consists of:

    - `mdl`: The module that executed the operation the span is tracking.
    - `name`: The name of the operation the span is tracking.
    - `extent`: The time the operation spent executing. The extent should be a span.
    - `props`: Additional [`Props`] to associate with the span. These may include the [`SpanCtxt`] with the trace and span ids for the span, or they may be part of the ambient context.
    */
    pub fn new(
        mdl: impl Into<Path<'a>>,
        name: impl Into<Str<'a>>,
        extent: impl ToExtent,
        props: P,
    ) -> Self {
        Span {
            mdl: mdl.into(),
            extent: extent.to_extent(),
            name: name.into(),
            props,
        }
    }

    /**
    Get the module that executed the operation.
    */
    pub fn mdl(&self) -> &Path<'a> {
        &self.mdl
    }

    /**
    Get the name of the operation.
    */
    pub fn name(&self) -> &Str<'a> {
        &self.name
    }

    /**
    Get the time the operation spent executing.
    */
    pub fn extent(&self) -> Option<&Extent> {
        self.extent.as_ref()
    }

    /**
    Get the extent of the metric as a point in time.

    If the span has an extent then this method will return `Some`, with the result of [`Extent::as_point`]. If the span doesn't have an extent then this method will return `None`.
    */
    pub fn ts(&self) -> Option<&Timestamp> {
        self.extent.as_ref().map(|extent| extent.as_point())
    }

    /**
    Get the start point of the extent of the span.

    If the span has an extent, and that extent covers a timespan then this method will return `Some`. Otherwise this method will return `None`.
    */
    pub fn ts_start(&self) -> Option<&Timestamp> {
        self.extent
            .as_ref()
            .and_then(|extent| extent.as_span())
            .map(|span| &span.start)
    }

    /**
    Get the additional properties associated with the span.
    */
    pub fn props(&self) -> &P {
        &self.props
    }
}

impl<'a, P: Props> ToEvent for Span<'a, P> {
    type Props<'b> = &'b Self where Self: 'b;

    fn to_event<'b>(&'b self) -> Event<'b, Self::Props<'b>> {
        // "{span_name} completed"
        const TEMPLATE: &'static [template::Part<'static>] = &[
            template::Part::hole("span_name"),
            template::Part::text(" completed"),
        ];

        Event::new(
            self.mdl.by_ref(),
            Template::new(TEMPLATE),
            self.extent.clone(),
            &self,
        )
    }
}

impl<'a, P: Props> ToExtent for Span<'a, P> {
    fn to_extent(&self) -> Option<Extent> {
        self.extent().cloned()
    }
}

impl<'a, P: Props> Props for Span<'a, P> {
    fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        mut for_each: F,
    ) -> ControlFlow<()> {
        for_each(KEY_EVT_KIND.to_str(), Kind::Span.to_value())?;
        for_each(KEY_SPAN_NAME.to_str(), self.name.to_value())?;

        self.props.for_each(&mut for_each)
    }
}

/**
The trace id, span id, and parent parent span id of a span.

These ids can be used to identify the distributed trace a span belongs to, and to identify the span itself within that trace.

The `SpanCtxt` for the currently executing span can be pulled from the ambient context with [`SpanCtxt::current`]. Once a `SpanCtxt` is constructed, a new child context can be generated by [`SpanCtxt::new_child`].

`SpanCtxt` should be pushed onto the ambient context with [`SpanCtxt::push`] so any events emitted during its execution are correlated to it.
*/
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpanCtxt {
    trace_id: Option<TraceId>,
    span_parent: Option<SpanId>,
    span_id: Option<SpanId>,
}

impl SpanCtxt {
    /**
    Create the context from a set of identifiers.

    The `trace_id` and `span_id` should both be `Some`, but `span_parent` may be `None` if the span is at the root of the distributed trace.

    If `trace_id` or `span_id` are `None` then the context is invalid, but can still be used.
    */
    pub const fn new(
        trace_id: Option<TraceId>,
        span_parent: Option<SpanId>,
        span_id: Option<SpanId>,
    ) -> Self {
        SpanCtxt {
            trace_id,
            span_parent,
            span_id,
        }
    }

    /**
    Create a context where all identifiers are `None`.
    */
    pub const fn empty() -> Self {
        Self {
            trace_id: None,
            span_parent: None,
            span_id: None,
        }
    }

    /**
    Generate a new context.
    */
    pub fn new_root(rng: impl Rng) -> Self {
        let trace_id = TraceId::random(&rng);
        let span_parent = None;
        let span_id = SpanId::random(&rng);

        SpanCtxt::new(trace_id, span_parent, span_id)
    }

    /**
    Read the current context from an ambient [`Ctxt`].

    This method will pull the [`TraceId`] from [`KEY_TRACE_ID`], the `SpanId` from [`KEY_SPAN_ID`], and the parent [`SpanId`] from [`KEY_SPAN_PARENT`].
    */
    pub fn current(ctxt: impl Ctxt) -> Self {
        ctxt.with_current(|current| {
            SpanCtxt::new(
                current.pull::<TraceId, _>(KEY_TRACE_ID),
                current.pull::<SpanId, _>(KEY_SPAN_PARENT),
                current.pull::<SpanId, _>(KEY_SPAN_ID),
            )
        })
    }

    /**
    Generate a new context that is a child of `self`.

    The new context will share the same trace id as `self`, use the span id of `self` as its parent span id, and generate a new random span id as its own through [`SpanId::random`].

    If [`Self::trace_id`] is `None` then a new trace id will be generated through [`TraceId::random`].
    */
    pub fn new_child(&self, rng: impl Rng) -> Self {
        let trace_id = self.trace_id.or_else(|| TraceId::random(&rng));
        let span_parent = self.span_id;
        let span_id = SpanId::random(&rng);

        SpanCtxt::new(trace_id, span_parent, span_id)
    }

    /**
    Get the trace id for the span.
    */
    pub fn trace_id(&self) -> Option<&TraceId> {
        self.trace_id.as_ref()
    }

    /**
    Get the parent of the span.
    */
    pub fn span_parent(&self) -> Option<&SpanId> {
        self.span_parent.as_ref()
    }

    /**
    Get the id of the span.
    */
    pub fn span_id(&self) -> Option<&SpanId> {
        self.span_id.as_ref()
    }

    /**
    Push the [`SpanCtxt`] onto the ambient context.

    The trace id, span id, and parent span id will be pushed to the context. This ensures diagnostics emitted during the execution of this span are properly linked to it.
    */
    pub fn push<T: Ctxt>(&self, ctxt: T) -> Frame<T> {
        Frame::push(ctxt, self)
    }
}

impl Props for SpanCtxt {
    fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        mut for_each: F,
    ) -> ControlFlow<()> {
        if let Some(ref trace_id) = self.trace_id {
            for_each(KEY_TRACE_ID.to_str(), trace_id.to_value())?;
        }

        if let Some(ref span_id) = self.span_id {
            for_each(KEY_SPAN_ID.to_str(), span_id.to_value())?;
        }

        if let Some(ref span_parent) = self.span_parent {
            for_each(KEY_SPAN_PARENT.to_str(), span_parent.to_value())?;
        }

        ControlFlow::Continue(())
    }
}

impl<'a, C: Clock, P: Props, F: FnOnce(Span<'a, P>)> Drop for SpanGuard<'a, C, P, F> {
    fn drop(&mut self) {
        if let (Some(value), Some(on_drop)) = (self.state.take(), self.on_drop.take()) {
            on_drop(value.complete())
        }
    }
}

impl<'a, C: Clock, P: Props, F: FnOnce(Span<'a, P>)> SpanGuard<'a, C, P, F> {
    pub(crate) fn filtered_new(
        filter: impl FnOnce(&SpanCtxt, Span<&P>) -> bool,
        mdl: impl Into<Path<'a>>,
        timer: Timer<C>,
        name: impl Into<Str<'a>>,
        ctxt: SpanCtxt,
        event_props: P,
        default_complete: F,
    ) -> Self {
        let mdl = mdl.into();
        let name = name.into();

        if filter(
            &ctxt,
            Span::new(
                mdl.by_ref(),
                name.by_ref(),
                timer.start_timestamp(),
                &event_props,
            ),
        ) {
            SpanGuard {
                state: Some(SpanGuardState {
                    timer,
                    mdl,
                    ctxt,
                    name,
                    props: event_props,
                }),
                on_drop: Some(default_complete),
            }
        } else {
            Self::disabled()
        }
    }

    pub(crate) fn disabled() -> Self {
        SpanGuard {
            state: None,
            on_drop: None,
        }
    }

    pub(crate) fn push_ctxt<T: Ctxt>(
        &mut self,
        ctxt: T,
        ctxt_props: impl Props,
    ) -> Frame<Option<T>> {
        if self.is_enabled() {
            Frame::push(
                Some(ctxt),
                self.state
                    .as_ref()
                    .map(|state| state.ctxt)
                    .and_props(ctxt_props),
            )
        } else {
            Frame::current(None)
        }
    }

    /**
    Whether the span will emit an event on completion.
    */
    pub fn is_enabled(&self) -> bool {
        self.state.is_some()
    }

    /**
    Complete the span.

    If the span is disabled then this method is a no-op.
    */
    pub fn complete(self) {
        drop(self);
    }

    /**
    Complete the span with the given closure.

    If the span is disabled then the `complete` closure won't be called.
    */
    pub fn complete_with(mut self, complete: impl FnOnce(Span<'a, P>)) -> bool {
        if let Some(value) = self.state.take() {
            complete(value.complete());
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::time::Duration;

    #[cfg(all(feature = "std", feature = "rand"))]
    use std::cell::Cell;

    use crate::Timestamp;

    #[test]
    fn span_id_parse() {
        for (case, expected) in [
            (
                "0123456789abcdef",
                Ok(SpanId::from_u64(0x0123456789abcdef).unwrap()),
            ),
            (
                "0000000000000001",
                Ok(SpanId::from_u64(0x0000000000000001).unwrap()),
            ),
            ("0000000000000000", Err(ParseIdError {})),
            ("0x00000000000001", Err(ParseIdError {})),
            ("0x0000000000000001", Err(ParseIdError {})),
            ("1", Err(ParseIdError {})),
            ("", Err::<SpanId, ParseIdError>(ParseIdError {})),
        ] {
            match expected {
                Ok(expected) => {
                    assert_eq!(expected, SpanId::try_from_hex(case).unwrap());
                    assert_eq!(expected, SpanId::try_from_hex(case).unwrap());
                }
                Err(e) => assert_eq!(
                    e.to_string(),
                    SpanId::try_from_hex(case).unwrap_err().to_string()
                ),
            }
        }
    }

    #[test]
    fn trace_id_parse() {
        for (case, expected) in [
            (
                "0123456789abcdef0123456789abcdef",
                Ok(TraceId::from_u128(0x0123456789abcdef0123456789abcdef).unwrap()),
            ),
            (
                "00000000000000000000000000000001",
                Ok(TraceId::from_u128(0x00000000000000000000000000000001).unwrap()),
            ),
            ("00000000000000000000000000000000", Err(ParseIdError {})),
            ("0x000000000000000000000000000001", Err(ParseIdError {})),
            ("0x00000000000000000000000000000001", Err(ParseIdError {})),
            ("1", Err(ParseIdError {})),
            ("", Err::<TraceId, ParseIdError>(ParseIdError {})),
        ] {
            match expected {
                Ok(expected) => assert_eq!(expected, TraceId::try_from_hex(case).unwrap()),
                Err(e) => assert_eq!(
                    e.to_string(),
                    TraceId::try_from_hex(case).unwrap_err().to_string()
                ),
            }
        }
    }

    #[test]
    fn span_id_fmt() {
        for (case, expected) in [
            (SpanId::from_u64(1).unwrap(), "0000000000000001"),
            (
                SpanId::from_u64(0x0123456789abcdef).unwrap(),
                "0123456789abcdef",
            ),
        ] {
            assert_eq!(expected, case.to_string());
            assert_eq!(expected, str::from_utf8(&case.to_hex()).unwrap());
        }
    }

    #[test]
    fn trace_id_fmt() {
        for (case, expected) in [
            (
                TraceId::from_u128(1).unwrap(),
                "00000000000000000000000000000001",
            ),
            (
                TraceId::from_u128(0x0123456789abcdef0123456789abcdef).unwrap(),
                "0123456789abcdef0123456789abcdef",
            ),
        ] {
            assert_eq!(expected, case.to_string());
            assert_eq!(expected, str::from_utf8(&case.to_hex()).unwrap());
        }
    }

    #[test]
    fn span_id_roundtrip() {
        let id = SpanId::new(NonZeroU64::new(u64::MAX / 2).unwrap());

        let fmt = id.to_string();

        let parsed: SpanId = fmt.parse().unwrap();

        assert_eq!(id, parsed, "{}", fmt);
    }

    #[test]
    fn trace_id_roundtrip() {
        let id = TraceId::new(NonZeroU128::new(u128::MAX / 2).unwrap());

        let fmt = id.to_string();

        let parsed: TraceId = fmt.parse().unwrap();

        assert_eq!(id, parsed, "{}", fmt);
    }

    #[test]
    fn span_id_random_empty() {
        assert!(SpanId::random(crate::Empty).is_none());
    }

    #[test]
    #[cfg(feature = "rand")]
    fn span_id_random_rand() {
        assert!(SpanId::random(crate::platform::rand_rng::RandRng::new()).is_some());
    }

    #[test]
    fn trace_id_random_empty() {
        assert!(TraceId::random(crate::Empty).is_none());
    }

    #[test]
    #[cfg(feature = "rand")]
    fn trace_id_random_rand() {
        assert!(TraceId::random(crate::platform::rand_rng::RandRng::new()).is_some());
    }

    #[test]
    fn span_id_to_from_value() {
        let id = SpanId::from_u64(u64::MAX / 2).unwrap();

        assert_eq!(id, SpanId::from_value(id.to_value()).unwrap());
    }

    #[test]
    fn span_id_from_value_string() {
        assert_eq!(
            SpanId::from_u64(0x0123456789abcdef).unwrap(),
            Value::from("0123456789abcdef").cast().unwrap()
        );
    }

    #[test]
    fn trace_id_to_from_value() {
        let id = TraceId::from_u128(u128::MAX / 2).unwrap();

        assert_eq!(id, TraceId::from_value(id.to_value()).unwrap());
    }

    #[test]
    fn trace_id_from_value_string() {
        assert_eq!(
            TraceId::from_u128(0x0123456789abcdef0123456789abcdef).unwrap(),
            Value::from("0123456789abcdef0123456789abcdef")
                .cast()
                .unwrap()
        );
    }

    #[test]
    #[cfg(all(feature = "std", feature = "rand"))]
    fn span_ctxt_new() {
        let rng = crate::platform::rand_rng::RandRng::new();
        let ctxt = crate::platform::thread_local_ctxt::ThreadLocalCtxt::new();

        // Span context from an empty source is empty
        let root = SpanCtxt::current(&ctxt);
        assert_eq!(SpanCtxt::empty(), root);

        // New root context has a new trace id and span id, but no parent
        let root = SpanCtxt::new_root(&rng);

        assert!(root.span_id.is_some());
        assert!(root.trace_id.is_some());
        assert!(root.span_parent.is_none());

        // Push the span context onto the source
        let mut frame = ctxt.open_push(root);

        ctxt.enter(&mut frame);

        // Span context from a non-empty source is the last pushed
        let current = SpanCtxt::current(&ctxt);
        assert_eq!(root, current);
        let root = current;

        // A child span shares the same trace id, but has a new span id
        // The span id of the parent becomes the span parent
        let child = SpanCtxt::new_child(&root, &rng);

        assert_eq!(root.trace_id, child.trace_id);
        assert_ne!(root.span_id, child.span_id);
        assert!(child.span_id.is_some());
        assert_eq!(root.span_id, child.span_parent);

        ctxt.exit(&mut frame);
        ctxt.close(frame);
    }

    #[test]
    fn span_new() {
        let span = Span::new(
            Path::new_unchecked("test"),
            "my span",
            Timestamp::from_unix(Duration::from_secs(1)),
            ("span_prop", true),
        );

        assert_eq!("test", span.mdl());
        assert_eq!(
            Timestamp::from_unix(Duration::from_secs(1)).unwrap(),
            span.extent().unwrap().as_point()
        );
        assert_eq!("my span", span.name());
        assert_eq!(true, span.props().pull::<bool, _>("span_prop").unwrap());
    }

    #[test]
    fn span_to_event() {
        let span = Span::new(
            Path::new_unchecked("test"),
            "my span",
            Timestamp::from_unix(Duration::from_secs(1)),
            ("span_prop", true),
        );

        let evt = span.to_event();

        assert_eq!("test", evt.mdl());
        assert_eq!(
            Timestamp::from_unix(Duration::from_secs(1)).unwrap(),
            evt.extent().unwrap().as_point()
        );
        assert_eq!("my span completed", evt.msg().to_string());
        assert_eq!(
            "my span",
            evt.props().pull::<Str, _>(KEY_SPAN_NAME).unwrap()
        );
        assert_eq!(true, evt.props().pull::<bool, _>("span_prop").unwrap());
        assert_eq!(
            Kind::Span,
            evt.props().pull::<Kind, _>(KEY_EVT_KIND).unwrap()
        );
    }

    #[test]
    fn span_to_extent() {
        for (case, expected) in [
            (
                Some(Timestamp::from_unix(Duration::from_secs(1)).unwrap()),
                Some(Extent::point(
                    Timestamp::from_unix(Duration::from_secs(1)).unwrap(),
                )),
            ),
            (None, None),
        ] {
            let span = Span::new(
                Path::new_unchecked("test"),
                "my span",
                case,
                ("span_prop", true),
            );

            let extent = span.to_extent();

            assert_eq!(
                expected.map(|extent| extent.as_range().clone()),
                extent.map(|extent| extent.as_range().clone())
            );
        }
    }

    #[cfg(all(feature = "std", feature = "rand"))]
    struct MyClock(Cell<u64>);

    #[cfg(all(feature = "std", feature = "rand"))]
    impl Clock for MyClock {
        fn now(&self) -> Option<crate::Timestamp> {
            let ts = crate::Timestamp::from_unix(Duration::from_secs(self.0.get()));
            self.0.set(self.0.get() + 1);
            ts
        }
    }

    #[test]
    #[cfg(all(feature = "std", feature = "rand"))]
    fn span_guard_filtered_new() {
        let clock = MyClock(Cell::new(0));
        let rng = crate::platform::rand_rng::RandRng::new();
        let ctxt = crate::platform::thread_local_ctxt::ThreadLocalCtxt::new();

        let span_ctxt = SpanCtxt::new_root(&rng);

        let complete_called = Cell::new(false);

        let mut guard = SpanGuard::filtered_new(
            |_, _| true,
            Path::new_unchecked("test"),
            Timer::start(&clock),
            "span",
            span_ctxt,
            ("event_prop", 1),
            |evt| {
                assert_eq!(
                    Timestamp::from_unix(Duration::from_secs(0)).unwrap(),
                    evt.extent().unwrap().as_span().unwrap().start
                );
                assert_eq!(
                    Timestamp::from_unix(Duration::from_secs(1)).unwrap(),
                    evt.extent().unwrap().as_span().unwrap().end
                );

                assert_eq!("test", evt.mdl());
                assert_eq!("span", evt.name());

                assert_eq!(1, evt.props().pull::<usize, _>("event_prop").unwrap());

                let current_ctxt = SpanCtxt::current(&ctxt);

                assert_eq!(span_ctxt, current_ctxt);

                complete_called.set(true);
            },
        );

        assert!(guard.is_enabled());

        guard.push_ctxt(&ctxt, ("ctxt_prop", 2)).call(move || {
            drop(guard);
        });

        assert!(complete_called.get());
    }

    #[test]
    #[cfg(all(feature = "std", feature = "rand", not(miri)))]
    fn span_guard_filtered_new_disabled() {
        let rng = crate::platform::rand_rng::RandRng::new();
        let clock = crate::platform::system_clock::SystemClock::new();
        let ctxt = crate::platform::thread_local_ctxt::ThreadLocalCtxt::new();

        let complete_called = Cell::new(false);

        let mut guard = SpanGuard::filtered_new(
            |_, _| false,
            Path::new_unchecked("test"),
            Timer::start(&clock),
            "span",
            SpanCtxt::new_root(&rng),
            crate::Empty,
            |_| {
                complete_called.set(true);
            },
        );

        assert!(!guard.is_enabled());

        guard.push_ctxt(&ctxt, crate::Empty).call(move || {
            drop(guard);
        });

        assert!(!complete_called.get());
    }

    #[test]
    #[cfg(all(feature = "std", feature = "rand", not(miri)))]
    fn span_guard_custom_complete() {
        let clock = crate::platform::system_clock::SystemClock::new();
        let rng = crate::platform::rand_rng::RandRng::new();

        let custom_complete_called = Cell::new(false);
        let default_complete_called = Cell::new(false);

        let guard = SpanGuard::filtered_new(
            |_, _| true,
            Path::new_unchecked("test"),
            Timer::start(&clock),
            "span",
            SpanCtxt::new_root(&rng),
            crate::Empty,
            |_| {
                default_complete_called.set(true);
            },
        );

        assert!(guard.is_enabled());

        guard.complete_with(|_| {
            custom_complete_called.set(true);
        });

        assert!(!default_complete_called.get());
        assert!(custom_complete_called.get());
    }
}
