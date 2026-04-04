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

### `E0597` or `E0521` when capturing properties

When capturing a property that doesn't satisfy `'static` (such as `std::panic::Location<'a>`), you'll encounter this error by default:

```rust,ignore
emit::emit!("template {x}");
```

```text
error[E0521]: borrowed data escapes outside of function
  --> src/compile_fail/std/emit_props_non_static.rs:10:28
   |
 9 | pub fn exec(x: &InternalRef<'_>) {
   |             -
   |             |
   |             `x` is a reference that is only valid in the function body
   |             has type `&InternalRef<'1>`
10 |     emit::emit!("template {x}");
   |                            ^
   |                            |
   |                            `x` escapes the function body here
   |                            argument requires that `'1` must outlive `'static`
```

Resolve it by using a capturing attribute explicitly on that property so `emit` won't try and downcast it:

```rust,ignore
emit::emit!("template {#[emit::as_display] x}");
```

### `E0277` when capturing `Option`

When capturing a property wrapped in an `Option` by default, you'll encounter this error:

```rust,ignore
let x = Some("some data");

emit::emit!("template {x}");
```

```text
error[E0277]: capturing requires `Option<&str>` implements `Display + Any` by default. If this value does implement `Display`, then dereference or annotate it with `#[emit::as_display]`. If it doesn't, then use one of the `#[emit::as_*]` attributes to capture this value using a trait it does implement.
 --> src/compile_fail/std/emit_props_non_optional.rs:4:28
  |
4 |     emit::emit!("template {x}");
  |                            ^ the trait `std::fmt::Display` is not implemented for `Option<&str>`
```

Resolve it by adding the `#[optional]` attribute on that property:

```rust,ignore
let x = Some("some data");

emit::emit!("template {#[emit::optional] x}");
```

You may also need to add `.as_ref()`, to ensure the property is `Option<&T>`, not `&Option<T>`:

```rust,ignore
let x = Some(42);

emit::emit!("template {#[emit::optional] x: x.as_ref()}");
```