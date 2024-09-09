use std::time::Duration;

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = emit::setup().emit_to(emit_term::stdout()).init_internal();

    let rt = emit::setup()
        .emit_to(
            emit_otlp::new()
                .logs(emit_otlp::logs_grpc_proto("http://localhost:4319"))
                .traces(emit_otlp::traces_grpc_proto("http://localhost:4319"))
                .metrics(emit_otlp::metrics_grpc_proto("http://localhost:4319"))
                .spawn(),
        )
        .init();

    run();

    rt.blocking_flush(Duration::from_secs(10));

    Ok(())
}
