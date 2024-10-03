# Emitting to the console

You can use [`emit_term`](https://docs.rs/emit_term/0.11.0-alpha.17/emit_term/index.html) to write diagnostic events to the console in a human-readable format:

```toml
[dependencies.emit_term]
version = "0.11.0-alpha.17"
```

```rust
fn main() {
    let rt = emit::setup().emit_to(emit_term::stdout()).init();

    // Your app code goes here

    rt.blocking_flush(std::time::Duration::from_secs(5));
}
```

Events are written with a header containing the timestamp, level, and emitting package name, followed by the rendered message template:

```rust
emit::info!("Hello, {user}", user: "Rust");
```

![`emit_term` output for the above program](../asset/term-log.png)

If the event contains an error (the well-known `err` property), then it will be formatted as a cause chain after the message:

```rust
emit::warn!("writing to {path} failed", path: "./file.txt", err);
```

![`emit_term` output for the above program](../asset/term-err.png)

If the event is part of a trace, the trace and span ids will be written in the header with corresponding colored boxes derived from their values:

```rust
#[emit::info_span(err_lvl: "warn", "write to {path}")]
fn write_to_file(path: &str, data: &[u8]) -> std::io::Result<()> {
# /*
    ..
# */

    emit::debug!("wrote {bytes} bytes to the file", bytes: data.len());

    Ok(())
}

write_to_file("./file.txt", b"Hello")?;
```

![`emit_term` output for the above program](../asset/term-span.png)

See [the crate docs](https://docs.rs/emit_term/0.11.0-alpha.17/emit_term/index.html) for more details.