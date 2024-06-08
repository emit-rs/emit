/*!
The [`Span`] type.

When your application executes key operations, you can emit span events that dover the time they were active. Any other operations involved in that execution, or any other events emitted during it, will be correlated through identifiers to form a hierarchical call tree. Together, these events form a trace, which in distributed systems can involve operations executed by other services. Traces are a useful way to build a picture of service dependencies in distributed applications, and to identify performance problems across them.

`emit` supports tracing operations through attribute macros on functions. These macros use the same syntax as those for emitting regular events:

```
# #[cfg(not(feature = "std"))] fn main() {}
# #[cfg(feature = "std")] fn main() {
# use std::{thread, time::Duration};
#[emit::span("wait a bit", sleep_ms)]
fn wait_a_bit(sleep_ms: u64) {
    thread::sleep(Duration::from_millis(sleep_ms))
}

wait_a_bit(1200);
# }
```

```text
Event {
    module: "my_app",
    tpl: "wait a bit",
    extent: Some(
        "2024-04-27T22:40:24.112859000Z".."2024-04-27T22:40:25.318273000Z",
    ),
    props: {
        "event_kind": span,
        "span_name": "wait a bit",
        "span_id": 71ea734fcbb4dc41,
        "trace_id": 6d6bb9c23a5f76e7185fb3957c2f5527,
        "sleep_ms": 1200,
    },
}
```

When the annotated function returns, a span event for its execution is emitted. The extent of a span event is a range, where the start is the time the function began executing, and the end is the time the function returned.

On nightly compilers, the same attributes can also be applied to blocks instead of functions.

Asynchronous functions are also supported:

```
# use std::{thread, time::Duration};
# fn main() {}
# async fn sleep(_: Duration) {}
# #[cfg(feature = "std")]
# async fn main_async() {
#[emit::span("wait a bit", sleep_ms)]
async fn wait_a_bit(sleep_ms: u64) {
    sleep(Duration::from_millis(sleep_ms)).await
}

wait_a_bit(1200).await;
# }
```

Span events may also be created manually:

```
# #[cfg(not(feature = "std"))] fn main() {}
# #[cfg(feature = "std")] fn main() {
# use std::{time::Duration, thread};
use emit::Filter;

let sleep_ms = 1200;

let rt = emit::runtime::shared();

// Create a span
let mut span = emit::Span::filtered_new(
    |span| rt.filter().matches(&span),
    "my_app",
    emit::Timer::start(rt.clock()),
    "wait a bit",
    emit::span::SpanCtxt::current(rt.ctxt()).new_child(rt.rng()),
    emit::Empty,
    |span| emit::emit!(event: span),
);

// Push the span onto the current context
let frame = span.push_ctxt(
    rt.ctxt(),
    emit::props! {
        sleep_ms,
    },
);

// Execute some operation within the frame
frame.call(move || {
    // Your code goes here
    thread::sleep(Duration::from_millis(sleep_ms));

    // Make sure you complete the span in the frame.
    // This is especially important for futures, otherwise the span may
    // complete before the future does
    span.complete();
});
# }
```

Spans can also be emitted directly as regular events:

```
# #[cfg(not(feature = "std"))] fn main() {}
# #[cfg(feature = "std")] fn main() {
# use std::{thread, time::Duration, panic};
use emit::{well_known::EVENT_KIND_SPAN, Filter};

let sleep_ms = 1200;

let rt = emit::runtime::shared();

let timer = emit::Timer::start(rt.clock());
let props = emit::span::SpanCtxt::current(rt.ctxt()).new_child(rt.rng());

// Check whether the span should be created or not
let frame = if rt.filter().matches(&emit::event! {
    extent: timer.start_timestamp(),
    props,
    "wait a bit",
    event_kind: EVENT_KIND_SPAN,
}) {
    emit::Frame::push(Some(rt.ctxt()), props)
} else {
    emit::Frame::current(None)
};

// Execute some operation within the frame
frame.call(|| {
    let r = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        // Your code goes here
        thread::sleep(Duration::from_millis(sleep_ms));
    }));

    // Emit the span event at the end of the scope
    // We do this regardless of panics
    emit::emit!(
        extent: timer,
        "wait a bit",
        event_kind: EVENT_KIND_SPAN,
        sleep_ms,
    );

    match r {
        Ok(r) => r,
        Err(r) => panic::resume_unwind(r),
    }
});
# }
```

# Data model

The data model of spans is an extension of `emit`'s events. Span events include the following well-known properties:

- `event_kind`: with a value of `"span"` to indicate that the event is a span.
- `span_name`: a name for the operation the span represents. This defaults to the template.
- `span_id`: an identifier for this specific invocation of the operation.
- `parent_id`: the `span_id` of the operation that invoked this one.
- `trace_id`: an identifier shared by all events in a distributed trace. A `trace_id` is assigned by the first operation.

# Contextual properties

Properties added to the span macros are added to an ambient context and automatically included on any events emitted within that operation:

```
# #[cfg(not(feature = "std"))] fn main() {}
# #[cfg(feature = "std")] fn main() {
# use std::{thread, time::Duration};
#[emit::span("wait a bit", sleep_ms)]
fn wait_a_bit(sleep_ms: u64) {
    thread::sleep(Duration::from_millis(sleep_ms));

    emit::emit!("waiting a bit longer");

    thread::sleep(Duration::from_millis(sleep_ms));
}
# }
```

```text
Event {
    module: "my_app",
    tpl: "waiting a bit longer",
    extent: Some(
        "2024-04-27T22:47:34.780288000Z",
    ),
    props: {
        "trace_id": d2a5e592546010570472ac6e6457c086,
        "sleep_ms": 1200,
        "span_id": ee9fde093b6efd78,
    },
}
Event {
    module: "my_app",
    tpl: "wait a bit",
    extent: Some(
        "2024-04-27T22:47:33.574839000Z".."2024-04-27T22:47:35.985844000Z",
    ),
    props: {
        "event_kind": span,
        "span_name": "wait a bit",
        "trace_id": d2a5e592546010570472ac6e6457c086,
        "sleep_ms": 1200,
        "span_id": ee9fde093b6efd78,
    },
}
```

Any operations started within a span will inherit its identifiers:

```
# #[cfg(not(feature = "std"))] fn main() {}
# #[cfg(feature = "std")] fn main() {
# use std::{thread, time::Duration};
#[emit::span("outer span", sleep_ms)]
fn outer_span(sleep_ms: u64) {
    thread::sleep(Duration::from_millis(sleep_ms));

    inner_span(sleep_ms / 2);
}

#[emit::span("inner span", sleep_ms)]
fn inner_span(sleep_ms: u64) {
    thread::sleep(Duration::from_millis(sleep_ms));
}
# }
```

```text
Event {
    module: "my_app",
    tpl: "inner span",
    extent: Some(
        "2024-04-27T22:50:50.385706000Z".."2024-04-27T22:50:50.994509000Z",
    ),
    props: {
        "event_kind": span,
        "span_name": "inner span",
        "trace_id": 12b2fde225aebfa6758ede9cac81bf4d,
        "span_parent": 23995f85b4610391,
        "sleep_ms": 600,
        "span_id": fc8ed8f3a980609c,
    },
}
Event {
    module: "my_app",
    tpl: "outer span",
    extent: Some(
        "2024-04-27T22:50:49.180025000Z".."2024-04-27T22:50:50.994797000Z",
    ),
    props: {
        "event_kind": span,
        "span_name": "outer span",
        "sleep_ms": 1200,
        "span_id": 23995f85b4610391,
        "trace_id": 12b2fde225aebfa6758ede9cac81bf4d,
    },
}
```

Notice the `span_parent` of `inner_span` is the same as the `span_id` of `outer_span`. That's because `inner_span` was called within the execution of `outer_span`.

# Propagating span context across threads

Ambient span properties are not shared across threads by default. This context needs to be fetched and sent across threads manually:

```
# #[cfg(not(feature = "std"))] fn main() {}
# #[cfg(feature = "std")] fn main() {
# use std::thread;
# fn my_operation() {}
thread::spawn({
    let ctxt = emit::Frame::current(emit::runtime::shared().ctxt());

    move || ctxt.call(|| {
        // Your code goes here
    })
});
# }
```

This same process is also needed for async code that involves thread spawning:

```
# mod tokio { pub fn spawn(_: impl std::future::Future) {} }
# #[cfg(not(feature = "std"))] fn main() {}
# #[cfg(feature = "std")] fn main() {
tokio::spawn(
    emit::Frame::current(emit::runtime::shared().ctxt()).in_future(async {
        // Your code goes here
    }),
);
# }
```

Async functions that simply migrate across threads in work-stealing runtimes don't need any manual work to keep their context across those threads.

# Propagating span context across services

`emit` doesn't implement any distributed trace propagation itself. This is the responsibility of end-users through their web framework and clients to manage.

When an incoming request arrives, you can parse the trace and span ids from its traceparent header and push them onto the current context:

```
# #[cfg(not(feature = "std"))] fn main() {}
# #[cfg(feature = "std")] fn main() {
// Parsed from a traceparent header
let trace_id = "12b2fde225aebfa6758ede9cac81bf4d";
let span_id = "23995f85b4610391";

let frame = emit::Frame::push(emit::runtime::shared().ctxt(), emit::props! {
    trace_id,
    span_id,
});

frame.call(handle_request);

#[emit::span("incoming request")]
fn handle_request() {
    // Your code goes here
}
# }
```

```text
Event {
    module: "my_app",
    tpl: "incoming request",
    extent: Some(
        "2024-04-29T05:37:05.278488400Z".."2024-04-29T05:37:05.278636100Z",
    ),
    props: {
        "event_kind": span,
        "span_name": "incoming request",
        "span_parent": 23995f85b4610391,
        "trace_id": 12b2fde225aebfa6758ede9cac81bf4d,
        "span_id": 641a578cc05c9db2,
    },
}
```

This pattern of pushing the incoming traceparent onto the context and then immediately calling a span annotated function ensures the `span_id` parsed from the traceparent becomes the `span_parent` in the events emitted by your application, without emitting a span event for the calling service itself.

When making outbound requests, you can pull the current trace and span ids from the current context and format them into a traceparent header:

```
# #[cfg(not(feature = "std"))] fn main() {}
# #[cfg(feature = "std")] fn main() {
use emit::{well_known::{KEY_SPAN_ID, KEY_TRACE_ID}, Ctxt, Props};

let (trace_id, span_id) = emit::runtime::shared().ctxt().with_current(|props| {
    (
        props.pull::<emit::span::TraceId, _>(KEY_TRACE_ID),
        props.pull::<emit::span::SpanId, _>(KEY_SPAN_ID),
    )
});

if let (Some(trace_id), Some(span_id)) = (trace_id, span_id) {
    let traceparent = format!("00-{trace_id}-{span_id}-00");

    // Push the traceparent header onto the request
}
# }
```

# Completing spans manually

The `arg` control parameter can be applied to span macros to bind an identifier in the body of the annotated function for the [`Span`] that's created for it. This span can be completed manually, changing properties of the span along the way:

```
# #[cfg(not(feature = "std"))] fn main() {}
# #[cfg(feature = "std")] fn main() {
# use std::{thread, time::Duration};
#[emit::span(arg: span, "wait a bit", sleep_ms)]
fn wait_a_bit(sleep_ms: u64) {
    thread::sleep(Duration::from_millis(sleep_ms));

    if sleep_ms > 500 {
        span.complete_with(|span| {
            emit::warn!(
                event: span,
                when: emit::filter::always(),
                "wait a bit took too long",
            );
        });
    }
}

wait_a_bit(100);
wait_a_bit(1200);
# }
```

```text
Event {
    module: "my_app",
    tpl: "wait a bit",
    extent: Some(
        "2024-04-28T21:12:20.497595000Z".."2024-04-28T21:12:20.603108000Z",
    ),
    props: {
        "event_kind": span,
        "span_name": "wait a bit",
        "trace_id": 5b9ab977a530dfa782eedd6db08fdb66,
        "sleep_ms": 100,
        "span_id": 6f21f5ddc707f730,
    },
}
Event {
    module: "my_app",
    tpl: "wait a bit took too long",
    extent: Some(
        "2024-04-28T21:12:20.603916000Z".."2024-04-28T21:12:21.808502000Z",
    ),
    props: {
        "event_kind": span,
        "span_name": "wait a bit",
        "lvl": warn,
        "trace_id": 9abad69ac8bf6d6ef6ccde8453226aa3,
        "sleep_ms": 1200,
        "span_id": c63632332de89ac3,
    },
}
```

Take care when completing spans manually that they always match the configured filter. This can be done using the `when` control parameter like in the above example. If a span is created it _must_ be emitted, otherwise the resulting trace will be incomplete.
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
    value::FromValue,
    well_known::{KEY_EVENT_KIND, KEY_SPAN_ID, KEY_SPAN_NAME, KEY_SPAN_PARENT, KEY_TRACE_ID},
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
#[derive(Clone, Copy, PartialEq, Eq)]
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
#[derive(Clone, Copy, PartialEq, Eq)]
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
    hex: [u8; N],
    idx: usize,
}

impl<const N: usize> Buffer<N> {
    fn new() -> Self {
        Buffer {
            hex: [0; N],
            idx: 0,
        }
    }

    fn buffer(&mut self, hex: impl fmt::Display) -> Result<&[u8], ParseIdError> {
        use fmt::Write as _;

        self.idx = 0;

        write!(self, "{}", hex).map_err(|_| ParseIdError {})?;

        Ok(&self.hex[..self.idx])
    }
}

impl<const N: usize> fmt::Write for Buffer<N> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let s = s.as_bytes();
        let next_idx = self.idx + s.len();

        if next_idx <= self.hex.len() {
            self.hex[self.idx..next_idx].copy_from_slice(s);
            self.idx = next_idx;

            Ok(())
        } else {
            Err(fmt::Error)
        }
    }
}

/**
An active span in a distributed trace.

This type manages the lifecycle of a span, which includes:

1. Call [`SpanCtxt::current`] and [`SpanCtxt::new_child`] to generate a set of identifiers for the new span.
2. Call [`Timer::start`] to begin a timer for the new span.
3. Call [`Span::filtered_new`] with the [`Timer`] and generated [`SpanCtxt`] to produce a [`Span`].
4. Call [`Span::push_ctxt`] to add the trace id, span id, and span parent id to the ambient context.
5. Run some code.
6. Call [`Span::complete_with`], or just drop the [`Span`] to complete it, emitting a [`SpanEvent`] for its execution.
*/
pub struct Span<'a, C: Clock, P: Props, F: FnOnce(SpanEvent<'a, P>)> {
    value: Option<ActiveSpanEvent<'a, C, P>>,
    on_drop: Option<F>,
}

