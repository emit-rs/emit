# Acknowledgements

`emit` is a point in a long history of development in diagnostic tooling. It takes inspiration from others that have come before it:

- [`Serilog`](https://serilog.net) in C#. `emit`'s templates are a concept borrowed from Serilog, and `emit`'s API was shaped by many conversations with its original author.
- [`log`](https://docs.rs/log) and [`slog`](https://docs.rs/slog). `emit` is where some of the more radical ideas for `log`'s structured logging support ended up. A lot of discussion and development in `log` fed back into `emit`. `log`'s own structured logging took inspiration from `slog`.
- [`tracing`](https://docs.rs/tracing). `emit` takes inspiration from `tracing`, particularly its `#[instrument]` macro, and its `Value` type is based on discussions with its original authors.
- [OpenTelemetry](https://opentelemetry.io). `emit`'s data model, particularly for traces and metrics, is heavily inspired by OpenTelemetry's, and is intentionally compatible with it.

`emit` also couldn't exist without some of the fundamental libraries it depends on:

- [`syn`](https://docs.rs/syn) and [`quote`](https://docs.rs/quote) in `emit_macros`.
- [`rand`](https://docs.rs/rand) and [`serde`](https://docs.rs/serde) in `emit`.
- [`tokio`](https://docs.rs/tokio), [`hyper`](https://docs.rs/hyper), and [`rustls`](https://docs.rs/rustls) in `emit_otlp`.
