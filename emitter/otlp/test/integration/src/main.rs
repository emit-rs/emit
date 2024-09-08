/*!
An integration test between `emit_otlp` and the OpenTelemetry Collector.
*/

use emit::Emitter;

use std::{
    io::Read,
    path::Path,
    process::{Child, Command, Stdio},
};

fn main() {
    let _ = emit::setup().emit_to(emit_term::stdout()).init_internal();
    let _ = emit::setup().emit_to(emit_term::stdout()).init();

    assert_emitter(
        "gRPC proto",
        OtelCol::spawn("config"),
        emit_otlp::new()
            .resource(emit::props! {
                #[emit::key("service.name")]
                service_name: emit::pkg!(),
            })
            .logs(emit_otlp::logs_grpc_proto("http://localhost:44319"))
            .traces(emit_otlp::traces_grpc_proto("http://localhost:44319"))
            .metrics(emit_otlp::metrics_grpc_proto("http://localhost:44319"))
            .spawn(),
    );

    assert_emitter(
        "HTTP proto",
        OtelCol::spawn("config"),
        emit_otlp::new()
            .resource(emit::props! {
                #[emit::key("service.name")]
                service_name: emit::pkg!(),
            })
            .logs(emit_otlp::logs_http_proto("http://localhost:44318/v1/logs"))
            .traces(emit_otlp::traces_http_proto(
                "http://localhost:44318/v1/traces",
            ))
            .metrics(emit_otlp::metrics_http_proto(
                "http://localhost:44318/v1/metrics",
            ))
            .spawn(),
    );

    assert_emitter(
        "HTTP JSON",
        OtelCol::spawn("config"),
        emit_otlp::new()
            .resource(emit::props! {
                #[emit::key("service.name")]
                service_name: emit::pkg!(),
            })
            .logs(emit_otlp::logs_http_json("http://localhost:44318/v1/logs"))
            .traces(emit_otlp::traces_http_json(
                "http://localhost:44318/v1/traces",
            ))
            .metrics(emit_otlp::metrics_http_json(
                "http://localhost:44318/v1/metrics",
            ))
            .spawn(),
    );

    let cert_path = "./127.0.0.1+1.pem";
    if Path::new(cert_path).exists() {
        emit::info!("checking TLS configurations");

        assert_emitter(
            "gRPC proto (TLS)",
            OtelCol::spawn("config.tls"),
            emit_otlp::new()
                .resource(emit::props! {
                    #[emit::key("service.name")]
                    service_name: emit::pkg!(),
                })
                .logs(emit_otlp::logs_grpc_proto("https://localhost:44319"))
                .traces(emit_otlp::traces_grpc_proto("https://localhost:44319"))
                .metrics(emit_otlp::metrics_grpc_proto("https://localhost:44319"))
                .spawn(),
        );

        assert_emitter(
            "HTTP proto (TLS)",
            OtelCol::spawn("config.tls"),
            emit_otlp::new()
                .resource(emit::props! {
                    #[emit::key("service.name")]
                    service_name: emit::pkg!(),
                })
                .logs(emit_otlp::logs_http_proto(
                    "https://localhost:44318/v1/logs",
                ))
                .traces(emit_otlp::traces_http_proto(
                    "https://localhost:44318/v1/traces",
                ))
                .metrics(emit_otlp::metrics_http_proto(
                    "https://localhost:44318/v1/metrics",
                ))
                .spawn(),
        );
    } else {
        emit::warn!(
            "not running TLS tests because the local certificate file {cert_path} doesn't exist"
        );
    }
}

#[emit::span("integration test {name}")]
fn assert_emitter(
    name: &str,
    otelcol: OtelCol,
    emitter: impl emit::Emitter + Send + Sync + 'static,
) {
    emit::info!("checking {name}");

    let rt = emit::runtime::Runtime::new().with_emitter(emitter);

    // Generate some random ids
    // These are used to assert the collector received our events
    let log_uuid = uuid::Uuid::new_v4().to_string();
    let span_uuid = uuid::Uuid::new_v4().to_string();
    let metric_uuid = uuid::Uuid::new_v4().to_string();

    // Emit a log event
    emit::info!(rt, "A log message {log_uuid}");

    // Emit a span in a trace
    emit::emit!(rt, extent: emit::now!()..emit::now!(), "A span {span_uuid}", evt_kind: "span", span_name: "emit_otlp_test", trace_id: emit::new_trace_id!(), span_id: emit::new_span_id!());

    // Emit a metric
    emit::emit!(rt, "A metric {metric_uuid}", evt_kind: "metric", metric_name: "emit_otlp_test", metric_agg: "count", metric_value: 1);

    // Flush `emit_otlp` and read the output from the collector
    rt.blocking_flush(std::time::Duration::from_secs(10));
    let output = otelcol.output();

    // Ensure the collector received and accepted the events we emitted
    assert_exporter(&output, "LogsExporter", &log_uuid);
    assert_exporter(&output, "TracesExporter", &span_uuid);
    assert_exporter(&output, "MetricsExporter", &metric_uuid);
}

fn assert_exporter(output: &str, exporter: &str, id: &str) {
    assert!(
        output.contains(&exporter),
        "{exporter} not found in:\n{output}"
    );
    assert!(output.contains(id), "{id} noot found in:\n{output}");
}

struct OtelCol(Child);

impl Drop for OtelCol {
    fn drop(&mut self) {
        let _ = self.0.kill();
    }
}

impl OtelCol {
    fn spawn(config: &str) -> Self {
        OtelCol(
            Command::new("otelcol")
                .args(["--config", &format!("./{config}.yaml")])
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
