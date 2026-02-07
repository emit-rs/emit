# Tracing data model

The data model of spans is an extension of [`emit`'s events](../../reference/events.md). Span events include the following [well-known properties](https://docs.rs/emit/1.17.1/emit/well_known/index.html):

- `evt_kind`: with a value of `"span"` to indicate that the event is a span.
- `span_name`: a name for the operation the span represents. This defaults to the template.
- `span_kind`: a hint about the way an operation and its parent are related.
- `span_id`: an identifier for this specific invocation of the operation.
- `span_links`: a set of links between the span and others that it's causally related to outside of its immediate parent.
- `parent_id`: the `span_id` of the operation that invoked this one.
- `trace_id`: an identifier shared by all events in a distributed trace. A `trace_id` is assigned by the first operation.
