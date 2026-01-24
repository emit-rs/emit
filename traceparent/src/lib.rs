/*!
Distributed trace context for `emit`.

This library implements the [W3C trace context](https://www.w3.org/TR/trace-context) standard over `emit`'s tracing functionality. Trace context is propagated as a traceparent and tracestate in a simple text format. Here's an example of a traceparent:

```text
00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01
```

It includes:

- A _version_: `00`.
- A _trace id_: `4bf92f3577b34da6a3ce929d0e0e4736`.
- A _span id_: `00f067aa0ba902b7`.
- A set of _trace flags_: `01` (sampled).

Here's an example of tracestate:

```text
vendorname1=opaqueValue1,vendorname2=opaqueValue2
```

It contains a collection of key-value pairs with vendor-specific information.

# Getting started

Add `emit` and `emit_traceparent` to your `Cargo.toml`:

```toml
[dependencies.emit]
version = "1.16.1"

[dependencies.emit_traceparent]
version = "1.16.1"
```

Initialize `emit` using the [`setup`] or [`setup_with_sampler`] functions from this library:

```
fn main() {
    let rt = emit_traceparent::setup()
        .emit_to(emit_term::stdout())
        .init();

    // Your app code goes here

    rt.blocking_flush(std::time::Duration::from_secs(30));
}
```

# Incoming traceparent

When a request arrives, parse a [`Traceparent`] from it and push it onto the current context. Handle the request in the returned [`emit::Frame`] to make trace context correlate with the upstream service:

```
let traceparent = emit_traceparent::Traceparent::try_from_str("00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01")
    .unwrap_or_else(|_| emit_traceparent::Traceparent::current());

// 2. Push the traceparent onto the context and execute your handler within it
traceparent.push().call(handle_request);

#[emit::span("incoming request")]
fn handle_request() {
    // Your code goes here
}
```

# Outgoing traceparent

When making an outgoing request, get the current [`Traceparent`] and format it into a header for the downstream service:

```
# use std::collections::HashMap;
let mut headers = HashMap::<String, String>::new();

// 1. Get the current traceparent
let traceparent = emit_traceparent::Traceparent::current();

if traceparent.is_valid() {
    // 2. Add the traceparent to the outgoing request
    headers.insert("traceparent".into(), traceparent.to_string());
}
```

# `Traceparent` and `SpanCtxt`

`emit` stores the active span context as an [`emit::SpanCtxt`] in its [`emit::Ctxt`], which it generates in [`macro@emit::span`] for you. `SpanCtxt` doesn't have the concept of sampling, so if it exists then it's sampled.
When in use, the [`Traceparent`] type defined by this library becomes the source of truth for the `SpanCtxt`. If the current `Traceparent` is not sampled, then no `SpanCtxt` will be returned.

Only the span id on incoming `SpanCtxt`s created by [`macro@emit::span`] are respected. The current `Traceparent` overrides any incoming trace ids or span parents.

# Sampling

The [`setup_with_sampler`] function lets you configure a sampling function that's run on the first span of each trace.
See the function docs for more details.
*/

#![deny(missing_docs)]

use std::{
    cell::RefCell,
    fmt::{self, Write as _},
    mem,
    ops::{BitAnd, BitOr, ControlFlow, Not},
    str::{self, FromStr},
};

use emit::{
    event::ToEvent,
    span::{SpanCtxt, SpanId, TraceId},
    well_known::{KEY_SPAN_ID, KEY_SPAN_PARENT, KEY_TRACE_ID},
    Ctxt, Empty, Filter, Frame, Props, Str, Value,
};

/**
Start a builder for a distributed-trace-aware pipeline.

```
fn main() {
    let rt = emit_traceparent::setup()
        .emit_to(emit_term::stdout())
        .init();

    // Your app code goes here

    rt.blocking_flush(std::time::Duration::from_secs(30));
}
```
*/
pub fn setup() -> emit::Setup<
    emit::setup::DefaultEmitter,
    TraceparentFilter,
    TraceparentCtxt<emit::setup::DefaultCtxt>,
> {
    emit::setup()
        .emit_when(TraceparentFilter::new())
        .map_ctxt(|ctxt| TraceparentCtxt::new(ctxt))
}

/**
Start a builder for a distributed-trace-aware pipeline with a sampler.

The sampler will be called once for each trace when it's started.
If the sampler returns `true`, the trace will be emitted.
If the sampler returns `false`, the trace will be discarded.

```
use std::sync::atomic::{AtomicUsize, Ordering};

fn main() {
    let rt = emit_traceparent::setup_with_sampler({
        let counter = AtomicUsize::new(0);

        move |_| {
            // Sample 1 in every 10 traces
            counter.fetch_add(1, Ordering::Relaxed) % 10 == 0
        }
    })
    .emit_to(emit_term::stdout())
    .init();

    // Your app code goes here

    rt.blocking_flush(std::time::Duration::from_secs(30));
}
```

Note that other events emitted within an unsampled trace will still be emitted.
You can add the [`in_sampled_trace_filter`] filter to filter out events in unsampled traces:

```
use std::sync::atomic::{AtomicUsize, Ordering};

fn main() {
    let rt = emit_traceparent::setup_with_sampler({
        let counter = AtomicUsize::new(0);

        move |_| {
            // Sample 1 in every 10 traces
            counter.fetch_add(1, Ordering::Relaxed) % 10 == 0
        }
    })
    // The `true` here tells us to include events outside traces
    // If we pass `false` then any event outside a trace will be discarded
    .and_emit_when(emit_traceparent::in_sampled_trace_filter(true))
    .emit_to(emit_term::stdout())
    .init();

    // Your app code goes here

    rt.blocking_flush(std::time::Duration::from_secs(30));
}
```

The `true` parameter passed to `in_sampled_trace_filter` here includes events that aren't in any trace.
Passing `false` will also filter out events that aren't part of any trace.
*/
pub fn setup_with_sampler<S: Fn(&SpanCtxt) -> bool + Send + Sync + 'static>(
    sampler: S,
) -> emit::Setup<
    emit::setup::DefaultEmitter,
    TraceparentFilter<S>,
    TraceparentCtxt<emit::setup::DefaultCtxt>,
