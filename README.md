<h1 style="display: flex; align-items: center">
<img style="display: inline" height="80px" width="80px" src="https://raw.githubusercontent.com/emit-rs/emit/main/asset/logo.svg" aria-hidden="true"> emit
</h1>

[![all](https://github.com/emit-rs/emit/actions/workflows/all.yml/badge.svg)](https://github.com/emit-rs/emit/actions/workflows/all.yml)

## Developer-first diagnostics for Rust applications

`emit` is a stable, complete, and capable framework for adding structured diagnostics to your Rust applications with a simple, powerful data model and an expressive syntax inspired by [Message Templates](https://messagetemplates.org). `emit`'s guiding design principle is low ceremony, low cognitive-load.

This readme covers just enough to give you an idea of what `emit` is. For a proper treatment, see:

- [the guide](https://emit-rs.io).
- [a set of task-oriented examples](https://github.com/emit-rs/emit/tree/main/examples).
- [the API docs](https://docs.rs/emit/1.5.0/emit/index.html).

## Getting started

Add `emit` to your `Cargo.toml`:

```toml
[dependencies.emit]
version = "1.5.0"
# Optional
features = ["serde"]

# Optional
[dependencies.emit_term]
version = "1.5.0"

# Optional
[dependencies.serde]
version = "1"
features = ["derive"]
```

Initialize `emit` in your `main.rs` and start peppering diagnostics throughout your application:

```rust
fn main() {
    // Configure `emit` to write events to the console
    let rt = emit::setup()
        .emit_to(emit_term::stdout())
        .init();

    // Your app code goes here
    {
        // `emit` supports fully structured data
        // See the `#[emit::as_serde]` attribute in our `greet` function below
        #[derive(serde::Serialize)]
        struct User<'a> {
            id: u32,
            name: &'a str,
        }

        // Annotate functions with `#[emit::span]` to produce traces
        #[emit::span("Greet {user}", #[emit::as_serde] user)]
        fn greet(user: &User) {
            // Use `emit::info` to produce log events
            emit::info!("Hello, {user: user.name}!");
        }

        greet(&User { id: 1, name: "Rust" });
    }

    // Flush any remaining events before `main` returns
    rt.blocking_flush(std::time::Duration::from_secs(5));
}
```

![The output of running the above program](https://github.com/emit-rs/emit/blob/main/asset/emit_term.png?raw=true)

`emit` has a capable syntax for writing events that's different from the standard `format!` trait. You can read more about it in [the guide](https://emit-rs.io/reference/templates.html).

## Tracing

`emit` can produce trace data that's compatible with OpenTelemetry and standard tracing tools, like Zipkin.

![An example trace produced by `emit` in Zipkin](https://github.com/emit-rs/emit/blob/main/asset/trace-zipkin.png?raw=true)

The above screenshot was generated by [this example application](https://github.com/emit-rs/emit/tree/main/examples/trace_zipkin).

See [the guide](https://emit-rs.io/producing-events/tracing.html) for details.

## Metrics

`emit` can produce metric data that's compatible with OpenTelemetry and standard metric tools, like Prometheus.

![An example metric produced by `emit` in Prometheus](https://github.com/emit-rs/emit/blob/main/asset/metric-prometheus.png?raw=true)

The above screenshot was generated by [this example application](https://github.com/emit-rs/emit/tree/main/examples/metric_prometheus).

See [the guide](https://emit-rs.io/producing-events/metrics.html) for details.

## Quick debugging

`emit` has a `dbg!` macro like the standard library's which you can use for quick-and-dirty debugging:

```rust
#[derive(Debug)]
pub struct User<'a> {
    id: u32,
    name: &'a str,
}

emit::dbg!(&User { id: 1, name: "Rust" });
```

See [the guide](https://emit-rs.io/producing-events/quick-debugging.html) for details.

## Stability

`emit` has a complete and stable API that's suitable for production environments.
