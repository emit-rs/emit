# In tests

`emit` can be configured in `#[test]` functions using the [`#[span]`](https://docs.rs/emit/1.20.1/emit/attr.span.html) attribute. There are several useful [control parameters](../reference/control-parameters.md) you can use to improve diagnostics in tests:

- `setup`: Run some code at the start of the annotated function. We can use `setup` in tests to configure the `emit` runtime. When multiple tests share the same setup the first to execute will initialize the runtime.
- `fn_name`: Include a property with the name of the annotated function. We can use `fn_name` to distinguish tests in the output.
- `catch_unwind`: Wrap the annotated function in `catch_unwind`. We can use `catch_unwind` to make assertion failures appear on the resulting span.

Here's a simple example of some `emit` testing infrastructure:

```rust
# extern crate emit;
# extern crate emit_term;
// This is the piece of code we're going to test
pub fn add(a: i32, b: i32) -> i32 {
    let r = a + b;

    emit::debug!("{r} = {a} + {b}");

    r
}

// A function called at the start of each `#[test]`
#[cfg(test)]
fn setup() -> Option<impl Drop> {
	// Configure `emit`'s runtime
    let rt = emit::setup()
        .emit_to(emit_term::stdout())
        .try_init()
        .map(|init| init.flush_on_drop(std::time::Duration::from_secs(1)));

    // Set a panic hook so the location of the panic will also be captured
    // We only need to do this once
    if rt.is_some() {
        std::panic::set_hook(Box::new(|payload| {
            emit::error!("panic detected", #[emit::as_display] err: payload);
        }));
    }

    rt
}

// Annotate test functions with `emit::span`, including the `setup` control parameter
// for our `setup` function above
#[test]
#[emit::span(setup, fn_name, "test {fn_name}")]
fn add_1_1() {
    assert_eq!(2, add(1, 1));
}
```

The `setup` function in the above example uses [`emit_term`](https://docs.rs/emit_term/1.20.1/emit_term/index.html) as its emitter. Since it uses `stdout` directly, it bypasses Rust's default test harness behavior of capturing output for passing tests. To preserve the default capturing, you can instead emit events with `println!`s:

```rust
# extern crate emit;
fn setup() -> Option<impl Drop> {
	emit::setup()
        .emit_to(emit::emitter::from_fn(|evt| println!("{evt:?}")))
        .try_init()
        .map(|init| init.flush_on_drop(std::time::Duration::from_secs(1)))
}
```

The `setup` function also uses `panic::set_hook` to capture panics from failed assertions instead of adding the `catch_unwind` control parameter to the `#[span]` attribute. Rust's panic hooks are given more information about a panic than `catch_unwind`, so using a hook means we still get the location where the panic was first thrown. It might not be practical for all applications to use a panic hook this way, in which case you can still get the panic payload with the `catch_unwind` control parameter:

```rust
# extern crate emit;
# extern crate emit_term;
# pub fn add(a: i32, b: i32) -> i32 { a + b }
# fn setup() {}
#[test]
#[emit::span(setup, catch_unwind: true, fn_name, "test {fn_name}")]
fn add_1_1() {
    assert_eq!(2, add(1, 1));
}
```
