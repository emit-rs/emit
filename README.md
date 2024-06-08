<h1 style="display: flex; align-items: center">
<img style="display: inline" height="80px" width="80px" src="https://raw.githubusercontent.com/emit-rs/emit/main/asset/logo.svg" aria-hidden="true"> emit
</h1>

[![all](https://github.com/emit-rs/emit/actions/workflows/all.yml/badge.svg)](https://github.com/emit-rs/emit/actions/workflows/all.yml)

[Current docs](https://docs.rs/emit/0.11.0-alpha.2/emit/index.html)

## Structured diagnostics for Rust applications

`emit` is a structured logging framework for manually instrumenting Rust applications with an expressive syntax inspired by [Message Templates](https://messagetemplates.org).

`emit` represents all diagnostics as _events_; a combination of timestamp or timespan, template, and properties. Traditional log records, spans in a distributed trace, and metric samples are all represented as events. Having a unified model of all these signals means you can always capture your diagnostics in one way or another.

```toml
[dependencies.emit]
version = "0.11.0-alpha.2"

[dependencies.emit_term]
version = "0.11.0-alpha.2"
```

```rust
use std::time::Duration;

fn main() {
    let rt = emit::setup()
        .emit_to(emit_term::stdout())
        .init();

    greet("Rust");

    rt.blocking_flush(Duration::from_secs(5));
}

#[emit::span("Greet {user}")]
fn greet(user: &str) {
    emit::info!("Hello, {user}!");
}
```

![The output of running the above program](https://github.com/emit-rs/emit/blob/main/asset/emit_term.png?raw=true)

## Current status

This is alpha-level software. It implements a complete framework but has almost no tests and needs a lot more documentation.

## Getting started

See the `examples` directory and `emit` documentation for how to get started with `emit`.
