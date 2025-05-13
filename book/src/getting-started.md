# Getting started

Add `emit` to your `Cargo.toml`, along with an _emitter_ to write diagnostics to:

```toml
[dependencies.emit]
version = "1.8.1"

[dependencies.emit_term]
version = "1.8.1"
```

Initialize `emit` at the start of your `main.rs` using [`emit::setup()`](https://docs.rs/emit/1.8.1/emit/setup/index.html), and ensure any emitted diagnostics are flushed by calling [`blocking_flush()`](https://docs.rs/emit/1.8.1/emit/setup/struct.Init.html#method.blocking_flush) at the end:

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

## Logging events

When something of note happens, use [`debug!`](https://docs.rs/emit/1.8.1/emit/macro.debug.html) or [`info!`](https://docs.rs/emit/1.8.1/emit/macro.info.html) to log it:

```rust
# extern crate emit;
let user = "user-123";
let item = "product-123";

emit::info!("{user} added {item} to their cart");
```

When something fails, use [`warn!`](https://docs.rs/emit/1.8.1/emit/macro.warn.html) or [`error!`](https://docs.rs/emit/1.8.1/emit/macro.error.html):

```rust
# extern crate emit;
# let user = "user-123";
let err = std::io::Error::new(
    std::io::ErrorKind::Other,
    "failed to connect to the remote service",
);

emit::warn!("updating {user} cart failed: {err}");
```

## Tracing functions

Add [`#[span]`](https://docs.rs/emit/1.8.1/emit/attr.span.html) to a significant function in your application to trace its execution:

```rust
# extern crate emit;
#[emit::span("add {item} to {user} cart")]
async fn add_item(user: &str, item: &str) {
    // Your code goes here
}
```

Any diagnostics emitted within a traced function will be correlated with it. Any other traced functions it calls will form a trace hierarchy.

## Quick debugging

Use the [`dbg!`](https://docs.rs/emit/1.8.1/emit/macro.dbg.html) macro to help debug code as you're writing it:

```rust
# extern crate emit;
# fn main() {
let user = "user@example.com";
let id = 42;

emit::dbg!(user, id);
# }
```

It works a lot like the standard library's `dbg!` macro, and is meant to be used as a quick, temporary debugging aid.

## Next steps

To learn more about configuring `emit`, see the [Emitting events](./emitting-events.md) section.

To learn more about using `emit`, see the [Producing events](./producing-events.md) section.

To learn `emit`'s architecture in more detail, see the [Reference](./reference.md) section.

You may also want to explore:

- [the source on GitHub](https://github.com/emit-rs/emit).
- [a set of task-oriented examples](https://github.com/emit-rs/emit/tree/main/examples).
- [the API docs](https://docs.rs/emit/1.8.1/emit/index.html).
