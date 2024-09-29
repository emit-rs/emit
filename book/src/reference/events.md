# Event data model

All diagnostics in `emit` are represented as an [`Event`](https://docs.rs/emit/0.11.0-alpha.17/emit/struct.Event.html).

Each event is the combination of:

- `mdl` ([`Path`](https://docs.rs/emit/0.11.0-alpha.17/emit/struct.Path.html)): The path of the component that generated the event.
- `tpl` ([`Template`](https://docs.rs/emit/0.11.0-alpha.17/emit/struct.Template.html)): A lazily-rendered, user-facing description of the event.
- `extent` ([`Extent`](https://docs.rs/emit/0.11.0-alpha.17/emit/struct.Extent.html)): The point in time that the event occurred at, or the span of time for which it was active.
- `props` ([`Props`](https://docs.rs/emit/0.11.0-alpha.17/emit/trait.Props.html)): A set of key-value pairs associated with the event.

Here's an example of an event created using the [`emit!`](https://docs.rs/emit/0.11.0-alpha.17/emit/macro.emit.html) macro:

```rust
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

The core event data model doesn't encode any specific diagnostic paradigm. It doesn't even include log levels. Events use well-known properties for 

Well-known props and `evt_kind`.

## Value data model

`serde`, `sval` -> `Value` data model

## Extents and timestamps

Points vs ranges

## Constructing events without macros

Events don't have to be constructed using macros. You can use the [`Event::new`](https://docs.rs/emit/0.11.0-alpha.17/emit/struct.Event.html#method.new) constructor manually:

```rust
let evt = emit::Event::new(
    // mdl
    emit::Path::new("shop::orders::add_to_cart").unwrap(),
    // tpl
    emit::Template::new(&[
        emit::template::Part::hole("user"),
        emit::template::Part::text(" added "),
        emit::template::Part::hole("item"),
        emit::template::Part::text(" to their cart"),
    ]),
    // extent
    emit::Timestamp::from_str("2024-01-02T03:04:05.678Z").unwrap(),
    // props
    [
        ("user", "user-123"),
        ("item", "product-456"),
    ]
);
```
