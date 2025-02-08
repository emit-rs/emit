# Event data model

All diagnostics in `emit` are represented as an [`Event`](https://docs.rs/emit/0.11.9/emit/struct.Event.html).

Each event is the combination of:

- `mdl` ([`Path`](https://docs.rs/emit/0.11.9/emit/struct.Path.html)): The path of the component that generated the event.
- `tpl` ([`Template`](https://docs.rs/emit/0.11.9/emit/struct.Template.html)): A lazily-rendered, user-facing description of the event.
- `extent` ([`Extent`](https://docs.rs/emit/0.11.9/emit/struct.Extent.html)): The point in time that the event occurred at, or the span of time for which it was active.
- `props` ([`Props`](https://docs.rs/emit/0.11.9/emit/trait.Props.html)): A set of key-value pairs associated with the event.

Here's an example of an event created using the [`emit!`](https://docs.rs/emit/0.11.9/emit/macro.emit.html) macro:

```rust
# extern crate emit;
let user = "user-123";
let item = "product-456";

emit::emit!("{user} added {item} to their cart");
```

This event will have:

- `mdl`: The path of the module that called `emit!`, like `shop::orders::add_to_cart`.
- `tpl`: The raw template. In this case it's `"{user} added {item} to their cart"`. When rendered, the template will produce _user-123 added product-456 to their cart_.
- `extent`: The time when `emit!` was called, like `2024-01-02T03:04:05.678Z`. Extents may also be a range. See [Extents and timestamps](#extents-and-timestamps) for details.
- `props`: Any properties referenced in or after the template. In this case it's `user` and `item`, so the properties are `{ user: "user-123", item: "product-456" }`. Property values aren't restricted to strings, they can be any primitive or complex type. See [Value data model](#value-data-model) for details.

## Extensions

The core event data model doesn't encode any specific diagnostic paradigm. It doesn't even include log levels. `emit` uses well-known properties to support extensions to its data model. A well-known property is a reserved name and set of allowed values that consumers of diagnostic data can use to treat an event as something more specific. See the [`well_known`](https://docs.rs/emit/0.11.9/emit/well_known/index.html) module for a complete list of well-known properties.

The two main extensions to the event data model are [tracing](../producing-events/tracing/data-model.md), and [metrics](../producing-events/metrics/data-model.md). You can also define your own extensions. These extensions are both based on the [`evt_kind`](https://docs.rs/emit/0.11.9/emit/well_known/constant.KEY_EVT_KIND.html) well-known property. Consumers that aren't specially aware of it will treat unknown extended events as regular ones.

## Value data model

The [`Value`](https://docs.rs/emit/0.11.9/emit/struct.Value.html) type is `emit`'s representation of an anonymous structured value based on the [`value_bag`](https://docs.rs/value_bag) library. `Value` is a concrete type rather than a trait to make working with them in [`Props`](https://docs.rs/emit/0.11.9/emit/trait.Props.html) easier. Internally, a value holds a direct reference or embedded primitive value for:

- **Integers:** `i8`-`i128`, `u8`-`u128`.
- **Binary floating points:** `f32`-`f64`.
- **Booleans:** `bool`.
- **Strings:** `&'v str`.

Values can also store more complex types by embedding references implementing a trait from a serialization framework:

- **Standard formatting:** `std::fmt::Debug`, `std::fmt::Display`.
- **Serde:** `serde::Serialize`.
- **Sval:** `sval::Value`.

A value can always be formatted or serialized using any of the above frameworks, regardless of whatever might be embedded in it, in the most structure-preserving way. That means if you embed an enum using `serde::Serialize` you can still serialize it as an enum using the `sval::Value` implementation on `Value`.

## Extents and timestamps

The time-oriented part of an event is its [`Extent`](https://docs.rs/emit/0.11.9/emit/struct.Extent.html). Internally, an extent stores [`Timestamp`](https://docs.rs/emit/0.11.9/emit/struct.Timestamp.html)s. An extent can either store one or a pair of timestamps.

An extent that stores a single timestamp is called a point. These are used by log events and other events that represent a point-in-time observation.

An extent that stores a pair of timestamps is called a range. These are used by trace spans and other events that represent something happening over time.

## Constructing events without macros

Events don't have to be constructed using macros. You can use the [`Event::new`](https://docs.rs/emit/0.11.9/emit/struct.Event.html#method.new) constructor manually:

```rust
# extern crate emit;
let parts = [
    emit::template::Part::hole("user"),
    emit::template::Part::text(" added "),
    emit::template::Part::hole("item"),
    emit::template::Part::text(" to their cart"),
];

let evt = emit::Event::new(
    // mdl
    emit::path!("shop::orders::add_to_cart"),
    // tpl
    emit::Template::new_ref(&parts),
    // extent
    emit::Timestamp::try_from_str("2024-01-02T03:04:05.678Z").unwrap(),
    // props
    [
        ("user", "user-123"),
        ("item", "product-456"),
    ]
);
```
