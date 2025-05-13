/*!
This example demonstrates configuring `emit` to produce OTLP directly.

You can point `emit` directly at an OpenTelemetry Collector or other compatible service.

`emit_otlp` can be configured manually, or via OpenTelemetry environment variables.
In this case, we're using environment variables, which will default to emitting via gRPC to `http://locslhost:4317`.

Try setting:

- `OTEL_EXPORTER_OTLP_ENDPOINT` and `OTEL_EXPORTER_OTLP_PROTOCOL` to specify where diagnostics go, and the protocol.
- `OTEL_SERVICE_NAME` to specify the name of the service.
- `OTEL_RESOURCE_ATTRIBUTES` to add ambient properties.

See [the docs](https://docs.rs/emit_otlp/1.8.1/emit_otlp/index.html#configuring-from-environment-variables) for more details on supported environment variables.
*/

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = emit::setup().emit_to(emit_term::stdout()).init_internal();

    // Configure `emit` to produce OTLP
    let rt = emit::setup().emit_to(emit_otlp::from_env().spawn()).init();

    run();

    rt.blocking_flush(std::time::Duration::from_secs(10));

    Ok(())
}

// Emit a span
#[emit::span("Running")]
fn run() {
    let mut counter = 1;

    for _ in 0..100 {
        counter += counter % 3;
    }

    // Emit a log record
    emit::info!("Counted up to {counter}");

    // Emit a metric sample
    emit::emit!(evt: emit::Metric::new(
        emit::mdl!(),
        "counter",
        emit::well_known::METRIC_AGG_COUNT,
        emit::Empty,
        counter,
        emit::Empty,
    ));
}
