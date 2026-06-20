# Tracing limitations

`emit`'s tracing model is intended to be simple, covering most key use-cases, but has some limitations compared to the OpenTelemetry model:

- No distinction between sampling and reporting; if a span exists, it's sampled. You can use [`emit_traceparent`](https://docs.rs/emit_traceparent/2.21.0/emit_traceparent/) to add sampling support.
- No span events.

Additionally, there is no guarantee of monotonicity; span extents are based on a start and end timestamp, so shifts in the underlying clock can produce misleading results.
