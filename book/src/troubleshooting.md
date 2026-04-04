# Troubleshooting

Emitters write their own diagnostics to an alternative `emit` runtime, which you can configure via [`init_internal`](https://docs.rs/emit/1.17.2/emit/setup/struct.Setup.html#method.init_internal) to debug them:

```rust
# extern crate emit;
# extern crate emit_term;
# extern crate emit_file;
fn main() {
    // Configure the internal runtime before your regular setup
    let internal_rt = emit::setup()
        .emit_to(emit_term::stdout())
        .init_internal();

    // Run your regular `emit` setup
    let rt = emit::setup()
        .emit_to(emit_file::set("./target/logs/my_app.txt").spawn())
        .init();

    // Your app code goes here

    rt.blocking_flush(std::time::Duration::from_secs(5));

    // Flush the internal runtime after your regular setup
    internal_rt.blocking_flush(std::time::Duration::from_secs(5));
}
```

## Common errors

This section documents some common compile errors you might encounter when using `emit`.

### `E0597` when capturing properties

When capturing a property that doesn't satisfy `'static` (such as `std::panic::Location<'a>`), you'll encounter this error by default:

```rust,no_run
emit::emit!("template {x}");
```

```text
error[E0597]: `short_lived` does not live long enough
 --> src/compile_fail/std/emit_props_non_static.rs:5:25
  |
4 |     let short_lived = String::from("x");
  |         ----------- binding `short_lived` declared here
5 |     let x = InternalRef(&short_lived);
  |                         ^^^^^^^^^^^^ borrowed value does not live long enough
6 |
7 |     emit::emit!("template {x}");
  |                            - argument requires that `short_lived` is borrowed for `'static`
8 | }
  | - `short_lived` dropped here while still borrowed
  |
note: requirement that the value outlives `'static` introduced here
 --> $WORKSPACE/src/macro_hooks.rs
  |
  |         Self: CaptureWithDefault,
  |               ^^^^^^^^^^^^^^^^^^

```

Resolve it by using a capturing attribute explicitly on that property so `emit` won't try and downcast it:

```rust,no_run
emit::emit!("template {#[emit::as_display] x}");
```