> {
    emit::setup()
        .emit_when(TraceparentFilter::new_with_sampler(sampler))
        .map_ctxt(|ctxt| TraceparentCtxt::new(ctxt))
}

/**
An error encountered attempting to work with a [`Traceparent`].
*/
#[derive(Debug)]
pub struct Error {
    msg: String,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.msg, f)
    }
}

impl std::error::Error for Error {}

/**
Get the current [`Traceparent`] and [`Tracestate`].

This method is equivalent to calling [`Traceparent::current`] and [`Tracestate::current`], but avoids some duplicate overhead.
*/
pub fn current() -> (Traceparent, Tracestate) {
    get_active_traceparent()
        .map(|active| (active.traceparent, active.tracestate))
        .unwrap_or((Traceparent::empty(), Tracestate::empty()))
}

/**
Set the current [`Traceparent`] and [`Tracestate`].

This method is equivalent to calling [`Traceparent::push`] and [`Tracestate::push`], but avoids some duplicate overhead.
*/
pub fn push(traceparent: Traceparent, tracestate: Tracestate) -> Frame<TraceparentCtxt> {
    let mut frame = Frame::current(TraceparentCtxt::new(Empty));

    let slot = if let Some(active) = get_active_traceparent() {
        ActiveTraceparent {
            span_parent: if active.is_parent_of(traceparent.trace_id) {
                active.traceparent.span_id
            } else {
                None
            },
            traceparent,
            tracestate,
        }
    } else {
        ActiveTraceparent {
            traceparent,
            tracestate,
            span_parent: None,
        }
    };

    frame.inner_mut().slot = Some(slot);
    frame.inner_mut().active = true;

    frame
}

/**
A [W3C traceparent](https://www.w3.org/TR/trace-context/#traceparent-header).

This type contains `emit`'s [`TraceId`] and [`SpanId`], along with [`TraceFlags`] that determine sampling.

Traceparents exist at the edges of your application.
On incoming requests, it may carry a traceparent header that can be parsed into a `Traceparent` and pushed onto the active trace context with [`Traceparent::push`].
On outgoing requests, the active trace context is pulled by [`Traceparent::current`] and formatted into a traceparent header.
*/
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Traceparent {
    trace_id: Option<TraceId>,
    span_id: Option<SpanId>,
    trace_flags: TraceFlags,
}

impl Traceparent {
    /**
    Create a new traceparent with the given trace id, span id, and trace flags.
    */
    pub const fn new(
        trace_id: Option<TraceId>,
        span_id: Option<SpanId>,
        trace_flags: TraceFlags,
    ) -> Self {
        Traceparent {
            trace_id,
            span_id,
            trace_flags,
        }
    }

    const fn empty() -> Self {
        Traceparent::new(None, None, TraceFlags::SAMPLED)
    }

    /**
    Parse a traceparent from its text format, as defined in the [W3C standard](https://www.w3.org/TR/trace-context).
    */
    pub fn try_from_str(traceparent: &str) -> Result<Self, Error> {
        let bytes = traceparent.as_bytes();

        if bytes.len() != 55 {
            return Err(Error {
                msg: "traceparent headers must be 55 bytes".into(),
            });
        }

        if bytes[2] != b'-' || bytes[35] != b'-' || bytes[52] != b'-' {
            return Err(Error {
                msg: format!("traceparent contains invalid field separators"),
            });
        }

        let version = &bytes[0..2];

        let b"00" = version else {
            return Err(Error {
                msg: format!("unexpected non '00' traceparent version"),
            });
        };

        let trace_id = &bytes[3..35];
        let span_id = &bytes[36..52];
        let trace_flags = &bytes[53..55];

        let trace_id = if trace_id == b"00000000000000000000000000000000" {
            None
        } else {
            Some(TraceId::try_from_hex_slice(trace_id).map_err(|e| Error { msg: e.to_string() })?)
        };

        let span_id = if span_id == b"0000000000000000" {
            None
        } else {
            Some(SpanId::try_from_hex_slice(span_id).map_err(|e| Error { msg: e.to_string() })?)
        };

        let trace_flags = TraceFlags::try_from_hex_slice(trace_flags)?;

        Ok(Traceparent::new(trace_id, span_id, trace_flags))
    }

    /**
    Get the trace id.
    */
    pub fn trace_id(&self) -> Option<&TraceId> {
        self.trace_id.as_ref()
    }

    /**
    Get the span id (also called the parent id).
    */
    pub fn span_id(&self) -> Option<&SpanId> {
        self.span_id.as_ref()
    }

    /**
    Get the trace flags.

    These flags can be used to tell whether the traceparent is for a sampled trace or not.
    */
    pub fn trace_flags(&self) -> &TraceFlags {
        &self.trace_flags
    }

    /**
    Whether the sampled flag is set.
    */
    pub fn is_sampled(&self) -> bool {
        self.trace_flags.is_sampled()
    }

