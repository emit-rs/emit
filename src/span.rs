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
    empty::Empty,
    event::{Event, ToEvent},
    extent::{Extent, ToExtent},
    filter::Filter,
    path::Path,
    props::{ErasedProps, Props},
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
    fmt, mem,
    num::{NonZeroU128, NonZeroU64},
    ops::ControlFlow,
    str::{self, FromStr},
};

pub use self::completion::Completion;

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
            .or_else(|| u128::from_value(value.by_ref()).and_then(TraceId::from_u128))
            .or_else(|| TraceId::try_from_hex(value).ok())
    }
}

#[cfg(feature = "sval")]
impl sval::Value for TraceId {
    fn stream<'sval, S: sval::Stream<'sval> + ?Sized>(&'sval self, stream: &mut S) -> sval::Result {
        sval::stream_display(stream, self)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for TraceId {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
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
            .or_else(|| u64::from_value(value.by_ref()).and_then(SpanId::from_u64))
            .or_else(|| SpanId::try_from_hex(value).ok())
    }
}

#[cfg(feature = "sval")]
impl sval::Value for SpanId {
    fn stream<'sval, S: sval::Stream<'sval> + ?Sized>(&'sval self, stream: &mut S) -> sval::Result {
        sval::stream_display(stream, self)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for SpanId {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
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
A diagnostic event that represents a span in a distributed trace.

Spans are an extension of [`Event`]s that explicitly take the well-known properties that signal an event as being a span. See the [`mod@crate::span`] module for details.

A `SpanEvent` can be converted into an [`Event`] through its [`ToEvent`] implemenation, or passed directly to a [`crate::Emitter`] to emit it.
*/
pub struct Span<'a, P> {
    mdl: Path<'a>,
    name: Str<'a>,
    extent: Option<Extent>,
    tpl: Option<Template<'a>>,
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
            tpl: None,
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
    Set the module of the span.
    */
    pub fn with_mdl(mut self, mdl: impl Into<Path<'a>>) -> Self {
        self.mdl = mdl.into();
        self
    }

    /**
    Get the name of the operation.
    */
    pub fn name(&self) -> &Str<'a> {
        &self.name
    }

    /**
    Set the name of the span.
    */
    pub fn with_name(mut self, name: impl Into<Str<'a>>) -> Self {
        self.name = name.into();
        self
    }

    /**
    Get the time the operation spent executing.
    */
    pub fn extent(&self) -> Option<&Extent> {
        self.extent.as_ref()
    }

    /**
    Set the extent of the span.
    */
    pub fn with_extent(mut self, extent: impl ToExtent) -> Self {
        self.extent = extent.to_extent();
        self
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
            .and_then(|extent| extent.as_range())
            .map(|span| &span.start)
    }

    /**
    Get the additional properties associated with the span.
    */
    pub fn props(&self) -> &P {
        &self.props
    }

    /**
    Set the properties of the span.
    */
    pub fn with_props<U>(self, props: U) -> Span<'a, U> {
        Span {
            mdl: self.mdl,
            extent: self.extent,
            name: self.name,
            tpl: self.tpl,
            props,
        }
    }

    /**
    Map the properties of the span.
    */
    pub fn map_props<U>(self, map: impl FnOnce(P) -> U) -> Span<'a, U> {
        Span {
            mdl: self.mdl,
            extent: self.extent,
            name: self.name,
            tpl: self.tpl,
            props: map(self.props),
        }
    }

    /**
    Get the template that will be used to render the span.
    */
    pub fn tpl(&self) -> &Template<'a> {
        self.tpl.as_ref().unwrap_or(&END_TEMPLATE)
    }

    /**
    Set the template of the span.
    */
    pub fn with_tpl(mut self, tpl: impl Into<Template<'a>>) -> Self {
        self.tpl = Some(tpl.into());
        self
    }

    /**
    Get a type-erased span, borrowing data from this one.
    */
    pub fn erase<'b>(&'b self) -> Span<'b, &'b dyn ErasedProps> {
        Span {
            mdl: self.mdl.by_ref(),
            extent: self.extent.clone(),
            name: self.name.by_ref(),
            tpl: self.tpl.as_ref().map(|tpl| tpl.by_ref()),
            props: &self.props,
        }
    }
}

