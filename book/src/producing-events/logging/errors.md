# Attaching errors to events

In Rust, errors are typically communicated through the [`Error`](https://doc.rust-lang.org/std/error/trait.Error.html) trait. If you attach a property with the `err` [well-known property](https://docs.rs/emit/0.11.0-alpha.19/emit/well_known/index.html) to an event, it will automatically try capture it using its `Error` implementation:

```rust
# extern crate emit;
# fn write_to_file(bytes: &[u8]) -> std::io::Result<()> { Err(std::io::Error::new(std::io::ErrorKind::Other, "the file is in an invalid state")) }
if let Err(err) = write_to_file(b"Hello") {
    emit::warn!("file write failed: {err}");
}
```

```text
Event {
    mdl: "emit_sample",
    tpl: "file write failed: {err}",
    extent: Some(
        "2024-10-02T21:14:40.566303000Z",
    ),
    props: {
        "err": Custom {
            kind: Other,
            error: "the file is in an invalid state",
        },
        "lvl": warn,
    },
}
```

Emitters may treat the `err` property specially when receiving diagnostic events, such as by displaying them more prominently.

You can also use the [`#[as_error]`](https://docs.rs/emit/0.11.0-alpha.19/emit/attr.as_error.html) attribute on a property to capture it using its `Error` implementation.

The [`#[span]`](https://docs.rs/emit/0.11.0-alpha.19/emit/attr.span.html) macro can automatically capture errors from fallible functions. See [Fallible functions](../tracing/fallible-functions.md) for details.
