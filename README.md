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

Add `emit` to your `Cargo.toml`:

```toml
[dependencies.emit]
version = "0.11.0-alpha.16"

[dependencies.emit_term]
version = "0.11.0-alpha.16"
```

This example uses [`emit_term`](https://docs.rs/emit_term/0.11.0-alpha.16/emit_term/index.html) for writing diagnostics to the console. You can also write the, [to rolling files](https://docs.rs/emit_file/0.11.0-alpha.16/emit_file/index.html), [to a remote collector](https://docs.rs/emit_otlp/0.11.0-alpha.16/emit_otlp/index.html), or to a custom destination.

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

## Instrumenting functions

## Sampling metrics

## Troubleshooting

## Next steps

## Current status

This is alpha-level software. It implements a complete framework but has almost no tests and needs a lot more documentation. 
