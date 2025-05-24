# An example `emit` browser application

This example demonstrates how to configure `emit` in a web-based application using `emit_web` and `emit_otlp`. The key pieces are:

1. The `setup` function in `lib.rs`, where `emit` is configured.
2. The script block in `index.html`, where the `setup` function is imported and invoked.

## Running the example

1. Install `wasm-pack`.
2. Start the OpenTelemetry Collector using the `config.yaml` in this directory.
3. Call `run.sh` and visit `localhost:8080` in a browser.
