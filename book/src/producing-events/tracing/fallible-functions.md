# Tracing fallible functions

The `ok_lvl` and `err_lvl` [control parameters](../../reference/control-parameters.md) can be applied to span macros to assign a level based on whether the annotated function returned `Ok` or `Err`:

```rust
# extern crate emit;
#[emit::span(
    ok_lvl: emit::Level::Info,
    err_lvl: emit::Level::Error,
    "wait a bit",
    sleep_ms,
)]
fn wait_a_bit(sleep_ms: u64) -> Result<(), std::io::Error> {
    if sleep_ms > 500 {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "the wait is too long"));
    }

    std::thread::sleep(std::time::Duration::from_millis(sleep_ms));

    Ok(())
}

let _ = wait_a_bit(100);
let _ = wait_a_bit(1200);
```

```text
Event {
    mdl: "my_app",
    tpl: "wait a bit",
    extent: Some(
        "2024-06-12T21:43:03.556361000Z".."2024-06-12T21:43:03.661164000Z",
    ),
    props: {
        "lvl": info,
        "evt_kind": span,
        "span_name": "wait a bit",
        "trace_id": 6a3fc0e46bfa1da71537e39e3bf1942c,
        "span_id": f5bcc5821c6c3227,
        "sleep_ms": 100,
    },
}
Event {
    mdl: "my_app",
    tpl: "wait a bit",
    extent: Some(
        "2024-06-12T21:43:03.661850000Z".."2024-06-12T21:43:03.661986000Z",
    ),
    props: {
        "lvl": error,
        "err": Custom {
            kind: Other,
            error: "the wait is too long",
        },
        "evt_kind": span,
        "span_name": "wait a bit",
        "trace_id": 3226b70b45ff90f92f4feccee4325d4d,
        "span_id": 3702ba2429f9a7b7,
        "sleep_ms": 1200,
    },
}
```

## Mapping error types

Attaching errors to spans requires they're either [`&str`](https://doc.rust-lang.org/std/primitive.str.html), [`&(dyn std::error::Error + 'static)`](https://doc.rust-lang.org/std/error/trait.Error.html#impl-dyn+Error), or [`impl std::error::Error`](https://doc.rust-lang.org/std/error/trait.Error.html). Error types like [`anyhow::Error`](https://docs.rs/anyhow/latest/anyhow/) don't satisfy these requirements so need to be mapped.

The `err` [control parameter](../../reference/control-parameters.md) can be used to map the error type of a fallible span into one that can be captured:

```rust
# extern crate emit;
# extern crate anyhow;
#[emit::span(
    ok_lvl: emit::Level::Info,
    err: emit::err::as_ref,
    "wait a bit",
    sleep_ms,
)]
fn wait_a_bit(sleep_ms: u64) -> Result<(), anyhow::Error> {
    if sleep_ms > 500 {
        return Err(anyhow::Error::msg("the wait is too long"));
    }

    std::thread::sleep(std::time::Duration::from_millis(sleep_ms));

    Ok(())
}

let _ = wait_a_bit(100);
let _ = wait_a_bit(1200);
```

The `err` control parameter accepts an expression that implements `Fn(&E) -> U`, which can either be provided as a closure inline, or as an external function like [`emit::err::as_ref`](https://docs.rs/emit/1.17.1/emit/err/fn.as_ref.html) in the above example.

If your error type can't be mapped, you can also fall back to just providing a static string description as the error value:

```rust
# extern crate emit;
#[emit::span(
    ok_lvl: emit::Level::Info,
    err: (|_| "wait a bit failed"),
    "wait a bit",
    sleep_ms,
)]
fn wait_a_bit(sleep_ms: u64) -> Result<(), ()> {
    if sleep_ms > 500 {
        return Err(());
    }

    std::thread::sleep(std::time::Duration::from_millis(sleep_ms));

    Ok(())
}

let _ = wait_a_bit(100);
let _ = wait_a_bit(1200);
```

## Panics

If a function annotated with `#[span]` panics, it will emit an event with an error level and an `err` property indicating a panic was observed. The `panic_lvl` [control parameter](../../reference/control-parameters.md) can be used to specify a different level in case of panics.
