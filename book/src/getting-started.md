# Getting started

Add `emit` to your `Cargo.toml`, along with an _emitter_ to write diagnostics to:

```toml
[dependencies.emit]
version = "1.17.2"

[dependencies.emit_term]
version = "1.17.2"
```

Initialize `emit` at the start of your `main.rs` using [`emit::setup()`](https://docs.rs/emit/1.17.2/emit/setup/index.html), and ensure any emitted diagnostics are flushed by calling [`blocking_flush()`](https://docs.rs/emit/1.17.2/emit/setup/struct.Init.html#method.blocking_flush) at the end:

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

You can configure `emit` to write to [OpenTelemetry](https://docs.rs/emit_otlp/1.17.2/emit_otlp/index.html), [the console](https://docs.rs/emit_term/1.17.2/emit_term/index.html), [rolling files](https://docs.rs/emit_file/1.17.2/emit_file/index.html), or any custom emitter you create.

Start peppering diagnostics through your application with `emit`'s macros.

## Logging events

When something of note happens, use [`debug!`](https://docs.rs/emit/1.17.2/emit/macro.debug.html) or [`info!`](https://docs.rs/emit/1.17.2/emit/macro.info.html) to log it:

```rust
# extern crate emit;
let user = "user-123";
let item = "product-123";

emit::info!("{user} added {item} to their cart");
```

When something fails, use [`warn!`](https://docs.rs/emit/1.17.2/emit/macro.warn.html) or [`error!`](https://docs.rs/emit/1.17.2/emit/macro.error.html):

```rust
# extern crate emit;
# let user = "user-123";
let err = std::io::Error::new(
    std::io::ErrorKind::Other,
    "failed to connect to the remote service",
);

emit::warn!("updating {user} cart failed: {err}");
```

### Macro syntax

`emit`'s syntax is compatible with `std::fmt` in simple cases, but much more capable. It uses field-value syntax to name properties captured from local variables:

```rust
# extern crate emit;
emit::info!(
    "{user} added {item} to their cart",
    user: "user-123",
    item: "product-123",
);
```

### Structured data

`emit` captures properties using their `Display` implementation by default with special handling for booleans and numbers. It uses attribute syntax to customize how properties are captured, such as using `Serialize` instead:

```toml
[dependencies.emit]
version = "1.17.2"
features = ["serde"]
```

```rust
# extern crate emit;
# extern crate serde;
# use serde::Serialize;
#[derive(Serialize)]
struct User<'a> {
    id: &'a str,
    name: &'a str,
}

let user = User {
    id: "user-123",
    name: "Some User",
};

emit::info!(
    "{#[emit::as_serde] user} added {item} to their cart",
    item: "product-123",
);
```

## Tracing functions

Add [`#[span]`](https://docs.rs/emit/1.17.2/emit/attr.span.html) to a significant function in your application to trace its execution:

```rust
# extern crate emit;
#[emit::span("add {item} to {user} cart")]
async fn add_item(user: &str, item: &str) {
    // Your code goes here
}
```

Any diagnostics emitted within a traced function will be correlated with it. Any other traced functions it calls will form a trace hierarchy.

`emit`'s tracing attributes use the same syntax and capturing rules as log events.

## Sampling metrics

Use [`sample!`](https://docs.rs/emit/1.17.2/emit/macro.sample.html) to write samples of the metrics your application tracks as events:

```rust
# extern crate emit;
let bytes_written = 417;

emit::sample!(value: bytes_written, agg: "count");
```

## Quick debugging

Use the [`dbg!`](https://docs.rs/emit/1.17.2/emit/macro.dbg.html) macro to help debug code as you're writing it:

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

To learn `emit`'s architecture and syntax in more detail, see the [Reference](./reference.md) section.

You may also want to explore:

- [the source on GitHub](https://github.com/emit-rs/emit).
- [a set of task-oriented examples](https://github.com/emit-rs/emit/tree/main/examples).
- [the API docs](https://docs.rs/emit/1.17.2/emit/index.html).
