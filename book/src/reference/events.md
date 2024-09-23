# Event data model

All diagnostics in `emit` are represented as an [`Event`](https://docs.rs/emit/0.11.0-alpha.17/emit/struct.Event.html).

Each event is the combination of:

- `mdl` ([`Path`](https://docs.rs/emit/0.11.0-alpha.17/emit/struct.Path.html)): The path of the component that generated the event.
- `tpl` ([`Template`](https://docs.rs/emit/0.11.0-alpha.17/emit/struct.Template.html)): A lazily-rendered, user-facing description of the event.
- `extent` ([`Extent`](https://docs.rs/emit/0.11.0-alpha.17/emit/struct.Extent.html)): The point in time that the event occurred at, or the span of time for which it was active.
- `props` ([`Props`](https://docs.rs/emit/0.11.0-alpha.17/emit/trait.Props.html)): A set of key-value pairs associated with the event.

## Extensions

Well-known props and `evt_kind`.

## Value data model

`serde`, `sval` -> `Value` data model

## Extents and timestamps

Points vs ranges
