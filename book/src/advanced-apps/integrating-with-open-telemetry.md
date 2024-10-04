# Integrating with OpenTelemetry

Larger applications may find themselves integrating components using multiple diagnostic frameworks, like [`log`](https://docs.rs/log/latest/log/) or [`tracing`](https://docs.rs/tracing/latest/tracing/). In these cases, you can use the [OpenTelemetry SDK](https://github.com/open-telemetry/opentelemetry-rust) as your central pipeline, with others integrating with it instead of eachother.

You can configure `emit` to send its diagnostics to the OpenTelemetry SDK using [`emit_opentelemetry`](https://docs.rs/emit_opentelemetry/0.11.0-alpha.18/emit_opentelemetry/index.html):

```toml
[dependencies.emit]
version = "0.11.0-alpha.18"

[dependencies.emit_opentelemetry]
version = "0.11.0-alpha.18"
```

```rust
# extern crate emit;
# extern crate emit_opentelemetry;
# extern crate opentelemetry;
# extern crate opentelemetry_sdk;
fn main() {
    // Configure the OpenTelemetry SDK
    // See the OpenTelemetry SDK docs for details on configuration
    let logger_provider = opentelemetry_sdk::logs::LoggerProvider::builder().build();
    let tracer_provider = opentelemetry_sdk::trace::TracerProvider::builder().build();

    // Configure `emit` to point to the OpenTelemetry SDK
    let rt = emit_opentelemetry::setup(logger_provider, tracer_provider).init();

    // Your app code goes here

    rt.blocking_flush(std::time::Duration::from_secs(30));

    // Shutdown the OpenTelemetry SDK
}
```

See [the crate docs](https://docs.rs/emit_opentelemetry/0.11.0-alpha.18/emit_opentelemetry/index.html) for more details.
