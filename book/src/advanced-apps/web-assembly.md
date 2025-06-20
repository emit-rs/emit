# Instrumenting WebAssembly

You can use `emit` in WebAssembly applications.

When WebAssembly support requires bindings to JavaScript APIs, crates will use a `web` Cargo feature that enables them.

If you're targeting WASI via the `wasm32-wasip1` or `wasm32-wasip2` targets, you shouldn't need to do anything special to make `emit` work, but not all emitters support WASI yet. The following emitters are compatible with WASI:

- [`emit_term`](../emitting-events/console.md) to emit events to stdout.

If you're targeting NodeJS or the web via the `wasm32-unknown-unknown` target, you can use `emit`'s default enabled `web` Cargo feature, and any emitters that support a `web` Cargo feature of their own. That includes:

- [`emit_otlp`](../emitting-events/otlp.md) to emit events to an OpenTelemetry-compatible service. Note that in the browser you may need to configure CORS. See [the docs](https://docs.rs/emit_otlp/#webassembly) for more details. `emit_otlp` supports `wasm32-unknown-unknown`, but not `wasm32v1-none`.
- [`emit_web`](https://docs.rs/emit_web) to emit events via the [`console`](https://developer.mozilla.org/en-US/docs/Web/API/console) API. `emit_web` supports both `wasm32-unknown-unknown` and `wasm32v1-none`.

You can also treat WebAssembly like any other embedded target. See [Instrumenting embedded applications](./embedded.md) for more details.
