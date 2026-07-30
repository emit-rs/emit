/*!
A throughput test for emitting events via the OpenTelemetry SDK + OTLP gRPC exporter.

This creates the same shape and frequency of spans/logs as `emit_otlp_test_throughput`, providing a baseline.
*/

use opentelemetry::logs::{AnyValue, LogRecord, Logger, LoggerProvider, Severity};
use opentelemetry::trace::{Span, Tracer, TracerProvider};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    Resource,
    logs::{BatchConfigBuilder, BatchLogProcessor},
    trace::{BatchConfigBuilder as TraceBatchConfigBuilder, BatchSpanProcessor},
};

use std::{
    env,
    process::{Child, Command},
    time::Instant,
};

#[tokio::main]
async fn main() {
    let spawn = env::args().any(|a| a == "--spawn");
    let do_flush = env::args().any(|a| a == "--flush");

    let resource = Resource::builder_empty().build();

    let span_exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint("http://localhost:44319")
        .build()
        .expect("Failed to create span exporter");

    let log_exporter = opentelemetry_otlp::LogExporter::builder()
        .with_tonic()
        .with_endpoint("http://localhost:44319")
        .build()
        .expect("Failed to create log exporter");

    // Queue and batch large enough to hold all events so none are dropped
    let capacity = 20_000;

    let span_processor = BatchSpanProcessor::builder(span_exporter)
        .with_batch_config(
            TraceBatchConfigBuilder::default()
                .with_max_queue_size(capacity)
                .with_max_export_batch_size(capacity)
                .build(),
        )
        .build();

    let log_processor = BatchLogProcessor::builder(log_exporter)
        .with_batch_config(
            BatchConfigBuilder::default()
                .with_max_queue_size(capacity)
                .with_max_export_batch_size(capacity)
                .build(),
        )
        .build();

    let tracer_provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
        .with_resource(resource.clone())
        .with_span_processor(span_processor)
        .build();

    let logger_provider = opentelemetry_sdk::logs::SdkLoggerProvider::builder()
        .with_resource(resource)
        .with_log_processor(log_processor)
        .build();

    let otelcol = if spawn { Some(OtelCol::spawn()) } else { None };

    let tracer = tracer_provider.tracer("otel-throughput-test");
    let logger = logger_provider.logger("otel-throughput-test");

    // Emit events
    let count = 10_000;
    let start = Instant::now();

    root(&tracer, &logger, count);

    if do_flush {
        tracer_provider.force_flush().unwrap();
        logger_provider.force_flush().unwrap();
    }

    let elapsed = start.elapsed();
    let per_iteration = elapsed.as_nanos() as f64 / count as f64;

    println!(
        "{count} iterations ({:.2}ns per iteration), spawn: {}, flush: {}",
        per_iteration, spawn, do_flush,
    );

    tracer_provider.shutdown().unwrap();
    logger_provider.shutdown().unwrap();

    drop(otelcol);
}

fn root(tracer: &impl Tracer, logger: &impl Logger, count: usize) {
    let mut root_span = tracer.start("test root");
    for i in 0..count {
        run(tracer, logger, i);
    }
    root_span.end();
}

fn run(tracer: &impl Tracer, logger: &impl Logger, i: usize) {
    let mut span = tracer.start(format!("test span {i}"));

    let mut log_record = logger.create_log_record();
    log_record.set_severity_number(Severity::Info);
    log_record.set_severity_text("INFO");
    log_record.set_body(AnyValue::String(format!("test event {i}").into()));
    logger.emit(log_record);

    span.end();
}

struct OtelCol(Child);

impl Drop for OtelCol {
    fn drop(&mut self) {
        let _ = self.0.kill();
    }
}

impl OtelCol {
    fn spawn() -> Self {
        OtelCol(
            Command::new("otelcol")
                .args(["--config", "./config.yaml"])
                .spawn()
                .expect("Failed to spawn otelcol"),
        )
    }
}
