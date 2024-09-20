# Getting started

Add `emit` to your `Cargo.toml`, along with an _emitter_ to write diagnostics to:

```toml
[dependencies.emit]
version = "0.11.0-alpha.17"

[dependencies.emit_term]
version = "0.11.0-alpha.17"
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

## Logging events

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

## Tracing functions

Add `#[emit::span]` to a significant function in your application to trace its execution:

```rust
# extern crate emit;
#[emit::span("add {item} to {user} cart")]
async fn add_item(user: &str, item: &str) {
    // Your code goes here
}
```

Any diagnostics emitted within a traced function will be correlated with it. Any other traced functions it calls will form a trace hierarchy.
