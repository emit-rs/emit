# `emit`

## Developer-first diagnostics for Rust applications

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
