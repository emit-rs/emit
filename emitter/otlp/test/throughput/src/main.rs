/*!
A throughput test for emitting events via OTLP.

This project doesn't prove much except what the on-thread cost of event serialization is like.
*/

use emit::Emitter;

use std::{
    env,
    process::{Child, Command},
    time::Duration,
};

fn main() {
    let stdout = emit_term::stdout();

    let spawn = env::args().any(|a| a == "--spawn");
    let flush = env::args().any(|a| a == "--flush");

    let mut reporter = emit::metric::Reporter::new();

    // Set up `emit_otlp`
    let rt = emit::setup()
        .emit_to({
            let emitter = emit_otlp::new()
                .traces(emit_otlp::traces_grpc_proto("http://localhost:44319"))
                .logs(emit_otlp::logs_grpc_proto("http://localhost:44319"))
                .spawn();

            reporter.add_source(emitter.metric_source());

            emitter
        })
        .init();

    let otelcol = if spawn { Some(OtelCol::spawn()) } else { None };

    // Emit our events
    let count = 10_000;
    let start = emit::clock().now().unwrap();

    root(count);

    if flush {
        rt.blocking_flush(Duration::from_secs(30));
    }

    let end = emit::clock().now().unwrap();

    // Write the results
    let per_event = (end - start).as_nanos() as f64 / count as f64;

    stdout.emit(&emit::evt!(
        extent: start..end,
        "emitted {count} events ({per_event}ns per run) with spawn {spawn} and flush {flush}",
        evt_kind: "span",
    ));

    reporter.emit_metrics(&stdout);

    drop(otelcol);
}

#[emit::span("test root")]
fn root(count: usize) {
    for i in 0..count {
        run(i);
    }
}

#[emit::span("test span {i}")]
fn run(i: usize) {
    emit::info!("test event {i}");
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
                .unwrap(),
        )
    }
}
