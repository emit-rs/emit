# Emitting events

Diagnostic events produced by `emit` are sent to an [`Emitter`](https://docs.rs/emit/0.11.0-alpha.17/emit/trait.Emitter.html). `emit` provides a few implementations in external libraries you can use in your applications:

- [`emit_term`](https://docs.rs/emit_term/0.11.0-alpha.17/emit_term/index.html) for [emitting to the console](./emitting-events/console.md).
- [`emit_file`](https://docs.rs/emit_file/0.11.0-alpha.17/emit_file/index.html) for [emitting to rolling files](./emitting-events/rolling-files.md).
- [`emit_otlp`](https://docs.rs/emit_otlp/0.11.0-alpha.17/emit_otlp/index.html) for [emitting via OTLP](./emitting-events/otlp.md).

## Setup

Emitters are configured through the [`setup`](https://docs.rs/emit/0.11.0-alpha.17/emit/setup/fn.setup.html) function at the start of your application by calling [`emit_to`](https://docs.rs/emit/0.11.0-alpha.17/emit/setup/struct.Setup.html#method.emit_to):

```rust
# extern crate emit;
fn main() {
    let rt = emit::setup()
        // Set the emitter
        .emit_to(emit_term::stdout())
        .init();

    // Your app code goes here

    rt.blocking_flush(std::time::Duration::from_secs(5));
}
```

Once initialized, any subsequent calls to [`init`](https://docs.rs/emit/0.11.0-alpha.17/emit/setup/struct.Setup.html#method.init) will panic.

`emit_to` will replace any previously set emitter during the same setup. You can set multiple emitters by calling [`and_emit_to`](https://docs.rs/emit/0.11.0-alpha.17/emit/setup/struct.Setup.html#method.and_emit_to):

```rust
# extern crate emit;
fn main() {
    let rt = emit::setup()
        // Set multiple emitters
        .emit_to(emit_term::stdout())
        .and_emit_to(emit_file::set("./target/logs/my_app.txt").spawn())
        .init();

    // Your app code goes here

    rt.blocking_flush(std::time::Duration::from_secs(5));
}
```

You can map an emitter to a new value by calling [`map_emitter`](https://docs.rs/emit/0.11.0-alpha.17/emit/setup/struct.Setup.html#method.map_emitter):

```rust
# extern crate emit;
# use emit::Emitter;
fn main() {
    let rt = emit::setup()
        // Set the emitter
        .emit_to(emit_file::set("./target/logs/my_app.txt").spawn())
        // Map the emitter, wrapping it with a transformation that
        // sets the module to "new_path". This could be done in the call
        // to `emit_to`, but may be easier to follow when split across two calls
        .map_emitter(|emitter| emitter
            .wrap_emitter(emit::emitter::wrapping::from_fn(|emitter, evt| {
                let evt = evt.with_mdl(emit::Path::new_unchecked("new_path"));
                
                emitter.emit(evt)
            })
        )
        .init();

    // Your app code goes here

    rt.blocking_flush(std::time::Duration::from_secs(5));
}
```