    /**
    Get the current trace context.

    If no context has been set, this method will return a new traceparent with no trace id or span id, but with [`TraceFlags::SAMPLED`].

    Note that unless a [`TraceparentCtxt`] is configured in [`emit::setup`], or if trace context is being managed through [`Traceparent::push`], this method will return a new traceparent.
    It won't pick up values in [`emit::span::SpanCtxt`].
    */
    pub fn current() -> Self {
        get_active_traceparent()
            .map(|active| active.traceparent)
            .unwrap_or(Traceparent::empty())
    }

    /**
    Get a [`Frame`] that can set the current trace context in a scope.

    While the frame is active, [`Traceparent::current`] will return this traceparent.
    */
    pub fn push(&self) -> Frame<TraceparentCtxt> {
        let traceparent = *self;

        let mut frame = Frame::current(TraceparentCtxt::new(Empty));

        let slot = if let Some(active) = get_active_traceparent() {
            ActiveTraceparent {
                span_parent: if active.is_parent_of(self.trace_id) {
                    active.traceparent.span_id
                } else {
                    None
                },
                traceparent,
                tracestate: active.tracestate,
            }
        } else {
            ActiveTraceparent {
                traceparent,
                tracestate: Tracestate::empty(),
                span_parent: None,
            }
        };

        frame.inner_mut().slot = Some(slot);
        frame.inner_mut().active = true;

        frame
    }

    /**
    Whether the traceparent carries a non-empty trace id and span id.

    An invalid traceparent can still be used for propagation, but will likely be ignored by downstream services.
    */
    pub fn is_valid(&self) -> bool {
        self.trace_id.is_some() && self.span_id.is_some()
    }
}

impl FromStr for Traceparent {
    type Err = Error;

    fn from_str(header: &str) -> Result<Self, Error> {
        Self::try_from_str(header)
    }
}

impl fmt::Display for Traceparent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("00-")?;

        if let Some(trace_id) = self.trace_id {
            fmt::Display::fmt(&trace_id, f)?;
            f.write_char('-')?;
        } else {
            f.write_str("00000000000000000000000000000000-")?;
        }

        if let Some(span_id) = self.span_id {
            fmt::Display::fmt(&span_id, f)?;
            f.write_char('-')?;
        } else {
            f.write_str("0000000000000000-")?;
        }

        fmt::Display::fmt(&self.trace_flags, f)
    }
}

/**
A set of flags associated with a [`Traceparent`].

Trace flags can be used to control and communicate sampling decisions between distributed services.
*/
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TraceFlags(u8);

impl TraceFlags {
    /**
    An empty set of unsampled trace flags (`00`).
    */
    pub const EMPTY: Self = TraceFlags(0);

    /**
    A sampled set of trace flags (`01`).
    */
    pub const SAMPLED: Self = TraceFlags(1);

    const ALL: Self = TraceFlags(!0);

    /**
    Get a set of trace flags from a raw byte value.
    */
    pub fn from_u8(raw: u8) -> Self {
        TraceFlags(raw)
    }

    /**
    Get the trace flags as a raw byte value.
    */
    pub fn to_u8(&self) -> u8 {
        self.0
    }

    /**
    Whether the sampled flag is set (`01`).
    */
    pub fn is_sampled(&self) -> bool {
        self.0 & Self::SAMPLED.0 == 1
    }

    /**
    Convert the trace flags into a 2 byte ASCII-compatible hex string, like `01`.
    */
    pub fn to_hex(&self) -> [u8; 2] {
        const HEX_ENCODE_TABLE: [u8; 16] = [
            b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'a', b'b', b'c', b'd',
            b'e', b'f',
        ];

        [
            HEX_ENCODE_TABLE[(self.0 >> 4) as usize],
            HEX_ENCODE_TABLE[(self.0 & 0x0f) as usize],
        ]
    }

    /**
    Try parse a slice of ASCII hex bytes into trace flags.

    If `hex` is not a 2 byte array of valid hex characters (`[a-fA-F0-9]`) then this method will fail.
    */
    pub fn try_from_hex_slice(hex: &[u8]) -> Result<Self, Error> {
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

        let hex: &[u8; 2] = hex.try_into().map_err(|_| Error {
            msg: "flags must be a 2 digit value".into(),
        })?;

        let h1 = HEX_DECODE_TABLE[hex[0] as usize];
        let h2 = HEX_DECODE_TABLE[hex[1] as usize];

        // We use `0xff` as a sentinel value to indicate
        // an invalid hex character sequence (like the letter `G`)
        if h1 | h2 == 0xff {
            return Err(Error {
                msg: "invalid hex character".into(),
            });
        }

        // The upper nibble needs to be shifted into position
        // to produce the final byte value
        Ok(TraceFlags((h1 << 4) | h2))
    }
}

impl BitAnd for TraceFlags {
    type Output = Self;

    fn bitand(self, other: Self) -> Self {
        TraceFlags(self.0 & other.0)
    }
}

impl BitOr for TraceFlags {
    type Output = Self;

    fn bitor(self, other: Self) -> Self {
        TraceFlags(self.0 | other.0)
    }
}

impl Not for TraceFlags {
    type Output = Self;

    fn not(self) -> Self {
        TraceFlags(!self.0)
    }
}

impl FromStr for TraceFlags {
    type Err = Error;

    fn from_str(flags: &str) -> Result<Self, Error> {
        Self::try_from_hex_slice(flags.as_bytes())
    }
}

impl fmt::Display for TraceFlags {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(str::from_utf8(&self.to_hex()).unwrap())
    }
}

