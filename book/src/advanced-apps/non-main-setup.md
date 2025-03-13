# Setup outside of `main`

`emit` is typically configured in your `main` function, but that might not be feasible for some applications. In these cases, you can run `emit`'s setup in a function and flush it deliberately at some later point:

```rust
# extern crate emit;
# extern crate emit_term;
fn diagnostics_init() {
    let _ = emit::setup()
        .emit_to(emit_term::stdout())
        .try_init();
}

fn diagnostics_flush() {
    emit::blocking_flush(std::time::Duration::from_secs(5));
}
```

Calling [`try_init()`](https://docs.rs/emit/1.2.0/emit/setup/struct.Setup.html#method.try_init) ensures you don't panic even if setup is called multiple times.

`emit` doesn't automatically flush or de-initialize its runtime when [`Init`](https://docs.rs/emit/1.2.0/emit/setup/struct.Init.html) goes out of scope so it's safe to let it drop before your application exits. 
