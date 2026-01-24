# Writing an emitter

You can write a simple emitter using [`emitter::from_fn`](https://docs.rs/emit/1.16.1/emit/emitter/fn.from_fn.html), but advanced cases need to implement the [`Emitter`](https://docs.rs/emit/1.16.1/emit/trait.Emitter.html) trait.

For a complete implementation, see [the source for `emit_file`](https://github.com/emit-rs/emit/blob/main/emitter/file/src/lib.rs).

## Dependencies

If you're writing a library with an emitter, you can depend on `emit` without default features:

```toml
[dependencies.emit]
version = "1.16.1"
# Always disable default features
default-features = false
# Add any features you need
features = ["implicit_internal_rt"]
```

## Internal diagnostics

If your emitter is complex enough to need its own diagnostics, you can add the `implicit_internal_rt` feature of `emit` and use it when calling [`emit!`](https://docs.rs/emit/1.16.1/emit/macro.emit.html) or [`#[span]`](https://docs.rs/emit/1.16.1/emit/attr.span.html):

```rust
# extern crate emit;
# let err = "";
emit::warn!(rt: emit::runtime::internal(), "failed to emit an event: {err}");
```

Your emitter _must not_ write diagnostics to the default runtime. If you disabled default features when adding `emit` to your `Cargo.toml` then this will be verified for you at compile-time.

## Metrics

A standard pattern for emitters is to expose a function called `metric_source` that exposes a [`Source`](https://docs.rs/emit/1.16.1/emit/metric/source/trait.Source.html) with any metrics for your emitter. See [this example from `emit_file`](https://docs.rs/emit_file/1.16.1/emit_file/struct.FileSet.html#method.metric_source).

## Background processing

Emitters should minimize their impact in calling code by offloading expensive processing to a background thread. You can use [`emit_batcher`](https://docs.rs/emit_batcher/1.16.1/emit_batcher/index.html) to implement a batching, retrying, asynchronous emitter.
