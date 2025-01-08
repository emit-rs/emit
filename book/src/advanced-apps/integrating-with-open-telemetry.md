# Integrating with OpenTelemetry

Larger applications may find themselves integrating components using multiple diagnostic frameworks, like [`log`](https://docs.rs/log/latest/log/) or [`tracing`](https://docs.rs/tracing/latest/tracing/). In these cases, you can use the [OpenTelemetry SDK](https://github.com/open-telemetry/opentelemetry-rust) as your central pipeline, with others integrating with it instead of eachother.

You can configure `emit` to send its diagnostics to the OpenTelemetry SDK using [`emit_opentelemetry`](https://docs.rs/emit_opentelemetry/0.11.0-alpha.21/emit_opentelemetry/index.html). See [the crate docs](https://docs.rs/emit_opentelemetry/0.11.0-alpha.21/emit_opentelemetry/index.html) for more details.
