# `emit`

## Developer-first diagnostics for Rust applications

`emit` is a framework for manually instrumenting Rust applications using an expressive syntax inspired by [Message Templates](https://messagetemplates.org).

```rust
extern crate emit;
extern crate emit_term;

fn main() {
    let rt = emit::setup().emit_to(emit_term::stdout()).init();

    let user = "World";

    emit::info!("Hello, {user}!");

    rt.blocking_flush(std::time::Duration::from_secs(5));
}
```
