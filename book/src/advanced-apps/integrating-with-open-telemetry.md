# Integrating with OpenTelemetry

- Use `emit_opentelemetry` to wire `emit` up to the OpenTelemetry SDK.
- Useful for applications using multiple frameworks, like `log` or `tracing`. The OpenTelemetry SDK is a good common target for all these frameworks.
