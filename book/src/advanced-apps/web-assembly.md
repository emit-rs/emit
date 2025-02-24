# Instrumenting WebAssembly

You can use `emit` in WebAssembly applications.

If you're targeting WASI via the `wasm32-wasi` target, you shouldn't need to do anything special to make `emit` work.

If you're targeting NodeJS or the web via the `wasm32-unknown` target, you can use [`emit_web`](https://docs.rs/emit_web) to provide a clock and source of randomness. See [the crate docs](https://docs.rs/emit_web) for more details. You can also treat `wasm32-unknown` like any other embedded target. See [Instrumenting embedded applications](./embedded.md) for more details.