/**
A diagnostic event that represents a span in a distributed trace.

Spans are an extension of [`Event`]s that explicitly take the well-known properties that signal an event as being a span. See the [`mod@crate::span`] module for details.

A `SpanEvent` can be converted into an [`Event`] through its [`ToEvent`] implemenation, or passed directly to a [`crate::Emitter`] to emit it.
*/
pub struct SpanEvent<'a, P: Props> {
    module: Path<'a>,
    extent: Option<Extent>,
    ctxt: SpanCtxt,
    name: Str<'a>,
    props: P,
}

struct ActiveSpanEvent<'a, C: Clock, P: Props> {
    module: Path<'a>,
    timer: Timer<C>,
    ctxt: SpanCtxt,
    name: Str<'a>,
    props: P,
    include_ctxt: bool,
}

impl<'a, C: Clock, P: Props> ActiveSpanEvent<'a, C, P> {
    fn complete(self) -> SpanEvent<'a, P> {
        if self.include_ctxt {
            SpanEvent::new(self.module, self.timer, self.ctxt, self.name, self.props)
        } else {
            SpanEvent::new(
                self.module,
                self.timer,
                SpanCtxt::empty(),
                self.name,
                self.props,
            )
        }
    }
}

impl<'a, P: Props> SpanEvent<'a, P> {
    /**
    Create a new span event from its parts.

    Each span consists of:

    - `module`: The module that executed the operation the span is tracking.
    - `extent`: The time the operation spent executing.
    - `ctxt`: The [`TraceId`] and [`SpanId`] that identify the span.
    - `name`: The name of the operation the span is tracking.
    - `props`: Additional [`Props`] to associate with the span.
    */
    pub fn new(
        module: impl Into<Path<'a>>,
        extent: impl ToExtent,
        ctxt: SpanCtxt,
        name: impl Into<Str<'a>>,
        props: P,
    ) -> Self {
        SpanEvent {
            module: module.into(),
            extent: extent.to_extent(),
            ctxt,
            name: name.into(),
            props,
        }
    }