/**
A [W3C tracestate](https://www.w3.org/TR/trace-context/#tracestate-header).

The role of `Tracestate` is to store and propagate the tracestate header.
It doesn't currently support any format-aware modification of the underlying value, nor does it parse or validate it.
*/
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tracestate(Str<'static>);

impl fmt::Display for Tracestate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl Tracestate {
    /**
    Construct a new tracestate with the given value, without checking its validity.

    This method is not unsafe. There are no safety properties tied to the validity of tracestate values.
    Downstream consumers may discard a tracestate value if it's not valid.
    */
    pub const fn new_raw(value: &'static str) -> Tracestate {
        Tracestate(Str::new(value))
    }

    /**
    Construct a new tracestate with the given value, without checking its validity.

    This method is not unsafe. There are no safety properties tied to the validity of tracestate values.
    Downstream consumers may discard a tracestate value if it's not valid.
    */
    pub const fn new_str_raw(value: Str<'static>) -> Tracestate {
        Tracestate(value)
    }

    /**
    Construct a new tracestate with the given value, without checking its validity.

    This method is not unsafe. There are no safety properties tied to the validity of tracestate values.
    Downstream consumers may discard a tracestate value if it's not valid.
    */
    pub fn new_owned_raw(value: impl Into<Box<str>>) -> Tracestate {
        Tracestate(Str::new_owned(value))
    }

    const fn empty() -> Tracestate {
        Tracestate(Str::new(""))
    }

    /**
    Get the value of this tracestate.
    */
    pub fn get(&self) -> &str {
        self.0.get()
    }

    /**
    Get the current trace state.

    If no context has been set, this method will return an empty value.
    */
    pub fn current() -> Self {
        get_active_traceparent()
            .map(|active| active.tracestate)
            .unwrap_or(Tracestate::empty())
    }

    /**
    Get a [`Frame`] that can set the current trace state in a scope.

    While the frame is active, [`Tracestate::current`] will return this state.
    */
    pub fn push(&self) -> Frame<TraceparentCtxt> {
        let tracestate = Tracestate(self.0.to_shared());

        let mut frame = Frame::current(TraceparentCtxt::new(Empty));

        let slot = if let Some(active) = get_active_traceparent() {
            ActiveTraceparent {
                span_parent: active.span_parent,
                traceparent: active.traceparent,
                tracestate,
            }
        } else {
            ActiveTraceparent {
                traceparent: Traceparent::empty(),
                tracestate,
                span_parent: None,
            }
        };

        frame.inner_mut().slot = Some(slot);
        frame.inner_mut().active = true;

        frame
    }
}

#[derive(Debug, Clone)]
struct ActiveTraceparent {
    traceparent: Traceparent,
    tracestate: Tracestate,
    span_parent: Option<SpanId>,
}

impl ActiveTraceparent {
    fn is_parent_of(&self, trace_id: Option<TraceId>) -> bool {
        // A traceparent is a parent of a trace id if:
        //   1. The `traceparent` has a trace id
        //   2. The `traceparent`'s trace id matches the incoming `trace_id`
        //
        // That means a `traceparent` is never considered the parent
        // of an empty `trace_id`, and a `trace_id` is never the child
        // of an empty `traceparent`

        self.traceparent.trace_id.is_some() && self.traceparent.trace_id == trace_id
    }
}

thread_local! {
    static ACTIVE_TRACEPARENT: RefCell<Option<ActiveTraceparent>> = RefCell::new(None);
}

fn set_active_traceparent(traceparent: Option<ActiveTraceparent>) -> Option<ActiveTraceparent> {
    ACTIVE_TRACEPARENT.with(|slot| {
        let mut slot = slot.borrow_mut();

        mem::replace(&mut *slot, traceparent)
    })
}

fn get_active_traceparent() -> Option<ActiveTraceparent> {
    ACTIVE_TRACEPARENT.with(|slot| slot.borrow().clone())
}

/**
A [`Ctxt`] that synchronizes [`Traceparent`]s with an underlying ambient context.

The trace context is shared by all instances of `TraceparentCtxt`, and any calls to [`Traceparent::current`] and [`Traceparent::push`].
*/
#[derive(Debug, Clone, Copy)]
pub struct TraceparentCtxt<C = Empty> {
    inner: C,
}

/**
The [`emit::Ctxt::Frame`] used by [`TraceparentCtxt`].
*/
#[derive(Debug, Clone)]
pub struct TraceparentCtxtFrame<F = Empty> {
    inner: F,
    active: bool,
    slot: Option<ActiveTraceparent>,
}

/**
The [`emit::Ctxt::Current`] used by [`TraceparentCtxt`].
*/
pub struct TraceparentCtxtProps<P: ?Sized> {
    inner: *const P,
    ctxt: SpanCtxt,
}

impl<P: Props + ?Sized> Props for TraceparentCtxtProps<P> {
    fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        mut for_each: F,
    ) -> ControlFlow<()> {
        self.ctxt.for_each(&mut for_each)?;

        // SAFETY: This type is only exposed for arbitrarily short (`for<'a>`) lifetimes
        // so inner it's guaranteed to be valid for `'kv`, which must be shorter than its
        // original lifetime
        unsafe { &*self.inner }.for_each(for_each)
    }
}

impl<C> TraceparentCtxt<C> {
    /**
    Wrap the given context, synchronizing any [`emit::SpanCtxt`] with the current [`Traceparent`].
    */
    pub const fn new(inner: C) -> Self {
        TraceparentCtxt { inner }
    }
}

impl<C: Ctxt> Ctxt for TraceparentCtxt<C> {
    type Current = TraceparentCtxtProps<C::Current>;
    type Frame = TraceparentCtxtFrame<C::Frame>;

