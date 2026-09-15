# Tracing data model

The tracing data model is based on span events. Span events are an extension of [`emit`'s events](../../reference/events.md). They include the following [well-known properties](https://docs.rs/emit/2.23.0/emit/well_known/index.html):

- `evt_kind`: with a value of `"span"` to indicate that the event is a span.
- `trace_id`: an identifier shared by all events in a distributed trace. A `trace_id` is assigned by the first operation.
- `span_id`: an identifier for this specific invocation of the operation.
- `parent_id`: the `span_id` of the operation that invoked this one.
- `span_name`: a name for the operation the span represents. This defaults to the template.
- `span_kind`: a hint about the way an operation and its parent are related.
- `span_links`: a set of links between the span and others that it's causally related to outside of its immediate parent.
