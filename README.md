<h1 style="display: flex; align-items: center">
<img style="display: inline" height="80px" width="80px" src="https://raw.githubusercontent.com/emit-rs/emit/main/asset/logo.svg" aria-hidden="true"> emit
</h1>

[![all](https://github.com/emit-rs/emit/actions/workflows/all.yml/badge.svg)](https://github.com/emit-rs/emit/actions/workflows/all.yml)

## Developer-first diagnostics for Rust applications

`emit` is a framework for adding diagnostics to your Rust applications with a simple, powerful data model and an expressive syntax inspired by [Message Templates](https://messagetemplates.org). `emit`'s guiding design principle is low ceremony, low cognitive-load.

This readme covers just enough to give you an idea of what `emit` is. For a proper treatment, see:

- [the guide](https://emit-rs.io)
- [a set of task-oriented examples](https://github.com/emit-rs/emit/tree/main/examples).
- [the API docs](https://docs.rs/emit/0.11.0-alpha.18/emit/index.html).

## Getting started

Add `emit` to your `Cargo.toml`:

```toml
[dependencies.emit]
version = "0.11.0-alpha.18"

[dependencies.emit_term]
version = "0.11.0-alpha.18"
```

Initialize `emit` in your `main.rs` and start peppering diagnostics throughout your application:

```rust
fn main() {
    // Configure `emit` to write events to the console
    let rt = emit::setup()
        .emit_to(emit_term::stdout())
        .init();

    // Your app code goes here
    //
    // Try uncommenting the following line as an example:
    //
    // greet("Rust");

    // Flush any remaining events before `main` returns
    rt.blocking_flush(std::time::Duration::from_secs(5));
}

#[emit::span("Greet {user}")]
fn greet(user: &str) {
    emit::info!("Hello, {user}!");
}
```

![The output of running the above program](https://github.com/emit-rs/emit/blob/main/asset/emit_term.png?raw=true)
