# Emitting to the console

You can use [`emit_term`](https://docs.rs/emit_term/0.11.9/emit_term/index.html) to write diagnostic events to the console in a human-readable format:

```toml
[dependencies.emit_term]
version = "0.11.9"
```

```rust
# extern crate emit;
# extern crate emit_term;
fn main() {
    let rt = emit::setup().emit_to(emit_term::stdout()).init();

    // Your app code goes here

    rt.blocking_flush(std::time::Duration::from_secs(5));
}
```

See [the crate docs](https://docs.rs/emit_term/0.11.9/emit_term/index.html) for more details.

## Format

Events are written with a header containing the timestamp, level, and emitting package name, followed by the rendered message template:

```rust
# extern crate emit;
emit::info!("Hello, {user}", user: "Rust");
```

![`emit_term` output for the above program](../asset/term-log.png)

If the event contains an error (the well-known `err` property), then it will be formatted as a cause chain after the message:

```rust
# extern crate emit;
# let err = "";
emit::warn!("writing to {path} failed", path: "./file.txt", err);
```

![`emit_term` output for the above program](../asset/term-err.png)

If the event is part of a trace, the trace and span ids will be written in the header with corresponding colored boxes derived from their values:

```rust
# extern crate emit;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
#[emit::info_span(err_lvl: "warn", "write to {path}")]
fn write_to_file(path: &str, data: &[u8]) -> std::io::Result<()> {
# /*
    ..
# */

    emit::debug!("wrote {bytes} bytes to the file", bytes: data.len());

    Ok(())
}

write_to_file("./file.txt", b"Hello")?;
# Ok(())
# }
```

![`emit_term` output for the above program](../asset/term-span.png)

## Writing your own console emitter

The `emit_term` [source code](https://github.com/emit-rs/emit/blob/main/emitter/term/src/lib.rs) is written to be hackable. You can take and adapt its source to your needs, or write your own emitter that formats events the way you'd like. See [Writing an emitter](../for-developers/writing-an-emitter.md) for details.
