# Working with events

The result of [producing an event](./producing-events.md) is an instance of the [`Event`](https://docs.rs/emit/0.11.0-alpha.21/emit/struct.Event.html) type. When [filtering](./filtering-events.md) or [emitting](./emitting-events.md) events, you may need to inspect or manipulate its fields and properties.

## Timestamp

- Extent

## Properties

Event properties are kept in a generic [`Props`](https://docs.rs/emit/0.11.0-alpha.21/emit/props/trait.Props.html) collection, which can be accessed through the [`props()`](https://docs.rs/emit/0.11.0-alpha.21/emit/struct.Event.html#method.props) method on the event.

### Finding properties

To find a property value by key, you can call [`get()`](https://docs.rs/emit/0.11.0-alpha.21/emit/props/trait.Props.html#method.get) on the event properties. If present, the returned [`Value`](https://docs.rs/emit/0.11.0-alpha.21/emit/value/struct.Value.html) can be used to further format, serialize, or cast the matching value:

```rust
# extern crate emit;
# fn get(evt: emit::Event<impl emit::Props>) {
if let Some(value) = evt.props().get("my_property") {
    // The value is a type-erased object implementing Display/Serialize
}
# }
```

### Casting properties

To find a property and cast it to a concrete type, like `str` or `i32`, you can call [`pull()`](https://docs.rs/emit/0.11.0-alpha.21/emit/props/trait.Props.html#method.pull) on the event properties:

```rust
# extern crate emit;
# fn get(evt: emit::Event<impl emit::Props>) {
if let Some::<emit::Str>(value) = evt.props().pull("my_property") {
    // The value is a borrowed string
}
# }
```

Any type implementing the [`FromValue`](https://docs.rs/emit/0.11.0-alpha.21/emit/value/trait.FromValue.html) trait can be pulled as a concrete value from the event properties.

When pulling string values, prefer [`emit::Str`](https://docs.rs/emit/0.11.0-alpha.21/emit/str/struct.Str.html) or `Cow<str>` over `&str`. The former will successfully cast even if the value needs buffering internally. The latter will only successfully cast if the original value was a borrowed string.
