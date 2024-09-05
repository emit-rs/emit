// Emit a span
#[emit::span("Running emit")]
fn run_emit() {
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

fn run_opentelemetry<T: opentelemetry::trace::TracerProvider>(tracer_provider: T)
where
    <T::Tracer as opentelemetry::trace::Tracer>::Span: Send + Sync + 'static,
{
    use opentelemetry::trace::Tracer;

    tracer_provider
        .tracer("run_opentelemetry")
        .in_span("Running OTel", |_| {
            std::thread::sleep(std::time::Duration::from_secs(1));

            run_emit();
        })
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

    // Run our example program
    run_opentelemetry(tracer_provider.clone());

    // Shutdown the SDK
    let _ = logger_provider.shutdown();
    let _ = tracer_provider.shutdown();
}
