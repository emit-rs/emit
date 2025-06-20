# Control parameters

Field values that appear before the template literal in [`emit!`](https://docs.rs/emit/1.11.0/emit/macro.emit.html) or [`#[span]`](https://docs.rs/emit/1.11.0/emit/attr.span.html) aren't captured as properties. They're used to control the behavior of the code generated by the macro. The set of valid control parameters and their types is different for each macro.

## `emit!`

See [the crate docs](https://docs.rs/emit/1.11.0/emit/macro.emit.html#control-parameters) for control parameters on `emit!`.

## `#[span]`

See [the crate docs](https://docs.rs/emit/1.11.0/emit/attr.span.html#control-parameters) for control parameters on `#[span]`.