// "{span_name} started"
const START_TEMPLATE_PARTS: &'static [template::Part<'static>] = &[
    template::Part::hole("span_name"),
    template::Part::text(" started"),
];

static START_TEMPLATE: Template<'static> = Template::new(&START_TEMPLATE_PARTS);

// "{span_name} completed"
const END_TEMPLATE_PARTS: &'static [template::Part<'static>] = &[
    template::Part::hole("span_name"),
    template::Part::text(" completed"),
];

static END_TEMPLATE: Template<'static> = Template::new(&END_TEMPLATE_PARTS);

impl<'a, P: Props> ToEvent for Span<'a, P> {
    type Props<'b>
        = &'b Self
    where
        Self: 'b;

    fn to_event<'b>(&'b self) -> Event<'b, Self::Props<'b>> {
        Event::new(
            self.mdl.by_ref(),
            self.tpl().by_ref(),
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
The trace id, span id, and parent span id of a span.

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

/**
An active span in a distributed trace.

## Creating active spans automatically

This type is created by the [`macro@crate::span!`] macro with the `guard` control parameter, or with the [`macro@crate::new_span!`] macro.



Call [`SpanGuard::complete_with`], or just drop the guard to complete it, passing the resulting [`Span`] to a [`Completion`].

## Creating active spans manually

The [`SpanGuard::new`] method can be used to construct a `SpanGuard` and [`Frame`] manually.

Call [`SpanGuard::start`] in the closure of [`Frame::call`] or async block of [`Frame::in_future`] on the returned [`Frame`] to begin the span. Once the span is started, it will complete automatically on drop, or manually through [`SpanGuard::complete`].

**Make sure you pass ownership of the returned `SpanGuard` into the closure in [`Frame::call`] or async block in [`Frame::in_future`]**. If you don't, the span will complete early, without its ambient context.
*/
pub struct SpanGuard<'a, T: Clock, P: Props, F: Completion> {
    state: SpanGuardState<T>,
    // `data` is `None` if the span is completed
    data: Option<SpanGuardData<'a, P>>,
    // `completion` is `None` if the span is disabled
    completion: Option<F>,
}

struct SpanGuardData<'a, P: Props> {
    mdl: Path<'a>,
    name: Str<'a>,
    ctxt: SpanCtxt,
    props: P,
}

enum SpanGuardState<T: Clock> {
    Initial(T),
    Started(Timer<T>),
    Completed,
}

impl<T: Clock> SpanGuardState<T> {
    fn take(&mut self) -> Self {
        mem::replace(self, SpanGuardState::Completed)
    }
}

impl<'a, T: Clock, P: Props, F: Completion> Drop for SpanGuard<'a, T, P, F> {
    fn drop(&mut self) {
        self.complete_default();
    }
}

impl<'a, T: Clock, P: Props, F: Completion> SpanGuard<'a, T, P, F> {
    /**
    Create a new active span.

    This method takes a number of parameters to construct a span. They are:

    - `filter`, `ctxt`, `clock`, `rng`: These typically come from a [`crate::runtime::Runtime`], like [`crate::runtime::shared`].
    - `completion`: A [`Completion`] that will be used by default when the returned `SpanGuard` is completed.
    - `ctxt_props`: A set of [`Props`] that will be pushed to the ambient context.
    - `span_mdl`, `span_name`, `span_props`: The input parameters to [`Span::new`] used to construct a span when the guard is completed.

    This method constructs a span based on the input properties and current context as follows:

    - A [`SpanCtxt`] for the span is generated using [`SpanCtxt::new_child`].
    - The filter is checked to see if the span should be enabled or disabled. The event passed to the filter is a [`Span`] carrying the generated span context, but without an extent.

    This method returns a tuple of a `SpanGuard` for starting and completing the span, and a [`Frame`] carrying the generated [`SpanCtxt`] and `ctxt_props`.

    Call [`SpanGuard::start`] in the closure of [`Frame::call`] or async block of [`Frame::in_future`] on the returned [`Frame`] to begin the span. Once the span is started, it will complete automatically on drop, or manually through [`SpanGuard::complete`].

    **Make sure you pass ownership of the returned `SpanGuard` into the closure in [`Frame::call`] or async block in [`Frame::in_future`]**. If you don't, the span will complete early, without its ambient context.
    */
    pub fn new<C: Ctxt>(
        filter: impl Filter,
        ctxt: C,
        clock: T,
        rng: impl Rng,
        completion: F,
        ctxt_props: impl Props,
        span_mdl: impl Into<Path<'a>>,
        span_name: impl Into<Str<'a>>,
        span_props: P,
    ) -> (Self, Frame<C>) {
        let span_mdl = span_mdl.into();
        let span_name = span_name.into();

        let span_ctxt = SpanCtxt::current(&ctxt).new_child(rng);

        // Check whether the span should be constructed using a dummy event
        let is_enabled = ctxt.with_current(|current_ctxt_props| {
            filter.matches(
                Span::new(
                    span_mdl.by_ref(),
                    span_name.by_ref(),
                    Empty,
                    (&span_props)
                        .and_props(&ctxt_props)
                        .and_props(span_ctxt)
                        .and_props(current_ctxt_props),
                )
                .to_event()
                .with_tpl(START_TEMPLATE.by_ref()),
            )
        });

        // Create a guard for the span
        // This can be completed automatically by dropping
        // or manually through the `complete` method
        let guard = SpanGuard {
            state: SpanGuardState::Initial(clock),
            data: Some(SpanGuardData {
                mdl: span_mdl,
                ctxt: span_ctxt,
                name: span_name,
                props: span_props,
            }),
            completion: if is_enabled { Some(completion) } else { None },
        };

        // Create a frame for the span props
        // This includes the trace and span ids
        let frame = guard.push_ctxt(ctxt, ctxt_props);

        (guard, frame)
    }

