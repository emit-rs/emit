# Emitting to the console

You can use [`emit_term`](https://docs.rs/emit_term/1.3.0/emit_term/index.html) to write diagnostic events to the console in a human-readable format:

```toml
[dependencies.emit_term]
version = "1.3.0"
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

See [the crate docs](https://docs.rs/emit_term/1.3.0/emit_term/index.html) for more details.

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

## Emitting JSON

The `emit_term` [source code](https://github.com/emit-rs/emit/blob/main/emitter/term/src/lib.rs) is written to be hackable rather than configurable. It doesn't support changing its output into other formats, but you can write your own emitter that suits your specific needs. Here is an example of an emitter that writes minified JSON via [`serde_json`](docs.rs/serde_json) to the console using the [`println!`](https://doc.rust-lang.org/std/macro.println.html) macro:

```rust
# extern crate emit;
# extern crate serde;
# mod serde_json { pub fn to_string<T>(v: &T) -> Result<String, String> { Ok("".into()) } }
# use serde::Serialize;
# fn main() {
let rt = emit::setup()
    .emit_to(emit::emitter::from_fn(|evt| {
        use emit::Props as _;

        // Generics avoid needing to specify concrete types here
        #[derive(Serialize)]
        struct Event<E, M, R, P> {
            #[serde(flatten)]
            extent: E,
            mdl: M,
            msg: R,
            #[serde(flatten)]
            props: P,
        }

        let json = serde_json::to_string(&Event {
            // `as_map()` serializes the extent as a map with one or two keys:
            // `ts` for the end timestamp, and `ts_start` for the start, if there is one
            extent: evt.extent().as_map(),
            mdl: evt.mdl(),
            msg: evt.msg(),
            // `dedup()` ensures there are no duplicate properties
            // `as_map()` serializes properties as a map where each property is a key-value pair
            props: evt.props().dedup().as_map(),
        })
        .unwrap();

        println!("{json}");
    }))
    .init();

let user = "Rust";

emit::info!("Hello, {user}");

rt.blocking_flush(std::time::Duration::from_secs(5));
# }
```

```json
{"ts":"2025-03-02T08:13:29.497557000Z","mdl":"emit_json","msg":"Hello, Rust","lvl":"info","user":"Rust"}
```

Note that you'll need to enable the `serde` Cargo feature of `emit`.

Instead of using `println!` here, you could adapt the [source of `emit_term`](https://github.com/emit-rs/emit/blob/main/emitter/term/src/lib.rs). See [Writing an emitter](../for-developers/writing-an-emitter.md) for details.