    fn with_current<R, F: FnOnce(&Self::Current) -> R>(&self, with: F) -> R {
        // Get the current traceparent and use it as the span context
        // if it's sampled
        let ctxt = get_active_traceparent()
            .and_then(|active| {
                if active.traceparent.trace_flags.is_sampled() {
                    Some(SpanCtxt::new(
                        active.traceparent.trace_id,
                        active.span_parent,
                        active.traceparent.span_id,
                    ))
                } else {
                    None
                }
            })
            .unwrap_or(SpanCtxt::empty());

        self.inner.with_current(|props| {
            let props = TraceparentCtxtProps {
                ctxt,
                inner: props as *const C::Current,
            };

            with(&props)
        })
    }

    fn open_root<P: Props>(&self, props: P) -> Self::Frame {
        let (slot, props) =
            incoming_traceparent(None::<fn(&SpanCtxt) -> bool>, props, TraceFlags::ALL);

        let inner = self.inner.open_root(props);

        TraceparentCtxtFrame {
            inner,
            active: true,
            slot,
        }
    }

    fn open_push<P: Props>(&self, props: P) -> Self::Frame {
        let (slot, props) =
            incoming_traceparent(None::<fn(&SpanCtxt) -> bool>, props, TraceFlags::ALL);

        let inner = self.inner.open_push(props);

        TraceparentCtxtFrame {
            inner,
            active: slot.is_some(),
            slot,
        }
    }

    fn open_disabled<P: Props>(&self, props: P) -> Self::Frame {
        let (slot, props) =
            incoming_traceparent(None::<fn(&SpanCtxt) -> bool>, props, TraceFlags::EMPTY);

        let inner = self.inner.open_disabled(props);

        TraceparentCtxtFrame {
            inner,
            active: slot.is_some(),
            slot,
        }
    }

    fn enter(&self, frame: &mut Self::Frame) {
        if frame.active {
            frame.slot = set_active_traceparent(frame.slot.take());
        }

        self.inner.enter(&mut frame.inner)
    }

    fn exit(&self, frame: &mut Self::Frame) {
        if frame.active {
            frame.slot = set_active_traceparent(frame.slot.take());
        }

        self.inner.exit(&mut frame.inner)
    }

    fn close(&self, frame: Self::Frame) {
        self.inner.close(frame.inner)
    }
}

fn incoming_traceparent(
    sampler: Option<impl Fn(&SpanCtxt) -> bool>,
    props: impl Props,
    trace_flags: TraceFlags,
) -> (Option<ActiveTraceparent>, impl Props) {
    let span_id = props.pull::<SpanId, _>(KEY_SPAN_ID);

    // Only consider props that carry a span id
    let Some(span_id) = span_id else {
        return (
            None,
            ExcludeTraceparentProps {
                check: false,
                inner: props,
            },
        );
    };

    let active = get_active_traceparent().filter(|active| active.traceparent.is_valid());

    // Only consider props if the span id has changed
    if Some(span_id)
        == active
            .as_ref()
            .and_then(|parent| parent.traceparent.span_id)
    {
        return (
            None,
            ExcludeTraceparentProps {
                check: false,
                inner: props,
            },
        );
    }

    // If we get this far then the props will be mapped to a traceparent

    let incoming = if let Some(active) = active {
        // The incoming context is for a child span
        //
        // Construct a traceparent from it with the same trace id and flags,
        // using the span id of the parent as the parent id of the incoming

        ActiveTraceparent {
            traceparent: Traceparent::new(
                active.traceparent.trace_id,
                Some(span_id),
                active.traceparent.trace_flags & trace_flags,
            ),
            tracestate: active.tracestate,
            span_parent: active.traceparent.span_id,
        }
    } else {
        // The incoming traceparent is for a root span

        let trace_id = props.pull::<TraceId, _>(KEY_TRACE_ID);

        // Run the sampler
        let trace_flags = if let Some(sampler) = sampler {
            // If the incoming flags don't allow sampling then don't bother running the sampler
            if trace_flags.is_sampled() && sampler(&SpanCtxt::new(trace_id, None, Some(span_id))) {
                // Sampled
                trace_flags & TraceFlags::SAMPLED
            } else {
                // Unsampled
                trace_flags & TraceFlags::EMPTY
            }
        } else {
            // There's no sampler; default to sampled
            trace_flags & TraceFlags::SAMPLED
        };

        ActiveTraceparent {
            traceparent: Traceparent::new(trace_id, Some(span_id), trace_flags),
            tracestate: Tracestate::empty(),
            span_parent: None,
        }
    };

    (
        Some(incoming),
        ExcludeTraceparentProps {
            check: true,
            inner: props,
        },
    )
}

struct ExcludeTraceparentProps<P> {
    check: bool,
    inner: P,
}

impl<P: Props> Props for ExcludeTraceparentProps<P> {
    fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        mut for_each: F,
    ) -> ControlFlow<()> {
        if !self.check {
            return self.inner.for_each(for_each);
        }

        self.inner.for_each(|key, value| match key.get() {
            // Properties that come from the traceparent context
            KEY_TRACE_ID | KEY_SPAN_ID | KEY_SPAN_PARENT => ControlFlow::Continue(()),
            // Properties to pass through to the underlying context
            _ => for_each(key, value),
        })
    }
}

/**
A filter that runs a sampler over traces as they're created and excludes events when the current trace context is unsampled.

This filter uses [`Traceparent::current`] and checks [`TraceFlags::is_sampled`] on the returned [`Traceparent`] to determine whether the current context is sampled or not.
*/
pub struct TraceparentFilter<S = fn(&SpanCtxt) -> bool> {
    sampler: Option<S>,
}

