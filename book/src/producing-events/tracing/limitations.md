# Tracing limitations

`emit`'s tracing model is intended to be simple, covering most key use-cases, but has some limitations compared to the OpenTelemetry model:

- No distinction between sampling and reporting; if a span exists, it's sampled.
- No span links.
- No span events.
