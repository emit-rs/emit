# Tracing fallible functions

The `ok_lvl` and `err_lvl` control parameters can be applied to span macros to assign a level based on whether the annotated function returned `Ok` or `Err`:

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
