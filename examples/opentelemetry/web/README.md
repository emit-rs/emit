# An example `emit` browser application

This example demonstrates how to configure `emit` in a web-based application using `emit_otlp`. The key pieces are:

1. The `setup` function in `lib.rs`, where `emit` is configured.
2. The script block in `index.html`, where the `setup` function is imported and invoked.
3. The `config.yaml` for `otelcol`, which includes CORS configuration.
