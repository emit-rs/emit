<h1 style="display: flex; align-items: center">
<img style="display: inline" height="80px" width="80px" src="https://raw.githubusercontent.com/emit-rs/emit/main/asset/logo.svg" aria-hidden="true"> emit
</h1>

[![all](https://github.com/emit-rs/emit/actions/workflows/all.yml/badge.svg)](https://github.com/emit-rs/emit/actions/workflows/all.yml)

- [Guide](https://emitrs.io)
- [Examples](https://github.com/emit-rs/emit/tree/main/examples)
- [Technical docs](https://docs.rs/emit/0.11.0-alpha.16/emit/index.html)

## Developer-first diagnostics

`emit` is a framework for manually instrumenting Rust applications using an expressive syntax inspired by [Message Templates](https://messagetemplates.org). `emit` has a focused API that keeps configuration straightforward and doesn't stand out within code being instrumented.

## Getting started

```toml
[dependencies.emit]
version = "0.11.0-alpha.16"

[dependencies.emit_term]
version = "0.11.0-alpha.16"
```

```rust
fn main() {
    let rt = emit::setup()
        .emit_to(emit_term::stdout())
        .init();

    // Your app code goes here
    greet("Rust");

    rt.blocking_flush(std::time::Duration::from_secs(5));
}

#[emit::span("Greet {user}")]
fn greet(user: &str) {
    emit::info!("Hello, {user}!");
}
```

![The output of running the above program](https://github.com/emit-rs/emit/blob/main/asset/emit_term.png?raw=true)

## Where can I send my diagnostics?

Diagnostics produced by `emit` are sent to an _emitter_. This repository currently implements the following emitters:

- [`emit_term`](https://docs.rs/emit_term/0.11.0-alpha.16/emit_term/index.html) for writing human-readable output to the console.
- [`emit_file`](https://docs.rs/emit_file/0.11.0-alpha.16/emit_file/index.html) for writing JSON or another machine-readable format to rolling files.
- [`emit_otlp`](https://docs.rs/emit_otlp/0.11.0-alpha.16/emit_otlp/index.html) for sending diagnostics to an OpenTelemetry compatible collector.
- [`emit_opentelemetry`](https://docs.rs/emit_opentelemetry/0.11.0-alpha.16/emit_opentelemetry/index.html) for integrating `emit` into an application using the OpenTelemetry SDK for its diagnostics.

## Current status

This is alpha-level software. It implements a complete framework but has almost no tests and needs a lot more documentation. 