    /**
    Get the module that executed the operation.
    */
    pub fn module(&self) -> &Path<'a> {
        &self.module
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
    Get the additional properties associated with the span.
    */
    pub fn props(&self) -> &P {
        &self.props
    }
}

impl<'a, P: Props> ToEvent for SpanEvent<'a, P> {
    type Props<'b> = &'b Self where Self: 'b;

    fn to_event<'b>(&'b self) -> Event<Self::Props<'b>> {
        // "{span_name} completed"
        const TEMPLATE: &'static [template::Part<'static>] = &[
            template::Part::hole("span_name"),
            template::Part::text(" completed"),
        ];

        Event::new(
            self.module.by_ref(),
            self.extent.clone(),
            Template::new(TEMPLATE),
            &self,
        )
    }
}

impl<'a, P: Props> Props for SpanEvent<'a, P> {
    fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        mut for_each: F,
    ) -> ControlFlow<()> {
        for_each(KEY_EVENT_KIND.to_str(), Kind::Span.to_value())?;
        for_each(KEY_SPAN_NAME.to_str(), self.name.to_value())?;

        self.ctxt.for_each(&mut for_each)?;
        self.props.for_each(&mut for_each)
    }
}

/**
The trace id, span id, and parent parent span id of a [`SpanEvent`].

These ids can be used to identify the distributed trace a span belongs to, and to identify the span itself within that trace.

The `SpanCtxt` for the currently executing span can be pulled from the ambient context with [`SpanCtxt::current`]. Once a `SpanCtxt` is constructed, a new child context can be generated by [`SpanCtxt::new_child`].

`SpanCtxt` implements [`Props`], so it can be pushed onto the ambient context through [`Frame::push`], or [`Span::push_ctxt`].
*/
#[derive(Debug, Clone, Copy)]
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