impl TraceparentFilter {
    /**
    Create a new distributed-trace-aware filter with no sampler.

    All incoming traces will be included.
    */
    pub const fn new() -> Self {
        TraceparentFilter { sampler: None }
    }
}

impl<S> TraceparentFilter<S> {
    /**
    Create a new distributed-trace-aware filter with a sampler.

    The sampler will run at the start of the root span of each trace to determine whether to include it.
    If the sampler returns `true`, the trace and any events produced within it will be emitted.
    If the sampler returns `false`, the trace and any events produced within it will be discarded.
    */
    pub const fn new_with_sampler(sampler: S) -> Self {
        TraceparentFilter {
            sampler: Some(sampler),
        }
    }
}

impl<S: Fn(&SpanCtxt) -> bool> Filter for TraceparentFilter<S> {
    fn matches<E: ToEvent>(&self, evt: E) -> bool {
        let evt = evt.to_event();

        // If the event is a span then run the sampler over its incoming traceparent
        if emit::kind::is_span_filter().matches(&evt) {
            if let (Some(incoming), _) =
                incoming_traceparent(self.sampler.as_ref(), evt.props(), TraceFlags::SAMPLED)
            {
                return incoming.traceparent.trace_flags().is_sampled();
            }
        }

        // If the event is not a span then include it
        true
    }
}

/**
A filter that matches events in a sampled trace.
*/
pub struct InSampledTraceFilter {
    match_events_outside_traces: bool,
}

impl Filter for InSampledTraceFilter {
    fn matches<E: ToEvent>(&self, _: E) -> bool {
        if let Some(active) = get_active_traceparent() {
            active.traceparent.trace_flags().is_sampled()
        } else {
            self.match_events_outside_traces
        }
    }
}

impl InSampledTraceFilter {
    /**
    Create a filter that matches events in sampled traces.

    The `match_events_outside_traces` parameter determines whether events outside of any trace are matched or discarded.
    */
    pub const fn new(match_events_outside_traces: bool) -> Self {
        InSampledTraceFilter {
            match_events_outside_traces,
        }
    }
}

/**
Create a filter that matches events in sampled traces.

The `match_events_outside_traces` parameter determines whether events outside of any trace are matched or discarded.
*/
pub const fn in_sampled_trace_filter(match_events_outside_traces: bool) -> InSampledTraceFilter {
    InSampledTraceFilter::new(match_events_outside_traces)
}

#[cfg(test)]
mod tests {
    use super::*;

    use emit::{
        and::And,
        emitter, filter,
        platform::{
            rand_rng::RandRng, system_clock::SystemClock, thread_local_ctxt::ThreadLocalCtxt,
        },
        runtime::Runtime,
        Empty, Rng,
    };

    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn traceparent_roundtrip() {
        let traceparent = Traceparent::new(None, None, TraceFlags::EMPTY);

        assert_eq!(
            "00-00000000000000000000000000000000-0000000000000000-00",
            traceparent.to_string()
        );

        assert_eq!(
            traceparent,
            Traceparent::try_from_str(&traceparent.to_string()).unwrap()
        );

        let rng = RandRng::new();
        for _ in 0..1_000 {
            let trace_id = TraceId::random(&rng);
            let span_id = SpanId::random(&rng);
            let trace_flags = TraceFlags::from_u8(rng.gen_u64().unwrap() as u8);

            let traceparent = Traceparent::new(trace_id, span_id, trace_flags);
            let formatted = traceparent.to_string();

            assert_eq!(
                Some(traceparent),
                Traceparent::try_from_str(&formatted).ok(),
                "{traceparent:?} ({formatted}) did not roundtrip"
            );
        }
    }

    #[test]
    fn traceparent_parse_invalid() {
        for case in [
            "",
            "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-010",
            "00 4bf92f3577b34da6a3ce929d0e0e4736 00f067aa0ba902b7 01",
            "0x-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01",
            "00-4bf92f3577b34da6a3ce929d0e0e473x-00f067aa0ba902b7-01",
            "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902bx-01",
            "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-0x",
        ] {
            assert!(Traceparent::try_from_str(case).is_err());
        }
    }

    #[test]
    fn traceparent_set_current_sampled() {
        let rng = RandRng::new();

        assert_eq!(None, Traceparent::current().trace_id());
        assert_eq!(None, Traceparent::current().span_id());
        assert_eq!(TraceFlags::SAMPLED, *Traceparent::current().trace_flags());

        let traceparent = Traceparent::new(
            TraceId::random(&rng),
            SpanId::random(&rng),
            TraceFlags::SAMPLED,
        );

        let frame = traceparent.push();

        assert_eq!(None, Traceparent::current().trace_id());
        assert_eq!(None, Traceparent::current().span_id());
        assert_eq!(TraceFlags::SAMPLED, *Traceparent::current().trace_flags());

        frame.call(|| {
            assert_eq!(traceparent, Traceparent::current());

            // Any `TraceparentCtxt` should observe the current trace context
            let emit_ctxt = SpanCtxt::current(TraceparentCtxt::new(Empty));

            assert_eq!(traceparent.trace_id(), emit_ctxt.trace_id());
            assert_eq!(traceparent.span_id(), emit_ctxt.span_id());
            assert!(emit_ctxt.span_parent().is_none());
        });

        assert_eq!(None, Traceparent::current().trace_id());
        assert_eq!(None, Traceparent::current().span_id());
        assert_eq!(TraceFlags::SAMPLED, *Traceparent::current().trace_flags());
    }

