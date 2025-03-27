# Instrumenting embedded applications

`emit` supports embedded and constrained environments, but isn't explicitly targeting them the way frameworks like [`defmt`](https://docs.rs/defmt/latest/defmt/) do.

To use `emit` in an embedded environment, you'll need to disable default crate features to avoid pulling in the standard library:

```toml
[dependencies.emit]
version = "1.5.0"
default-features = false
```

## Configuring a static runtime

`emit` abstracts everything a diagnostic pipeline needs within a [`Runtime`](../reference/architecture.md#runtimes). Using runtimes, you can customize the clock and source of randomness in environments that don't have an obvious default, or disable them when there isn't one.

```rust
# extern crate emit;
# fn main() {}
// Replace these with your own implementations, or leave as `emit::Empty`
// if they're not supported in your target environment
type Emitter = emit::Empty;
type Filter = emit::Empty;
type Ctxt = emit::Empty;
type Clock = emit::Empty;
type Rng = emit::Empty;

// Define a static runtime using the given components
static MY_RUNTIME: emit::runtime::Runtime<Emitter, Filter, Ctxt, Clock, Rng> = emit::runtime::Runtime::build(
    emit::Empty,
    emit::Empty,
    emit::Empty,
    emit::Empty,
    emit::Empty,
);
```

Static runtimes can also be used in other environments to avoid dynamic dispatch and improve performance.

## Using `emit!` and `#[span]`

Embedded environments need to specify a runtime explicitly in the [`emit!`](https://docs.rs/emit/1.5.0/emit/macro.emit.html) or [`#[span]`](https://docs.rs/emit/1.5.0/emit/attr.span.html) macros using the `rt` [control parameter](../reference/control-parameters.md):

```rust
# extern crate emit;
# static MY_RUNTIME: emit::runtime::Runtime<emit::Empty, emit::Empty, emit::Empty, emit::Empty, emit::Empty> = emit::runtime::Runtime::build(emit::Empty, emit::Empty, emit::Empty, emit::Empty, emit::Empty);
# fn main() {
let user = "Embedded";

// Use the static runtime, `MY_RUNTIME`, defined previously
emit::emit!(rt: MY_RUNTIME, "Hello, {user}");
# }
```
