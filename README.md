<h1 style="display: flex; align-items: center">
<img style="display: inline" height="80px" width="80px" src="https://raw.githubusercontent.com/emit-rs/emit/main/asset/logo.svg" aria-hidden="true"> emit
</h1>

[![all](https://github.com/emit-rs/emit/actions/workflows/all.yml/badge.svg)](https://github.com/emit-rs/emit/actions/workflows/all.yml)

[Current docs](https://docs.rs/emit/0.11.0-alpha.15/emit/index.html)

## Developer-first diagnostics

`emit` is a framework for manually instrumenting Rust applications using an expressive syntax inspired by [Message Templates](https://messagetemplates.org). `emit` has a focused API that keeps configuration straightforward and doesn't stand out within code being instrumented.

`emit` represents all diagnostics as _events_; a combination of timestamp or timespan, message template, and properties. Traditional log records, spans in a distributed trace, and metric samples are all represented as events. Having a unified model of all these signals means you can always capture your diagnostics in one way or another.

`emit` is fully structured. Properties can be simple primitives like numbers, strings, and booleans, or arbitrarily complex arrays, maps, or enums.

## Getting started

```toml
[dependencies.emit]
version = "0.11.0-alpha.15"

[dependencies.emit_term]
version = "0.11.0-alpha.15"
```

```rust
use std::time::Duration;

fn main() {
    let rt = emit::setup()
        .emit_to(emit_term::stdout())
        .init();

    // Your app code goes here
    greet("Rust");

    rt.blocking_flush(Duration::from_secs(5));
}

// The `#[emit::span]` attribute instruments a function
// When it returns it will emit an event with the time it took to execute
// Any events emitted inside it will be correlated together 
#[emit::span("Greet {user}")]
fn greet(user: &str) {
    // The `emit::info!` attribute emits an event for a log record
    emit::info!("Hello, {user}!");
}
```

![The output of running the above program](https://github.com/emit-rs/emit/blob/main/asset/emit_term.png?raw=true)

## Current status

This is alpha-level software. It implements a complete framework but has almost no tests and needs a lot more documentation. 

## Getting started

See the `examples` directory and [`emit` documentation](https://docs.rs/emit/0.11.0-alpha.15/emit/index.html) to get started with `emit`.

## Where can I send my diagnostics?

Diagnostics produced by `emit` are sent to an _emitter_. This repository currently implements the following emitters:

- [`emit_term`](https://docs.rs/emit_term/0.11.0-alpha.15/emit_term/index.html) for writing human-readable output to the console.
- [`emit_file`](https://docs.rs/emit_file/0.11.0-alpha.15/emit_file/index.html) for writing JSON or another machine-readable format to rolling files.
- [`emit_otlp`](https://docs.rs/emit_otlp/0.11.0-alpha.15/emit_otlp/index.html) for sending diagnostics to an OpenTelemetry compatible collector.
- [`emit_opentelemetry`](https://docs.rs/emit_opentelemetry/0.11.0-alpha.15/emit_opentelemetry/index.html) for integrating `emit` into an application using the OpenTelemetry SDK for its diagnostics.
