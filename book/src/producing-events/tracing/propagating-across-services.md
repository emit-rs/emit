# Propagating span context across services

Span context can be used in distributed applications to correlate their operations together. When services call eachother, they _propagate_ their span context to the callee so it can act as if it were part of that context instead of generating its own. That just makes sure trace ids and span parents line up.

Propagation and [sampling](./sampling.md) are tied together. If a service decides not to sample a given trace then it _must_ propagate that decision to downstream services. Otherwise you'll end up with a broken trace.

`emit` supports span context propagation via [W3C traceparents](https://www.w3.org/TR/trace-context/) using [`emit_traceparent`](https://docs.rs/emit_traceparent/0.11.10/emit_traceparent/) or the OpenTelemetry SDK.

## Using `emit_traceparent` for propagation

[`emit_traceparent`](https://docs.rs/emit_traceparent/0.11.10/emit_traceparent/) is a library that implements trace sampling and propagation.

When an incoming request arrives, you can push the incoming traceparent onto the current context:

```rust
# extern crate emit;
# extern crate emit_traceparent;
// 1. Pull the incoming traceparent
//    If the request doesn't specify one then use an empty sampled context
let traceparent = emit_traceparent::Traceparent::try_from_str("00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01")
    .unwrap_or_else(|_| emit_traceparent::Traceparent::current());

// 2. Push the traceparent onto the context and execute your handler within it
traceparent.push().call(handle_request);

#[emit::span("incoming request")]
fn handle_request() {
    // Your code goes here
}
```

```text
Event {
    mdl: "my_app",
    tpl: "incoming request",
    extent: Some(
        "2024-10-16T10:04:24.783410472Z".."2024-10-16T10:04:24.783463852Z",
    ),
    props: {
        "evt_kind": span,
        "span_name": "incoming request",
        "trace_id": 4bf92f3577b34da6a3ce929d0e0e4736,
        "span_id": d6ae3ee046c529d9,
        "span_parent": 00f067aa0ba902b7,
    },
}
```

When making outbound requests, you can pull the traceparent from the current context and format it as a header:

```rust
# extern crate emit_traceparent;
# use std::collections::HashMap;
let mut headers = HashMap::<String, String>::new();

// 1. Get the current traceparent and tracestate
let (traceparent, tracestate) = emit_traceparent::current();

if traceparent.is_valid() {
    // 2. Add the traceparent and tracestate to the outgoing request
    headers.insert("traceparent".into(), traceparent.to_string());
    headers.insert("tracestate".into(), tracestate.to_string());
}
```

## Using the OpenTelemetry SDK for propagation

If you're using the OpenTelemetry SDK with [`emit_opentelemetry`](https://docs.rs/emit_opentelemetry), it will handle propagation for you.

## Manual propagation

When an incoming request arrives, you can push the trace and span ids onto the current context:

```rust
# extern crate emit;
// Parsed from the incoming call
let trace_id = "12b2fde225aebfa6758ede9cac81bf4d";
let span_id = "23995f85b4610391";

let frame = emit::Frame::push(emit::ctxt(), emit::props! {
    trace_id,
    span_id,
});

frame.call(handle_request);

#[emit::span("incoming request")]
fn handle_request() {
    // Your code goes here
}
```

```text
Event {
    mdl: "my_app",
    tpl: "incoming request",
    extent: Some(
        "2024-04-29T05:37:05.278488400Z".."2024-04-29T05:37:05.278636100Z",
    ),
    props: {
        "evt_kind": span,
        "span_name": "incoming request",
        "span_parent": 23995f85b4610391,
        "trace_id": 12b2fde225aebfa6758ede9cac81bf4d,
        "span_id": 641a578cc05c9db2,
    },
}
```

This pattern of pushing the incoming trace and span ids onto the context and then immediately calling a span annotated function ensures the incoming `span_id` becomes the `span_parent` in the events emitted by your application, without emitting a span event for the calling service itself.

When making outbound requests, you can pull the trace and span ids from the current context and format them as needed:

```rust
# extern crate emit;
use emit::{Ctxt, Props};

let (trace_id, span_id) = emit::ctxt().with_current(|props| {
    (
        props.pull::<emit::TraceId, _>(emit::well_known::KEY_TRACE_ID),
        props.pull::<emit::SpanId, _>(emit::well_known::KEY_SPAN_ID),
    )
});

if let (Some(trace_id), Some(span_id)) = (trace_id, span_id) {
    // Added to the outgoing call
}
```
