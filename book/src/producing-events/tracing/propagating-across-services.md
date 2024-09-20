# Propagating span context across services

`emit` doesn't implement any distributed trace propagation itself. This is the responsibility of end-users through their web framework and clients to manage.

When an incoming request arrives, you can parse the trace and span ids from its traceparent header and push them onto the current context:

```rust
# extern crate emit;
// Parsed from a traceparent header
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

This pattern of pushing the incoming traceparent onto the context and then immediately calling a span annotated function ensures the `span_id` parsed from the traceparent becomes the `span_parent` in the events emitted by your application, without emitting a span event for the calling service itself.

When making outbound requests, you can pull the current trace and span ids from the current context and format them into a traceparent header:

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
    let traceparent = format!("00-{trace_id}-{span_id}-00");

    // Push the traceparent header onto the request
}
```
