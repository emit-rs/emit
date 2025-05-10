# Working with events

The result of [producing an event](./producing-events.md) is an instance of the [`Event`](https://docs.rs/emit/1.8.0/emit/struct.Event.html) type. When [filtering](./filtering-events.md) or [emitting](./emitting-events.md) events, you may need to inspect or manipulate its fields and properties.

## Timestamp

The time-oriented value of an event is called its extent. It can store either a single point-in-time timestamp or a time range. Use the [`extent()`](https://docs.rs/emit/1.8.0/emit/event/struct.Event.html#method.extent) method to get the extent:

```rust
# extern crate emit;
# fn get(evt: emit::Event<impl emit::Props>) {
if let Some(extent) = evt.extent() {
    // The event has an extent, which can be a single timestamp or a time range
}
# }
# fn main() {}
```

The returned [`Extent`](https://docs.rs/emit/1.8.0/emit/extent/struct.Extent.html) can then be inspected to get its timestamp as a [`Timestamp`](https://docs.rs/emit/1.8.0/emit/timestamp/struct.Timestamp.html) or time range as a `Range<Timestamp>`.

Extents can always be treated as a point-in-time timestamp:

```rust
# extern crate emit;
# fn get(extent: emit::Extent) {
// If the extent is a point-in-time, this will be the value
// If the extent is a time range, this will be the end bound
let as_timestamp = extent.as_point();
# }
# fn main() {}
```

An extent may also be a time range:

```rust
# extern crate emit;
# fn get(extent: emit::Extent) {
if let Some(range) = extent.as_range() {
    // The extent is a time range
} else {
    // The extent is a point-in-time
}
# }
# fn main() {}
```

## Properties

Event properties are kept in a generic [`Props`](https://docs.rs/emit/1.8.0/emit/props/trait.Props.html) collection, which can be accessed through the [`props()`](https://docs.rs/emit/1.8.0/emit/struct.Event.html#method.props) method on the event.

Any data captured on an event, as well as any ambient context at the point it was produced, will be available on its properties. This collection is also where [well-known properties](https://docs.rs/emit/1.8.0/emit/well_known/index.html) for extensions to `emit`'s data model will live.

### Finding properties

To find a property value by key, you can call [`get()`](https://docs.rs/emit/1.8.0/emit/props/trait.Props.html#method.get) on the event properties. If present, the returned [`Value`](https://docs.rs/emit/1.8.0/emit/value/struct.Value.html) can be used to further format, serialize, or cast the matching value:

```rust
# extern crate emit;
# fn get(evt: emit::Event<impl emit::Props>) {
use emit::Props;

if let Some(value) = evt.props().get("my_property") {
    // The value is a type-erased object implementing Display/Serialize
}
# }
# fn main() {}
```

### Casting properties

To find a property and cast it to a concrete type, like a string or `i32`, you can call [`pull()`](https://docs.rs/emit/1.8.0/emit/props/trait.Props.html#method.pull) on the event properties:

```rust
# extern crate emit;
# fn get(evt: emit::Event<impl emit::Props>) {
# use emit::Props;
if let Some::<emit::Str>(value) = evt.props().pull("my_property") {
    // The value is a string
}
# }
# fn main() {}
```

Any type implementing the [`FromValue`](https://docs.rs/emit/1.8.0/emit/value/trait.FromValue.html) trait can be pulled as a concrete value from the event properties.

You can also use the [`cast()`](https://docs.rs/emit/1.8.0/emit/struct.Value.html#method.cast) method on a value to try cast it to a given type:

```rust
# extern crate emit;
# fn get(evt: emit::Event<impl emit::Props>) {
# use emit::Props;
if let Some(value) = evt.props().get("my_property") {
    if let Some(value) = value.cast::<bool>() {
        // The value is a boolean
    }

    // The value is something else
}
# }
# fn main() {}
```

#### Casting to a string

When pulling string values, prefer [`emit::Str`](https://docs.rs/emit/1.8.0/emit/str/struct.Str.html), `Cow<str>`, or `String` over `&str`. Any of the former will successfully cast even if the value needs buffering internally. The latter will only successfully cast if the original value was a borrowed string.

#### Casting to an error

Property values can contain standard [`Error`](https://doc.rust-lang.org/std/error/trait.Error.html) values. To try cast a value to an implementation of the `Error` trait, you can call [`to_borrowed_error()`](https://docs.rs/emit/1.8.0/emit/struct.Value.html#method.to_borrowed_error):

```rust
# extern crate emit;
# fn get(evt: emit::Event<impl emit::Props>) {
# use emit::Props;
if let Some(err) = evt.props().get("err") {
    if let Some(err) = err.to_borrowed_error() {
        // The value is an error
    }

    // The value is something else
}
# }
# fn main() {}
```

You can also pull or cast the value to `&(dyn std::error::Error + 'static)`.

### Parsing properties

You can use the [`parse()`](https://docs.rs/emit/1.8.0/emit/struct.Value.html#method.parse) method on a value to try parse a concrete type implementing [`FromStr`](https://doc.rust-lang.org/std/str/trait.FromStr.html) from it:

```rust
# extern crate emit;
# fn get(evt: emit::Event<impl emit::Props>) {
# use emit::Props;
if let Some(value) = evt.props().get("ip") {
    if let Some::<std::net::IpAddr>(ip) = value.parse() {
        // The value is an IP address
    }

    // The value is something else
}
# }
# fn main() {}
```

### Iterating properties

Use the [`for_each()`](https://docs.rs/emit/1.8.0/emit/trait.Props.html#tymethod.for_each) method on the event properties to iterate over them. In this example, we iterate over all properties and build a list of their string representations from them:

```rust
# extern crate emit;
# use std::collections::BTreeMap;
# use emit::Props;
use std::ops::ControlFlow;
# fn get(evt: emit::Event<impl emit::Props>) {
let mut buffered = BTreeMap::<String, String>::new();

evt.props().for_each(|k, v| {
    if !buffered.contains_key(k.get()) {
        buffered.insert(k.into(), v.to_string());
    }

    ControlFlow::Continue(())
});
# }
# fn main() {}
```

The `for_each` method accepts a closure where the inputs are the property key as a [`Str`](https://docs.rs/emit/1.8.0/emit/str/struct.Str.html) and value as a [`Value`](https://docs.rs/emit/1.8.0/emit/value/struct.Value.html). The closure returns a [`ControlFlow`](https://doc.rust-lang.org/std/ops/enum.ControlFlow.html) to tell the property collection whether it should keep iterating or stop.

#### Deduplication

Property collections may contain duplicate values, which will likely be yielded when iterating via `for_each`. Properties are expected to be deduplicated by retaining _the first seen_ for a given key. You can use the [`dedup()`](https://docs.rs/emit/1.8.0/emit/trait.Props.html#method.dedup) method when working with properties to deduplicate them before yielding, but this may require internal allocation:

```rust
# extern crate emit;
# use std::collections::BTreeMap;
# use emit::Props;
use std::ops::ControlFlow;
# fn get(evt: emit::Event<impl emit::Props>) {
// This is the same example as before, but we know properties are unique
// thanks to `dedup`, so don't need a unique collection for them
let mut buffered = Vec::<(String, String)>::new();

evt.props().dedup().for_each(|k, v| {
    buffered.push((k.into(), v.to_string()));

    ControlFlow::Continue(())
});
# }
# fn main() {}
```

### Formatting properties

The [`Value`](https://docs.rs/emit/1.8.0/emit/value/struct.Value.html) type always implements [`Debug`](https://doc.rust-lang.org/std/fmt/trait.Debug.html) and [`Display`](https://doc.rust-lang.org/std/fmt/trait.Display.html) with a useful representation, regardless of the kind of value it holds internally.

#### Formatting errors

The [`Value`](https://docs.rs/emit/1.8.0/emit/value/struct.Value.html) type specializes its display implementation for errors. If the error has a source (from the [`Error::source()`](https://doc.rust-lang.org/std/error/trait.Error.html#method.source) method), then the lowest-level source will be formatted in parenthesis after the error itself. This is done to make the default stringification of errors more descriptive.

This only applies when formatting the `Value` containing the error, it doesn't affect the format of the error returned by [`Value::to_borrowed_error()`](https://docs.rs/emit/1.8.0/emit/struct.Value.html#method.to_borrowed_error).

### Serializing properties

When the `serde` Cargo feature is enabled, the [`Value`](https://docs.rs/emit/1.8.0/emit/value/struct.Value.html) type always implements [`serde::Serialize`](https://docs.rs/serde/latest/serde/trait.Serialize.html) trait in the most structure-preserving way, regardless of the kind of value it holds internally. The same is true of the `sval` Cargo feature and [`sval::Value`](https://docs.rs/sval/latest/sval/trait.Value.html).

## Data model

See [Event data model](./reference/events.md) for more details on the shape of `emit`'s events.
