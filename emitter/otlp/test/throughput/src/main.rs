/*!
A throughput test for emitting events via OTLP.

This project doesn't prove much except what the on-thread cost of event serialization is like.
*/

use emit::Emitter;

use std::{
    process::{Child, Command},
    time::Duration,
};

fn main() {
    let mut reporter = emit::metric::Reporter::new();

    // Set up `emit_otlp`
    let rt = emit::setup()
        .emit_to({
            let emitter = emit_otlp::new()
                .logs(emit_otlp::logs_grpc_proto("http://localhost:44319"))
                .spawn();

            reporter.add_source(emitter.metric_source());

            emitter
        })
        .init();

    // Spawn a collector
    let otelcol = OtelCol::spawn();

    // Emit our events
    let count = 10_000;
    let start = emit::now!().unwrap();

    for i in 0..count {
        emit::info!("test event {i}");
    }

    rt.blocking_flush(Duration::from_secs(30));

    let end = emit::now!().unwrap();

    // Write the results
    let per_event = (end - start).as_nanos() as f64 / count as f64;

    let stdout = emit_term::stdout();

    stdout.emit(&emit::evt!(
        extent: start..end,
        "emitted {count} events ({per_event}ns per event)",
        evt_kind: "span",
    ));

    reporter.emit_metrics(&stdout);

    drop(otelcol);
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