    #[test]
    fn traceparent_set_current_unsampled() {
        let rng = RandRng::new();

        let traceparent = Traceparent::new(
            TraceId::random(&rng),
            SpanId::random(&rng),
            TraceFlags::EMPTY,
        );

        traceparent.push().call(|| {
            // The `emit` context should be empty because the active traceparent is unsampled
            let emit_ctxt = SpanCtxt::current(TraceparentCtxt::new(Empty));

            assert!(emit_ctxt.trace_id().is_none());
            assert!(emit_ctxt.span_id().is_none());
            assert!(emit_ctxt.span_parent().is_none());
        });

        assert_eq!(TraceFlags::SAMPLED, *Traceparent::current().trace_flags());
    }

    #[test]
    fn traceparent_ctxt() {
        let rng = RandRng::new();
        let ctxt = TraceparentCtxt::new(ThreadLocalCtxt::new());

        let span_ctxt_1 = SpanCtxt::current(&ctxt).new_child(&rng);

        span_ctxt_1.push(&ctxt).call(|| {
            let span_ctxt_2 = SpanCtxt::current(&ctxt).new_child(&rng);

            span_ctxt_2.push(&ctxt).call(|| {
                let traceparent = Traceparent::current();

                let span_ctxt_3 = SpanCtxt::current(&ctxt);

                assert_eq!(span_ctxt_2, span_ctxt_3);

                assert_eq!(
                    span_ctxt_1.trace_id().unwrap(),
                    span_ctxt_2.trace_id().unwrap()
                );
                assert_eq!(
                    span_ctxt_1.span_id().unwrap(),
                    span_ctxt_2.span_parent().unwrap()
                );
                assert!(span_ctxt_2.span_id().is_some());

                assert_eq!(
                    traceparent.trace_id().unwrap(),
                    span_ctxt_2.trace_id().unwrap()
                );
                assert_eq!(
                    traceparent.span_id().unwrap(),
                    span_ctxt_2.span_id().unwrap()
                );
                assert!(traceparent.trace_flags().is_sampled());
            });
        });
    }

    #[test]
    fn traceparent_ctxt_ignores_invalid_parent() {
        let rng = RandRng::new();
        let ctxt = TraceparentCtxt::new(ThreadLocalCtxt::new());

        Traceparent::new(None, None, TraceFlags::EMPTY)
            .push()
            .call(|| {
                let span_ctxt_1 = SpanCtxt::current(&ctxt).new_child(&rng);

                span_ctxt_1.push(&ctxt).call(|| {
                    let span_ctxt_2 = SpanCtxt::current(&ctxt);

                    assert_eq!(span_ctxt_1, span_ctxt_2);
                });
            });
    }

    #[test]
    fn traceparent_ctxt_ignores_mismatched_trace_ids() {
        let rng = RandRng::new();
        let ctxt = TraceparentCtxt::new(ThreadLocalCtxt::new());

        let span_ctxt_1 = SpanCtxt::current(&ctxt).new_child(&rng);

        span_ctxt_1.push(&ctxt).call(|| {
            let span_ctxt_2 = SpanCtxt::new_root(&rng);

            span_ctxt_2.push(&ctxt).call(|| {
                let traceparent = Traceparent::current();

                let span_ctxt_3 = SpanCtxt::current(&ctxt);

                // The trace id from `span_ctxt_2` is ignored, as well as its span parent
                assert_ne!(span_ctxt_2.trace_id(), span_ctxt_3.trace_id());

                assert_eq!(span_ctxt_2.span_id(), span_ctxt_3.span_id());

                assert_eq!(
                    span_ctxt_1.trace_id().unwrap(),
                    span_ctxt_3.trace_id().unwrap()
                );
                assert_eq!(
                    span_ctxt_1.span_id().unwrap(),
                    span_ctxt_3.span_parent().unwrap()
                );
                assert!(span_ctxt_3.span_id().is_some());

                assert_eq!(
                    traceparent.trace_id().unwrap(),
                    span_ctxt_3.trace_id().unwrap()
                );
                assert_eq!(
                    traceparent.span_id().unwrap(),
                    span_ctxt_3.span_id().unwrap()
                );
                assert!(traceparent.trace_flags().is_sampled());
            });
        });
    }

    #[test]
    fn traceparent_ctxt_unsets_on_empty_root() {
        let rng = RandRng::new();
        let ctxt = TraceparentCtxt::new(ThreadLocalCtxt::new());

        let span_ctxt_1 = SpanCtxt::current(&ctxt).new_child(&rng);

        span_ctxt_1.push(&ctxt).call(|| {
            let traceparent = Traceparent::current();

            assert_ne!(Traceparent::empty(), traceparent);

            let frame = emit::Frame::root(&ctxt, emit::Empty);

            frame.call(|| {
                let traceparent = Traceparent::current();
                let span_ctxt = SpanCtxt::current(&ctxt);

                // Since there's no active span context, there
                // also should be no active traceparent
                assert_eq!(Traceparent::empty(), traceparent);

                assert!(span_ctxt.trace_id().is_none());
                assert!(span_ctxt.span_id().is_none());
                assert!(span_ctxt.span_parent().is_none());
            })
        });
    }

    #[test]
    #[cfg(not(target_os = "wasi"))]
    fn traceparent_across_threads() {
        let rng = RandRng::new();

        let traceparent = Traceparent::new(
            TraceId::random(&rng),
            SpanId::random(&rng),
            TraceFlags::SAMPLED,
        );

        let frame = traceparent.push();

        std::thread::spawn(move || {
            frame.call(|| {
                let current = Traceparent::current();

                assert_eq!(traceparent, current);
            })
        })
        .join()
        .unwrap();
    }

