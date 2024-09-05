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
    emit::runtime::shared().emit(emit::Metric::new(
        emit::mdl!(),
        "counter",
        emit::well_known::METRIC_AGG_COUNT,
        emit::Empty,
        counter,
        emit::Empty,
    ));
}

#[tokio::main]
async fn main() {
    // Configure the OpenTelemetry SDK
    let channel = tonic::transport::Channel::from_static("http://localhost:4319")
        .connect()
        .await
        .unwrap();

    let tracer_provider = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_channel(channel.clone()),
        )
        .install_batch(opentelemetry_sdk::runtime::Tokio)
        .unwrap();

    let logger_provider = opentelemetry_otlp::new_pipeline()
        .logging()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_channel(channel.clone()),
        )
        .install_batch(opentelemetry_sdk::runtime::Tokio)
        .unwrap();

    // Configure `emit` to point to `opentelemetry`
    let _ = emit_opentelemetry::setup(logger_provider.clone(), tracer_provider.clone()).init();

    run();

    let _ = logger_provider.shutdown();
    let _ = tracer_provider.shutdown();
}
