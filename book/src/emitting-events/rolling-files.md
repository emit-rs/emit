# Emitting to rolling files

You can use [`emit_file`](https://docs.rs/emit_file/1.16.2/emit_file/index.html) to write diagnostic events to local rolling files:

```toml
[dependencies.emit_file]
version = "1.16.2"
```

```rust
# extern crate emit;
# extern crate emit_file;
fn main() {
    let rt = emit::setup()
        .emit_to(emit_file::set("./target/logs/my_app.txt").spawn())
        .init();

    // Your app code goes here

    rt.blocking_flush(std::time::Duration::from_secs(5));
}
```

Events will be written in newline-delimited JSON by default:

```json
{"ts_start":"2024-05-29T03:35:13.922768000Z","ts":"2024-05-29T03:35:13.943506000Z","module":"my_app","msg":"in_ctxt failed with `a` is odd","tpl":"in_ctxt failed with `err`","a":1,"err":"`a` is odd","lvl":"warn","span_id":"0a3686d1b788b277","span_parent":"1a50b58f2ef93f3b","trace_id":"8dd5d1f11af6ba1db4124072024933cb"}
```

`emit_file` is a robust, asynchronous file writer that can recover from IO errors and manage the size of your retained logs on-disk.

See [the crate docs](https://docs.rs/emit_file/1.16.2/emit_file/index.html) for more details.
