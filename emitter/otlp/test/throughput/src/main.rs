/*!
A throughput test for emitting events via OTLP.

This project doesn't prove much except what the on-thread cost of event serialization is like.
*/

use emit::{Clock, Emitter};
use std::sync::LazyLock;

use std::{
    env,
    process::{Child, Command},
    time::Duration,
};

// Concrete runtime to make `perf` output easier to follow
static RUNTIME: LazyLock<
    emit::runtime::Runtime<
        emit_otlp::Otlp,
        emit::Empty,
        emit::platform::DefaultCtxt,
        emit::platform::DefaultClock,
        emit::platform::DefaultRng,
    >,
> = LazyLock::new(|| {
    let emitter = emit_otlp::Otlp::builder()
        .traces(emit_otlp::traces_grpc_proto("http://localhost:44319"))
        .logs(emit_otlp::logs_grpc_proto("http://localhost:44319"))
        .spawn();
    emit::runtime::Runtime::build(
        emitter,
        emit::Empty,
        emit::platform::DefaultCtxt::shared(),
        emit::platform::DefaultClock::new(),
        emit::platform::DefaultRng::new(),
    )
});

fn main() {
    let stdout = emit_term::stdout();

    let spawn = env::args().any(|a| a == "--spawn");
    let flush = env::args().any(|a| a == "--flush");

    let mut reporter = emit::metric::Reporter::new();
    reporter.add_source(RUNTIME.emitter().metric_source());

    let otelcol = if spawn { Some(OtelCol::spawn()) } else { None };

    // Emit our events
    let count = 10_000;
    let start = RUNTIME.clock().now().unwrap();

    root(count);

    if flush {
        RUNTIME.blocking_flush(Duration::from_secs(30));
    }

    let end = RUNTIME.clock().now().unwrap();

    // Write the results
    stdout.emit(&emit::evt!(
        extent: start..end,
        "{count} iterations ({per_iteration}ns per iteration) with spawn {spawn} and flush {flush}",
        evt_kind: "span",
        per_iteration: (end - start).as_nanos() as f64 / count as f64,
    ));

    reporter.emit_metrics(&stdout);

    drop(otelcol);
}

#[emit::span(rt: &RUNTIME, "test root")]
fn root(count: usize) {
    for i in 0..count {
        run(i);
    }
}

#[emit::span(rt: &RUNTIME, "test span {i}")]
fn run(i: usize) {
    emit::info!(rt: &RUNTIME, "test event {i}");
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