impl<'a, C: Clock, P: Props, F: FnOnce(SpanEvent<'a, P>)> Drop for Span<'a, C, P, F> {
    fn drop(&mut self) {
        if let (Some(value), Some(on_drop)) = (self.value.take(), self.on_drop.take()) {
            on_drop(value.complete())
        }
    }
}

impl<'a, C: Clock, P: Props, F: FnOnce(SpanEvent<'a, P>)> Span<'a, C, P, F> {
    /**
    Create a span for the given `ctxt`.

    The parameters to this method are:

    - `filter`: A sampler to determine whether the span should actually be created or not. If the filter doesn't match then a [`Span::disabled`] will be returned.
    - `module`: The name of the module executing the operation the span is tracking. This will become the [`SpanEvent::module`] on the resulting span event.
    - `timer`: A timer to determine the runtime of the executing operation.
    - `ctxt`: The trace id, span id, and span parent id for the span.
    - `event_props`: An additional set of properties. This will become the [`SpanEvent::props`] on the resulting span event. These properties won't be included in [`Span::push_ctxt`].
    - `default_complete`: A closure called on drop with a [`SpanEvent`] representing the operation the span was tracking.
    */
    pub fn filtered_new(
        filter: impl FnOnce(SpanEvent<&P>) -> bool,
        module: impl Into<Path<'a>>,
        timer: Timer<C>,
        name: impl Into<Str<'a>>,
        ctxt: SpanCtxt,
        event_props: P,
        default_complete: F,
    ) -> Self {
        let module = module.into();
        let name = name.into();

        if filter(SpanEvent::new(
            module.by_ref(),
            timer.start_timestamp(),
            ctxt,
            name.by_ref(),
            &event_props,
        )) {
            Span {
                value: Some(ActiveSpanEvent {
                    timer,
                    module,
                    ctxt,
                    name,
                    props: event_props,
                    include_ctxt: true,
                }),
                on_drop: Some(default_complete),
            }
        } else {
            Self::disabled()
        }
    }

    /**
    Create a new span, ignoring any sampling.

    This method is like [`Span::filtered_new`] with a filter that always returns `true`.
    */
    pub fn new(
        timer: Timer<C>,
        module: impl Into<Path<'a>>,
        name: impl Into<Str<'a>>,
        ctxt: SpanCtxt,
        event_props: P,
        default_complete: F,
    ) -> Self {
        Self::filtered_new(
            |_| true,
            module,
            timer,
            name,
            ctxt,
            event_props,
            default_complete,
        )
    }

    /**
    Create an empty span.

    A disabled span can be used exactly like an enabled one, except [`Span::push_ctxt`] won't push any properties, and [`Span::complete_with`] won't invoke the given closure.
    */
    pub fn disabled() -> Self {
        Span {
            value: None,
            on_drop: None,
        }
    }

    /**
    Whether the span will emit a [`SpanEvent`] or not.
    */
    pub fn is_enabled(&self) -> bool {
        self.value.is_some()
    }

    /**
    Get the module of the operation this span is tracking.
    */
    pub fn module(&self) -> Option<&Path<'a>> {
        self.value.as_ref().map(|value| &value.module)
    }

    /**
    Get the timer.
    */
    pub fn timer(&self) -> Option<&Timer<C>> {
        self.value.as_ref().map(|value| &value.timer)
    }

    /**
    Get the trace id, span id, and parent span id.
    */
    pub fn ctxt(&self) -> Option<&SpanCtxt> {
        self.value.as_ref().map(|value| &value.ctxt)
    }

    /**
    Get the name of the operation this span is tracking.
    */
    pub fn name(&self) -> Option<&Str<'a>> {
        self.value.as_ref().map(|value| &value.name)
    }

    /**
    Get the additional event properties associated with the span.

    These properties will be attached to the resulting [`SpanEvent`] when the span is completed.
    */
    pub fn props(&self) -> Option<&P> {
        self.value.as_ref().map(|value| &value.props)
    }

    /**
    Push the [`SpanCtxt`] onto the ambient context.

    If the span is enabled, then the trace id, span id, and parent span id will be pushed to the context. This ensures diagnostics emitted during the execution of this span are properly linked to it.

    If the span is disabled then this method is a no-op.
    */
    pub fn push_ctxt<T: Ctxt>(&mut self, ctxt: T, ctxt_props: impl Props) -> Frame<Option<T>> {
        if let Some(ref mut value) = self.value {
            value.include_ctxt = false;
        }

        if self.is_enabled() {
            Frame::push(Some(ctxt), self.ctxt().and_props(ctxt_props))
        } else {
            Frame::current(None)
        }
    }

    /**
    Complete the span, invoking the default completion closure.

    If the span is disabled then this method is a no-op.
    */
    pub fn complete(self) {
        drop(self);
    }

    /**
    Complete the span, invoking the given closure instead of the default one.

    If the span is disabled then the `complete` closure won't be called.
    */
    pub fn complete_with(mut self, complete: impl FnOnce(SpanEvent<'a, P>)) -> bool {
        if let Some(value) = self.value.take() {
            complete(value.complete());
            true
        } else {
            false
        }
    }
}

impl<'a, C: Clock, P: Props, F: FnOnce(SpanEvent<'a, P>)> ToExtent for Span<'a, C, P, F> {
    fn to_extent(&self) -> Option<Extent> {
        self.timer().to_extent()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
