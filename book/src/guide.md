# `emit`

`emit` is a framework for adding diagnostics to your Rust applications with a simple, powerful data model and an expressive syntax inspired by [Message Templates](https://messagetemplates.org).

Diagnostics in `emit` are represented as _events_ which combine:

- _extent:_ The point in time when the event occurred, or the span of time for which it was active.
- _template:_ A user-facing description of the event that supports property interpolation.
- _properties:_ A bag of structured key-value pairs that capture the context surrounding the event. Properties may be simple primitives like numbers or strings, or arbitrarily complex structures like maps, sequences, or enums.

Using `emit`'s events you can:

- log structured events.
- trace function execution and participate in distributed tracing.
- surface live metrics.
- build anything you can represent as a time-oriented bag of data.

## Who is `emit` for?

`emit` is for Rust applications, it's not intended to be used in public libraries. In general, libraries shouldn't use a diagnostics framework anyway, but `emit`'s opinionated data model, use of dependencies, and procedural macros, will likely make it unappealing for Rust libraries.

## Design goals

`emit`'s guiding design principle is low ceremony, low cognitive-load. Diagnostics are our primary focus, but they're not yours. Configuration should be straightforward, operation should be transparent, and visual noise in instrumented code should be low.

These goals result in some tradeoffs that may affect `emit`'s suitability for your needs:

- Simplicity over performance. Keeping the impact of diagnostics small is still important, but not at the expense of usability or simplicity.
- Not an SDK. `emit` has a hackable API you can tailor to your needs but is also a small, complete, and cohesive set of components for you to use out-of-the-box.

## Stability

`emit` follows the regular semver policy of other Rust libraries with the following additional considerations:

- Changes to the interpretation of events, such as the introduction of new extensions, are considered breaking.
- Breaking changes to `emit_core` are not planned.
- Breaking changes to `emit` itself, its macros, and emitters, may happen infrequently. Major changes to its APIs are not planned. We're aware that, as a diagnostics library, you're likely to spread a lot of `emit` code through your application, so even small changes can have a big impact.

As an application developer, you should be able to rely on the stability of `emit` not to get in the way of your everyday programming.

## Getting started

Add `emit` to your `Cargo.toml`, along with an _emitter_ to write diagnostics to:

```toml
[dependencies.emit]
version = "0.11.0-alpha.16"

[dependencies.emit_term]
version = "0.11.0-alpha.16"
```

Initialize `emit` at the start of your `main.rs` using `emit::setup()`, and ensure any emitted diagnostics are flushed by calling `blocking_flush()` at the end:

```rust
extern crate emit;
extern crate emit_term;

fn main() {
    // Configure `emit` to write events to the console
    let rt = emit::setup()
        .emit_to(emit_term::stdout())
        .init();

    // Your app code goes here

    // Flush any remaining events before `main` returns
    rt.blocking_flush(std::time::Duration::from_secs(5));
}
```

Start peppering diagnostics through your application with `emit`'s macros.

### Logging events

When something of note happens, use `emit::debug!` or `emit::info!` to log it:

```rust
# extern crate emit;
let user = "user-123";
let item = "product-123";

emit::info!("{user} added {item} to their cart");
```

When something fails, use `emit::warn!` or `emit::error!`:

```rust
# extern crate emit;
# let user = "user-123";
let err = std::io::Error::new(
    std::io::ErrorKind::Other,
    "failed to connect to the remote service",
);

emit::warn!("updating {user} cart failed: {err}");
```

### Tracing functions

Add `#[emit::span]` to a significant function in your application to trace its execution:

```rust
# extern crate emit;
#[emit::span("add {item} to {user} cart")]
async fn add_item(user: &str, item: &str) {
    // Your code goes here
}
```

Any diagnostics emitted within a traced function will be correlated with it. Any other traced functions it calls will form a trace hierarchy.
