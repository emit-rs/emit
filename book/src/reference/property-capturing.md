# Property capturing

`emit` supports fully structured properties through the [`Value`](https://docs.rs/emit/1.0.1/emit/struct.Value.html) type. Those properties don't have to implement any traits defined by `emit` itself. It instead leans on other popular serialization frameworks. See [Value data model](./events.md#value-data-model) for more details.

When a property value is captured in a call to [`emit!`](https://docs.rs/emit/1.0.1/emit/macro.emit.html) or [`#[span]`](https://docs.rs/emit/1.0.1/emit/attr.span.html) by default, it needs to satisfy `Display + 'static`. If the type of the property value is a primitive like an `i32`, `bool`, or `str`, then it will be stored directly as that type. `Copy` primitives are stored by-value. All other values are stored by-ref.

You can change the default `Display + 'static` bound [using attributes prefixed with `as_`](https://docs.rs/emit/1.0.1/emit/attr.span.html?search=attr%3Aas_) on them.
