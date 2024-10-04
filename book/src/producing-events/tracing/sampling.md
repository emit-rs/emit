# Sampling and filtering traces

Sampling is a tool to help limit the volume of ingested trace data. It's typically applied when a trace begins by making an upfront decision about whether to produce and/or emit the trace. This is usually called "head sampling" and is limited to probablistic methods. Tail sampling, or deciding whether to ingest a trace after it's completed is much harder to implement, because there's no absolute way to know when a particular trace is finished, or how long it will take.

`emit` doesn't bake in any sampling concept directly, but the [`#[span]`](https://docs.rs/emit/0.11.0-alpha.17/emit/attr.span.html) macro does apply filtering before creating any span context. This can be used to implement sampling. If you're using the OpenTelemetry SDK, [`emit_opentelemetry`](https://docs.rs/emit_opentelemetry/latest/emit_opentelemetry/) will respect its sampling.

When filtering spans, remember that once a span is created it _must_ be completed and emitted. Otherwise you'll end up with a broken trace where span subtrees are missing.
