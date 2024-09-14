<h1 style="display: flex; align-items: center">
<img style="display: inline" height="80px" width="80px" src="https://raw.githubusercontent.com/emit-rs/emit/main/asset/logo.svg" aria-hidden="true"> emit
</h1>

[![all](https://github.com/emit-rs/emit/actions/workflows/all.yml/badge.svg)](https://github.com/emit-rs/emit/actions/workflows/all.yml)

- [Guide](https://emitrs.io)
- [Examples](https://github.com/emit-rs/emit/tree/main/examples)
- [Crate docs](https://docs.rs/emit/0.11.0-alpha.16/emit/index.html)

## Developer-first diagnostics

`emit` is a framework for manually instrumenting Rust applications using an expressive syntax inspired by [Message Templates](https://messagetemplates.org).

`emit` has a small, fundamental data model where everything is represented by a single `Event` type.

`emit` has a focused API that keeps configuration straightforward and doesn't stand out within code being instrumented.

## Getting started

Add `emit` to your `Cargo.toml`:

```toml
[dependencies.emit]
version = "0.11.0-alpha.16"

[dependencies.emit_term]
version = "0.11.0-alpha.16"
```

Initialize `emit` in your `main.rs` and start peppering diagnostics throughout your application:

```rust
fn main() {
    // Configure `emit` to write events to the console
    let rt = emit::setup()
        .emit_to(emit_term::stdout())
        .init();

    // Your app code goes here
    greet("Rust");

    // Flush any remaining events before `main` returns
    rt.blocking_flush(std::time::Duration::from_secs(5));
}

#[emit::span("Greet {user}")]
fn greet(user: &str) {
    emit::info!("Hello, {user}!");
}
```

![The output of running the above program](https://github.com/emit-rs/emit/blob/main/asset/emit_term.png?raw=true)

## Emitting events

Use the `emit!` macros to emit diagnostic events in your application:

```rust
let user = "Rust";

emit::emit!("Hello, {user}!");
```

`emit`'s macro syntax is more than just format args. Values you use in the string template are included on your events, without losing their original type.

Events produced by `emit` are passed to an emitter, which encodes and forwards them on to some external observer. Typical emitters include:

- [`emit_term`](https://docs.rs/emit_term/0.11.0-alpha.16/emit_term/index.html) for writing human-readable output to the console.
- [`emit_file`](https://docs.rs/emit_file/0.11.0-alpha.16/emit_file/index.html) for writing JSON or another machine-readable format to rolling files.
- [`emit_otlp`](https://docs.rs/emit_otlp/0.11.0-alpha.16/emit_otlp/index.html) for sending diagnostics to an OpenTelemetry compatible collector.
- [`emit_opentelemetry`](https://docs.rs/emit_opentelemetry/0.11.0-alpha.16/emit_opentelemetry/index.html) for integrating `emit` into an application using the OpenTelemetry SDK for its diagnostics.

## Instrumenting functions

Use the `span!` macros to instrument a function, emitting a span event for it at the end with the time it took to execute:

```rust
#[emit::span("Greet {user}")]
fn greet(user: &str) {
    
}
```

Any other events emitted while this function executes will be correlated with it. Any other instrumented functions it calls will form a trace hierarchy.

`emit` doesn't actually hardcode the concept of spans. They're an extension of its core data model based on the presence of some well-known properties. A span-aware emitter can treat these events specially. In OTLP for instance, these events can be sent via the traces signal instead of to logs.

## Sampling metrics

`emit` doesn't have APIs for collecting metrics itself, that's left up to your application. What it does have is another extension to its data model for reporting metric samples:

```rust
let sample = sample_bytes_written();

emit::emit!(
    "{metric_agg} of {metric_name} is {metric_value}",
    evt_kind: "metric",
    metric_agg: "count",
    metric_name: "bytes_written",
    metric_value: sample,
);
```

Metric-aware emitters can treat these events specially. In OTLP for instance, these events can be sent via the metrics signal instead of to logs.

There's room in `emit` for many more extensions, including ones you define for your own applications.

## Troubleshooting

`emit` uses itself for diagnostics in its emitters. If you aren't seeing your diagnostics or something seems wrong, you can initialize the internal runtime and get an idea of what's going on:

```rust
fn main() {
    // NEW: Configure the internal runtime before your regular setup
    let internal_rt = emit::setup()
        .emit_to(emit_term::stdout())
        .init_internal();

    // Configure `emit` normally now
    // This example configures OTLP
    let rt = emit::setup()
        .emit_to(
            emit_otlp::new()
                .logs(emit_otlp::logs_grpc_proto("http://localhost:4319"))
                .traces(emit_otlp::traces_grpc_proto("http://localhost:4319"))
                .metrics(emit_otlp::metrics_grpc_proto("http://localhost:4319"))
                .spawn()
        )
        .init();

    // Your app code goes here

    rt.blocking_flush(std::time::Duration::from_secs(5));

    // NEW: Flush the internal runtime after your regular setup
    internal_rt.blocking_flush(std::time::Duration::from_secs(5));
}
```

## Next steps

Check the [Guide](https://emitrs.io) for a complete introduction to `emit`. Also see the [examples](https://github.com/emit-rs/emit/tree/main/examples) directory for common patterns, and the [crate docs](https://docs.rs/emit/0.11.0-alpha.16/emit/index.html) for technical details.

## Current status

This is alpha-level software. It implements a complete framework but has almost no tests and needs a lot more documentation. 
