# Emitting via OTLP

- Use `emit_otlp` to emit events to an OTLP-compatible service, like the OpenTelemetry Collector.
- Supports logs, metrics, and traces, but can be configured to emit a subset of those signals. You should at least always have logs configured.
- Supports gRPC, HTTP/proto, HTTP/JSON.
- Supports TLS.
- Supports gzip.