    #[test]
    #[cfg(not(target_os = "wasi"))]
    fn traceparent_ctxt_across_threads() {
        let rng = RandRng::new();
        let ctxt = TraceparentCtxt::new(ThreadLocalCtxt::new());

        let span_ctxt = SpanCtxt::current(&ctxt).new_child(&rng).push(ctxt.clone());

        std::thread::spawn(move || {
            span_ctxt.call(|| {
                let traceparent = Traceparent::current();
                let span_ctxt = SpanCtxt::current(&ctxt);

                assert_eq!(span_ctxt.trace_id(), traceparent.trace_id());
                assert_eq!(span_ctxt.span_id(), traceparent.span_id());
                assert_eq!(TraceFlags::SAMPLED, *traceparent.trace_flags());
            })
        })
        .join()
        .unwrap();
    }

    #[test]
    fn traceparent_span() {
        static RT: Runtime<
            Empty,
            And<filter::FromFn, TraceparentFilter>,
            TraceparentCtxt<ThreadLocalCtxt>,
            SystemClock,
            RandRng,
        > = Runtime::build(
            Empty,
            And::new(
                filter::FromFn::new(|evt| evt.mdl() != emit::path!("unsampled")),
                TraceparentFilter::new(),
            ),
            TraceparentCtxt::new(ThreadLocalCtxt::shared()),
            SystemClock::new(),
            RandRng::new(),
        );

        #[emit::span(rt: RT, mdl: emit::path!("unsampled"), "a")]
        fn unsampled() {
            unsampled_sampled();
        }

        #[emit::span(rt: RT, mdl: emit::path!("sampled"), "a")]
        fn unsampled_sampled() {
            let ctxt = SpanCtxt::current(RT.ctxt());
            let traceparent = Traceparent::current();

            assert!(ctxt.trace_id().is_none());
            assert!(ctxt.span_id().is_none());

            assert!(traceparent.trace_id().is_some());
            assert!(traceparent.span_id().is_some());
            assert!(!traceparent.trace_flags().is_sampled());
        }

        #[emit::span(rt: RT, mdl: emit::path!("sampled"), "a")]
        fn sampled() {
            let ctxt = SpanCtxt::current(RT.ctxt());
            let traceparent = Traceparent::current();

            assert!(ctxt.trace_id().is_some());
            assert!(ctxt.span_id().is_some());

            assert_eq!(traceparent.trace_id(), ctxt.trace_id());
            assert_eq!(traceparent.span_id(), ctxt.span_id());
            assert!(traceparent.trace_flags().is_sampled());
        }

        unsampled();
        sampled();
    }

    #[test]
    fn traceparent_span_sampler() {
        static EMITTER_SPAN_CALLS: AtomicUsize = AtomicUsize::new(0);
        static EMITTER_EVENT_CALLS: AtomicUsize = AtomicUsize::new(0);
        static SAMPLER_CALLS: AtomicUsize = AtomicUsize::new(0);

        static RT: Runtime<
            emitter::FromFn,
            TraceparentFilter,
            TraceparentCtxt<ThreadLocalCtxt>,
            SystemClock,
            RandRng,
        > = Runtime::build(
            emitter::FromFn::new(|evt| {
                if evt.props().pull("evt_kind") == Some(emit::Kind::Span) {
                    EMITTER_SPAN_CALLS.fetch_add(1, Ordering::Relaxed);
                } else {
                    EMITTER_EVENT_CALLS.fetch_add(1, Ordering::Relaxed);
                }
            }),
            TraceparentFilter::new_with_sampler({
                move |_| SAMPLER_CALLS.fetch_add(1, Ordering::Relaxed) % 10 == 0
            }),
            TraceparentCtxt::new(ThreadLocalCtxt::shared()),
            SystemClock::new(),
            RandRng::new(),
        );

        #[emit::span(rt: RT, "outer")]
        fn outer() {
            inner()
        }

        #[emit::span(rt: RT, "inner")]
        fn inner() {
            emit::emit!(rt: RT, "event");
        }

        for _ in 0..30 {
            outer();
        }

        // `outer` is called 3 times, which also calls `inner` 3 times
        assert_eq!(6, EMITTER_SPAN_CALLS.load(Ordering::Relaxed));

        // all events are included, regardless of sampling
        assert_eq!(30, EMITTER_EVENT_CALLS.load(Ordering::Relaxed));
    }

    #[test]
    fn tracestate_get_set() {
        assert_eq!("", Tracestate::current().get());
        assert_eq!(Tracestate::empty(), Tracestate::current());

        let state = Tracestate::new_raw("a=1");

        state.push().call(|| {
            assert_eq!("a=1", Tracestate::current().get());
            assert_eq!(state, Tracestate::current());
        });
    }

    #[test]
    fn in_trace_filter_includes_or_excludes_traced_events() {
        Traceparent::new(
            TraceId::from_u128(1),
            SpanId::from_u64(1),
            TraceFlags::SAMPLED,
        )
        .push()
        .call(|| {
            assert!(in_sampled_trace_filter(true).matches(emit::evt!("event")));
        });

        Traceparent::new(
            TraceId::from_u128(1),
            SpanId::from_u64(1),
            TraceFlags::EMPTY,
        )
        .push()
        .call(|| {
            assert!(!in_sampled_trace_filter(true).matches(emit::evt!("event")));
        });
    }

    #[test]
    fn in_trace_filter_includes_or_excludes_untraced_events() {
        assert!(in_sampled_trace_filter(true).matches(emit::evt!("event")));
        assert!(!in_sampled_trace_filter(false).matches(emit::evt!("event")));
    }
}