    fn push_ctxt<C: Ctxt>(&self, ctxt: C, ctxt_props: impl Props) -> Frame<C> {
        let span_ctxt = self.data.as_ref().expect("span is already complete").ctxt;

        if self.is_enabled() {
            Frame::push(ctxt, ctxt_props.and_props(span_ctxt))
        } else {
            Frame::disabled(ctxt, ctxt_props.and_props(span_ctxt))
        }
    }

    /**
    Start the span.

    From this point the span can be completed by either dropping this value, or by calling [`SpanGuard::complete`].
    */
    pub fn start(&mut self) {
        let state = mem::replace(&mut self.state, SpanGuardState::Completed);

        let SpanGuardState::Initial(clock) = state else {
            self.state = state;
            return;
        };

        self.state = SpanGuardState::Started(Timer::start(clock));
    }

    /**
    Whether the span will call its completion.

    If the filter called in [`SpanGuard::start`] evaluated to `false` then this method will also return `false`.
    */
    pub fn is_enabled(&self) -> bool {
        self.completion.is_some()
    }

    /**
    Set the default completion that will be called when the span is dropped or [`SpanGuard::complete`] is called.
    */
    pub fn with_completion<U: Completion>(mut self, completion: U) -> SpanGuard<'a, T, P, U> {
        // Ensure this guard won't complete on drop
        self.completion.take();

        SpanGuard {
            state: self.state.take(),
            data: self.data.take(),
            completion: Some(completion),
        }
    }

    /**
    Set the module of the span.
    */
    pub fn with_mdl(mut self, mdl: impl Into<Path<'a>>) -> Self {
        if let Some(ref mut data) = self.data {
            data.mdl = mdl.into();
        }
        self
    }

    /**
    Set the name of the span.
    */
    pub fn with_name(mut self, name: impl Into<Str<'a>>) -> Self {
        if let Some(ref mut data) = self.data {
            data.name = name.into();
        }
        self
    }

    /**
    Set the properties of the span.
    */
    pub fn with_props<U: Props>(self, props: U) -> SpanGuard<'a, T, U, F> {
        self.map_props(|_| props)
    }

    /**
    Map the properties of the span.
    */
    pub fn map_props<U: Props>(mut self, map: impl FnOnce(P) -> U) -> SpanGuard<'a, T, U, F> {
        let data = self.data.take().map(|data| SpanGuardData {
            mdl: data.mdl,
            name: data.name,
            ctxt: data.ctxt,
            props: map(data.props),
        });

        SpanGuard {
            state: self.state.take(),
            data,
            completion: self.completion.take(),
        }
    }

    /**
    Complete the span.

    If the span is disabled then this method is a no-op and will return `false`.
    */
    pub fn complete(mut self) -> bool {
        self.complete_default()
    }

    fn complete_default(&mut self) -> bool {
        if let (SpanGuardState::Started(timer), Some(data), Some(completion)) =
            (self.state.take(), self.data.take(), self.completion.take())
        {
            completion.complete(Span::new(data.mdl, data.name, timer, data.props));

            true
        } else {
            false
        }
    }

    /**
    Complete the span with the given closure.

    If the span is disabled then the `complete` closure won't be called and this method will return `false`.
    */
    pub fn complete_with(mut self, completion: impl Completion) -> bool {
        if let (SpanGuardState::Started(timer), Some(data), Some(_)) =
            (self.state.take(), self.data.take(), self.completion.take())
        {
            completion.complete(Span::new(data.mdl, data.name, timer, data.props));

            true
        } else {
            false
        }
    }
}

