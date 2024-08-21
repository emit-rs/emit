use std::{
    io::Read,
    process::{Child, Command, Stdio},
};

fn main() {
    let rt = emit::setup()
        .emit_to(
            emit_otlp::new()
                .resource(emit::props! {
                    #[emit::key("service.name")]
                    service_name: emit::pkg!(),
                })
                .logs(emit_otlp::logs_grpc_proto("http://localhost:44319"))
                .traces(emit_otlp::traces_grpc_proto("http://localhost:44319"))
                .metrics(emit_otlp::metrics_grpc_proto("http://localhost:44319"))
                .spawn()
                .unwrap(),
        )
        .init();

    let otelcol = OtelCol::spawn();

    let log_uuid = uuid::Uuid::new_v4().to_string();
    let span_uuid = uuid::Uuid::new_v4().to_string();
    let metric_uuid = uuid::Uuid::new_v4().to_string();

    emit::info!("A log message {log_uuid}");

    #[emit::span("A span {span_uuid}")]
    fn span(span_uuid: &str) {}

    span(&span_uuid);

    emit::emit!("A metric {metric_uuid}", evt_kind: "metric", metric_name: "emit_otlp_test", metric_agg: "count", metric_value: 1);

    rt.blocking_flush(std::time::Duration::from_secs(10));

    let output = otelcol.output();

    println!("{output}");

    assert_emitted(&output, "LogsExporter", &log_uuid);
    assert_emitted(&output, "TracesExporter", &span_uuid);
    assert_emitted(&output, "MetricsExporter", &metric_uuid);
}

fn assert_emitted(output: &str, exporter: &str, id: &str) {
    assert!(
        output.contains(&exporter),
        "{exporter} not found in otelcol output"
    );
    assert!(output.contains(id), "{id} noot found in otelcol output");
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
                .stderr(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
                .unwrap(),
        )
    }

    fn output(mut self) -> String {
        let mut stdout = self.0.stdout.take().unwrap();
        let mut stderr = self.0.stderr.take().unwrap();

        self.0.kill().unwrap();

        let mut buf = String::new();
        stdout.read_to_string(&mut buf).unwrap();
        stderr.read_to_string(&mut buf).unwrap();

        buf
    }
}