pub mod completion {
    /*!
    The [`Completion`] type.

    A [`Completion`] is a visitor for a [`Span`] that's called by a [`crate::span::SpanGuard`] when it completes.
    */

    use emit_core::{
        ctxt::Ctxt,
        emitter::Emitter,
        empty::Empty,
        event::ToEvent,
        props::{ErasedProps, Props},
        template::Template,
        value::{ToValue, Value},
        well_known::{KEY_ERR, KEY_LVL},
    };

    use crate::{level::Level, span::Span};

    use core::fmt;

    /**
    A receiver of [`Span`]s as they're completed by [`crate::span::SpanGuard`]s.
    */
    pub trait Completion {
        /**
        Receive a completing span.
        */
        fn complete<P: Props>(&self, span: Span<P>);
    }

    impl<'a, C: Completion + ?Sized> Completion for &'a C {
        fn complete<P: Props>(&self, span: Span<P>) {
            (**self).complete(span)
        }
    }

    impl Completion for Empty {
        fn complete<P: Props>(&self, _: Span<P>) {}
    }

    /**
    A default [`Completion`] that emits spans through an [`Emitter`].

    This type is a more sophisticated variant of [`FromEmitter`] that will:

    - Include ambient context from `C` on span events.
    - Detect panics and assign an error if it observes one.

    This type can be created directly, or via [`default`].
    */
    pub struct Default<'a, E, C, L = Level> {
        emitter: E,
        ctxt: C,
        tpl: Option<Template<'a>>,
        lvl: Option<L>,
        panic_lvl: Option<L>,
    }

    impl<'a, E: Emitter, C: Ctxt, L: ToValue> Completion for Default<'a, E, C, L> {
        fn complete<P: Props>(&self, span: Span<P>) {
            struct PanicError;

            impl fmt::Debug for PanicError {
                fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    fmt::Display::fmt(self, f)
                }
            }

            impl fmt::Display for PanicError {
                fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    write!(f, "panicked")
                }
            }

            #[cfg(feature = "std")]
            impl std::error::Error for PanicError {}

            impl ToValue for PanicError {
                fn to_value(&self) -> Value {
                    #[cfg(feature = "std")]
                    {
                        Value::capture_error(self)
                    }
                    #[cfg(not(feature = "std"))]
                    {
                        Value::capture_display(self)
                    }
                }
            }

            fn is_panicking() -> bool {
                #[cfg(feature = "std")]
                {
                    std::thread::panicking()
                }
                #[cfg(not(feature = "std"))]
                {
                    false
                }
            }

            let completion_props = if is_panicking() {
                [
                    self.panic_lvl
                        .as_ref()
                        .map(|lvl| Value::from_any(lvl))
                        .or_else(|| Some(Value::from_any(&Level::Error)))
                        .map(|lvl| (KEY_LVL, lvl)),
                    Some((KEY_ERR, Value::from_any(&PanicError))),
                ]
            } else {
                [
                    self.lvl
                        .as_ref()
                        .map(|lvl| Value::from_any(lvl))
                        .map(|lvl| (KEY_LVL, lvl)),
                    None,
                ]
            };

            let tpl = self.tpl.as_ref().unwrap_or_else(|| span.tpl()).by_ref();

            let evt = span
                .to_event()
                .with_tpl(tpl)
                .map_props(|span_props| completion_props.and_props(span_props));

            emit_core::emit(&self.emitter, Empty, &self.ctxt, Empty, evt);
        }
    }

    impl<'a, E, C, L> Default<'a, E, C, L> {
        /**
        Wrap the given emitter and context.
        */
        pub const fn new(emitter: E, ctxt: C) -> Self {
            Default {
                emitter,
                ctxt,
                tpl: None,
                lvl: None,
                panic_lvl: None,
            }
        }

        /**
        A level to assign to the span on completion.

        If the completion is called outside of a panic, this level will be used.
        */
        pub fn with_lvl(self, lvl: L) -> Self {
            Default {
                emitter: self.emitter,
                ctxt: self.ctxt,
                tpl: self.tpl,
                lvl: Some(lvl),
                panic_lvl: self.panic_lvl,
            }
        }

        /**
        A level to assign to the span on completion during a panic.
        */
        pub fn with_panic_lvl(self, lvl: L) -> Self {
            Default {
                emitter: self.emitter,
                ctxt: self.ctxt,
                tpl: self.tpl,
                lvl: self.lvl,
                panic_lvl: Some(lvl),
            }
        }

        /**
        A template to use for the span on completion.
        */
        pub fn with_tpl<'b>(self, tpl: impl Into<Template<'b>>) -> Default<'b, E, C, L> {
            Default {
                emitter: self.emitter,
                ctxt: self.ctxt,
                tpl: Some(tpl.into()),
                lvl: self.lvl,
                panic_lvl: self.panic_lvl,
            }
        }
    }

    /**
    Create a default [`Completion`] from an [`Emitter`] and [`Ctxt`].

    On completion, a [`Span`] will be emitted as an event using [`Span::to_event`].

    If the completion is called during a panic, it will attach an error to the span.
    */
    pub const fn default<'a, E: Emitter, C: Ctxt>(emitter: E, ctxt: C) -> Default<'a, E, C> {
        Default::new(emitter, ctxt)
    }

    /**
    A [`Completion`] from an [`Emitter`].

    On completion, a [`Span`] will be emitted as an event using [`Span::to_event`].

    This type can be created directly, or via [`from_emitter`].
    */
    pub struct FromEmitter<E>(E);

    impl<E: Emitter> Completion for FromEmitter<E> {
        fn complete<P: Props>(&self, span: Span<P>) {
            self.0.emit(span)
        }
    }

    impl<E> FromEmitter<E> {
        /**
        Wrap the given emitter.
        */
        pub const fn new(emitter: E) -> Self {
            FromEmitter(emitter)
        }
    }

    /**
    Create a [`Completion`] from an [`Emitter`].

    On completion, a [`Span`] will be emitted as an event using [`Span::to_event`].
    */
    pub const fn from_emitter<E: Emitter>(emitter: E) -> FromEmitter<E> {
        FromEmitter(emitter)
    }

    /**
    A [`Completion`] from a function.

    This type can be created directly, or via [`from_fn`].
    */
    pub struct FromFn<F = fn(Span<&dyn ErasedProps>)>(F);

    /**
    Create a [`Completion`] from a function.
    */
    pub const fn from_fn<F: Fn(Span<&dyn ErasedProps>)>(f: F) -> FromFn<F> {
        FromFn(f)
    }

    impl<F> FromFn<F> {
        /**
        Wrap the given completion function.
        */
        pub const fn new(completion: F) -> FromFn<F> {
            FromFn(completion)
        }
    }

    impl<F: Fn(Span<&dyn ErasedProps>)> Completion for FromFn<F> {
        fn complete<P: Props>(&self, span: Span<P>) {
            (self.0)(span.erase())
        }
    }

    mod internal {
        use super::*;

        pub trait DispatchCompletion {
            fn dispatch_complete(&self, span: Span<&dyn ErasedProps>);
        }

        pub trait SealedCompletion {
            fn erase_completion(&self) -> crate::internal::Erased<&dyn DispatchCompletion>;
        }
    }

    /**
    An object-safe [`Completion`].

    A `dyn ErasedCompletion` can be treated as `impl Completion`.
    */
    pub trait ErasedCompletion: internal::SealedCompletion {}

    impl<T: Completion> ErasedCompletion for T {}

    impl<T: Completion> internal::SealedCompletion for T {
        fn erase_completion(&self) -> crate::internal::Erased<&dyn internal::DispatchCompletion> {
            crate::internal::Erased(self)
        }
    }

    impl<T: Completion> internal::DispatchCompletion for T {
        fn dispatch_complete(&self, span: Span<&dyn ErasedProps>) {
            self.complete(span)
        }
    }

    impl<'a> Completion for dyn ErasedCompletion + 'a {
        fn complete<P: Props>(&self, span: Span<P>) {
            self.erase_completion().0.dispatch_complete(span.erase())
        }
    }

    impl<'a> Completion for dyn ErasedCompletion + Send + Sync + 'a {
        fn complete<P: Props>(&self, span: Span<P>) {
            (self as &(dyn ErasedCompletion + 'a)).complete(span)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::cell::Cell;

        use emit_core::{emitter, path::Path};

        #[test]
        fn from_fn_completion() {
            let called = Cell::new(false);

            let completion = from_fn(|span| {
                assert_eq!("test", span.name());

                called.set(true);
            });

            completion.complete(Span::new(Path::new_raw("test"), "test", Empty, Empty));

            assert!(called.get());
        }

        #[test]
        fn erased_completion() {
            let called = Cell::new(false);

            let completion = from_fn(|span| {
                assert_eq!("test", span.name());

                called.set(true);
            });

            let completion = &completion as &dyn ErasedCompletion;

            completion.complete(Span::new(Path::new_raw("test"), "test", Empty, Empty));

            assert!(called.get());
        }

        #[test]
        fn default_completion() {
            let called = Cell::new(false);

            let completion = default(
                emitter::from_fn(|_| {
                    called.set(true);
                }),
                Empty,
            );

            completion.complete(Span::new(Path::new_raw("test"), "test", Empty, Empty));

            assert!(called.get());
        }

        #[test]
        fn default_completion_uses_lvl() {
            let called = Cell::new(false);

            let completion = default(
                emitter::from_fn(|evt| {
                    assert_eq!(Level::Info, evt.props().pull("lvl").unwrap());

                    called.set(true);
                }),
                Empty,
            )
            .with_lvl(Level::Info);

            completion.complete(Span::new(Path::new_raw("test"), "test", Empty, Empty));

            assert!(called.get());
        }

        #[test]
        fn default_completion_uses_tpl() {
            let called = Cell::new(false);

            let completion = default(
                emitter::from_fn(|evt| {
                    assert_eq!("test template", evt.msg().to_string());

                    called.set(true);
                }),
                Empty,
            )
            .with_lvl(Level::Info)
            .with_tpl(Template::literal("test template"));

            completion.complete(Span::new(Path::new_raw("test"), "test", Empty, Empty));

            assert!(called.get());
        }

        #[cfg(feature = "std")]
        struct Guard<T: Completion>(T);

        #[cfg(feature = "std")]
        impl<T: Completion> Drop for Guard<T> {
            fn drop(&mut self) {
                self.0
                    .complete(Span::new(Path::new_raw("test"), "test", Empty, Empty));
            }
        }

        #[test]
        #[cfg(feature = "std")]
        fn default_completion_detects_panics() {
            let called = Cell::new(false);

            let completion = default(
                emitter::from_fn(|evt| {
                    assert_eq!(Level::Error, evt.props().pull("lvl").unwrap());
                    assert!(evt.props().get("err").is_some());

                    called.set(true);
                }),
                Empty,
            );

            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                let _guard = Guard(completion);

                panic!("explicit panic")
            }));

            assert!(called.get());
        }

        #[test]
        #[cfg(feature = "std")]
        fn default_completion_uses_panic_lvl() {
            let called = Cell::new(false);

            let completion = default(
                emitter::from_fn(|evt| {
                    assert_eq!(Level::Warn, evt.props().pull("lvl").unwrap());

                    called.set(true);
                }),
                Empty,
            )
            .with_panic_lvl(Level::Warn);

            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                let _guard = Guard(completion);

                panic!("explicit panic")
            }));

            assert!(called.get());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use emit_core::filter;

    use std::time::Duration;

    #[cfg(all(feature = "std", feature = "rand", not(miri)))]
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
        assert!(SpanId::random(Empty).is_none());
    }

    #[test]
    #[cfg(feature = "rand")]
    fn span_id_random_rand() {
        assert!(SpanId::random(crate::platform::rand_rng::RandRng::new()).is_some());
    }

    #[test]
    fn trace_id_random_empty() {
        assert!(TraceId::random(Empty).is_none());
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
    fn span_id_from_value_u64() {
        assert_eq!(
            SpanId::from_u64(0x0123456789abcdef).unwrap(),
            Value::from(0x0123456789abcdefu64).cast().unwrap()
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
    fn trace_id_from_value_u128() {
        assert_eq!(
            TraceId::from_u128(0x0123456789abcdef0123456789abcdef).unwrap(),
            Value::from(0x0123456789abcdef0123456789abcdefu128)
                .cast()
                .unwrap()
        );
    }

    #[cfg(feature = "sval")]
    #[test]
    fn span_id_stream() {
        sval_test::assert_tokens(
            &SpanId::from_u64(0x0123456789abcdef).unwrap(),
            &[
                sval_test::Token::TextBegin(None),
                sval_test::Token::TextFragmentComputed("0123456789abcdef".to_owned()),
                sval_test::Token::TextEnd,
            ],
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn span_id_serialize() {
        serde_test::assert_ser_tokens(
            &SpanId::from_u64(0x0123456789abcdef).unwrap(),
            &[serde_test::Token::Str("0123456789abcdef")],
        );
    }

    #[cfg(feature = "sval")]
    #[test]
    fn trace_id_stream() {
        sval_test::assert_tokens(
            &TraceId::from_u128(0x0123456789abcdef0123456789abcdef).unwrap(),
            &[
                sval_test::Token::TextBegin(None),
                sval_test::Token::TextFragmentComputed(
                    "0123456789abcdef0123456789abcdef".to_owned(),
                ),
                sval_test::Token::TextEnd,
            ],
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn trace_id_serialize() {
        serde_test::assert_ser_tokens(
            &TraceId::from_u128(0x0123456789abcdef0123456789abcdef).unwrap(),
            &[serde_test::Token::Str("0123456789abcdef0123456789abcdef")],
        );
    }

    #[test]
    #[cfg(all(feature = "std", feature = "rand", not(miri)))]
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
            Path::new_raw("test"),
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
            Path::new_raw("test"),
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
    fn span_to_event_uses_tpl() {
        assert_eq!(
            "test",
            Span::new(
                Path::new_raw("test"),
                "my span",
                Timestamp::from_unix(Duration::from_secs(1)),
                ("span_prop", true),
            )
            .with_tpl(Template::literal("test"))
            .to_event()
            .msg()
            .to_string(),
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
            let span = Span::new(Path::new_raw("test"), "my span", case, ("span_prop", true));

            let extent = span.to_extent();

            assert_eq!(
                expected.map(|extent| extent.as_range().cloned()),
                extent.map(|extent| extent.as_range().cloned())
            );
        }
    }

    #[cfg(all(feature = "std", feature = "rand", not(miri)))]
    struct MyClock(Cell<u64>);

    #[cfg(all(feature = "std", feature = "rand", not(miri)))]
    impl Clock for MyClock {
        fn now(&self) -> Option<crate::Timestamp> {
            let ts = crate::Timestamp::from_unix(Duration::from_secs(self.0.get()));
            self.0.set(self.0.get() + 1);
            ts
        }
    }

    #[test]
    #[cfg(all(feature = "std", feature = "rand", not(miri)))]
    fn active_span_new() {
        let clock = MyClock(Cell::new(0));
        let rng = crate::platform::rand_rng::RandRng::new();
        let ctxt = crate::platform::thread_local_ctxt::ThreadLocalCtxt::new();

        let complete_called = Cell::new(false);

        let (mut guard, frame) = SpanGuard::new(
            filter::from_fn(|evt| {
                assert_eq!(2, evt.props().pull::<usize, _>("ctxt_prop").unwrap());

                assert!(evt.props().get("trace_id").is_some());
                assert!(evt.props().get("span_id").is_some());

                true
            }),
            &ctxt,
            &clock,
            &rng,
            completion::from_fn(|evt| {
                assert_eq!(
                    Timestamp::from_unix(Duration::from_secs(0)).unwrap(),
                    evt.extent().unwrap().as_range().unwrap().start
                );
                assert_eq!(
                    Timestamp::from_unix(Duration::from_secs(1)).unwrap(),
                    evt.extent().unwrap().as_range().unwrap().end
                );

                assert_eq!("test", evt.mdl());
                assert_eq!("span", evt.name());

                assert_eq!(1, evt.props().pull::<usize, _>("event_prop").unwrap());

                ctxt.with_current(|props| {
                    assert_eq!(2, props.pull::<usize, _>("ctxt_prop").unwrap());
                });

                let current_ctxt = SpanCtxt::current(&ctxt);

                assert_ne!(current_ctxt, SpanCtxt::empty());

                complete_called.set(true);
            }),
            ("ctxt_prop", 2),
            Path::new_raw("test"),
            "span",
            ("event_prop", 1),
        );

        assert!(guard.is_enabled());

        frame.call(move || {
            guard.start();

            drop(guard);
        });

        assert!(complete_called.get());
    }

    #[test]
    #[cfg(all(feature = "std", feature = "rand", not(miri)))]
    fn active_span_unstarted_complete() {
        let clock = MyClock(Cell::new(0));
        let rng = crate::platform::rand_rng::RandRng::new();
        let ctxt = crate::platform::thread_local_ctxt::ThreadLocalCtxt::new();

        let complete_called = Cell::new(false);

        let (guard, frame) = SpanGuard::new(
            filter::from_fn(|_| true),
            &ctxt,
            &clock,
            &rng,
            completion::from_fn(|_| {}),
            Empty,
            Path::new_raw("test"),
            "span",
            Empty,
        );

        assert!(guard.is_enabled());

        frame.call(move || {
            drop(guard);
        });

        assert!(!complete_called.get());
    }

    #[test]
    #[cfg(all(feature = "std", feature = "rand", not(miri)))]
    fn active_span_new_disabled() {
        let rng = crate::platform::rand_rng::RandRng::new();
        let clock = crate::platform::system_clock::SystemClock::new();
        let ctxt = crate::platform::thread_local_ctxt::ThreadLocalCtxt::new();

        let complete_called = Cell::new(false);

        let (mut guard, frame) = SpanGuard::new(
            filter::from_fn(|_| false),
            &ctxt,
            &clock,
            &rng,
            completion::from_fn(|_| {
                complete_called.set(true);
            }),
            Empty,
            Path::new_raw("test"),
            "span",
            Empty,
        );

        assert!(!guard.is_enabled());

        frame.call(move || {
            guard.start();

            drop(guard);
        });

        assert!(!complete_called.get());
    }

    #[test]
    #[cfg(all(feature = "std", feature = "rand", not(miri)))]
    fn active_span_custom_complete() {
        let ctxt = crate::platform::thread_local_ctxt::ThreadLocalCtxt::new();
        let clock = crate::platform::system_clock::SystemClock::new();
        let rng = crate::platform::rand_rng::RandRng::new();

        let custom_complete_called = Cell::new(false);
        let default_complete_called = Cell::new(false);

        let (mut guard, _) = SpanGuard::new(
            filter::from_fn(|_| true),
            &ctxt,
            &clock,
            &rng,
            completion::from_fn(|_| {
                default_complete_called.set(true);
            }),
            Empty,
            Path::new_raw("test"),
            "span",
            Empty,
        );

        assert!(guard.is_enabled());

        guard.start();

        guard.complete_with(completion::from_fn(|_| {
            custom_complete_called.set(true);
        }));

        assert!(!default_complete_called.get());
        assert!(custom_complete_called.get());
    }

    #[test]
    #[cfg(all(feature = "std", feature = "rand", not(miri)))]
    fn active_span_with_props() {
        let clock = MyClock(Cell::new(0));
        let rng = crate::platform::rand_rng::RandRng::new();
        let ctxt = crate::platform::thread_local_ctxt::ThreadLocalCtxt::new();

        let complete_called = Cell::new(false);

        let (mut guard, frame) = SpanGuard::new(
            filter::from_fn(|_| true),
            &ctxt,
            &clock,
            &rng,
            completion::from_fn(|evt| {
                assert_eq!(2, evt.props().pull::<usize, _>("event_prop").unwrap());

                complete_called.set(true);
            }),
            Empty,
            Path::new_raw("test"),
            "span",
            ("event_prop", 1),
        );

        frame.call(move || {
            guard.start();

            let guard = guard.with_props(("event_prop", 2));

            drop(guard);
        });

        assert!(complete_called.get());
    }
}
