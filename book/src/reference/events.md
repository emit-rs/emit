# Event data model

All diagnostics in `emit` are represented as an [`Event`](https://docs.rs/emit/0.11.0-alpha.17/emit/struct.Event.html). The dependency graph of an event looks like this:

```mermaid
classDiagram
    direction RL
    Timestamp <.. Extent

    Str <.. Props
    Value <.. Props

    class Props {
        for_each(Fn(Str, Value))*
    }

    <<Trait>> Props

    Path <.. Event
    Props <.. Event
    Template <.. Event

    class Extent {
        as_point() Timestamp
        as_range() Option~Range~Timestamp~~
    }

    Extent <.. Event

    class Event {
        mdl() Path
        tpl() Template
        extent() Extent
        props() Props
    }

    click Event href "https://docs.rs/emit/0.11.0-alpha.17/emit/struct.Event.html"
    click Timestamp href "https://docs.rs/emit/0.11.0-alpha.17/emit/struct.Timestamp.html"
    click Extent href "https://docs.rs/emit/0.11.0-alpha.17/emit/struct.Extent.html"
    click Str href "https://docs.rs/emit/0.11.0-alpha.17/emit/struct.Str.html"
    click Value href "https://docs.rs/emit/0.11.0-alpha.17/emit/struct.Value.html"
    click Props href "https://docs.rs/emit/0.11.0-alpha.17/emit/trait.Props.html"
    click Template href "https://docs.rs/emit/0.11.0-alpha.17/emit/struct.Template.html"
    click Path href "https://docs.rs/emit/0.11.0-alpha.17/emit/struct.Path.html"
```

Each event is the combination of:

- `mdl` ([`Path`](https://docs.rs/emit/0.11.0-alpha.17/emit/struct.Path.html)): The path of the component that generated the event.
- `tpl` ([`Template`](https://docs.rs/emit/0.11.0-alpha.17/emit/struct.Template.html)): A lazily-rendered, user-facing description of the event.
- `extent` ([`Extent`](https://docs.rs/emit/0.11.0-alpha.17/emit/struct.Extent.html)): The point in time that the event occurred at, or the span of time for which it was active.
- `props` ([`Props`](https://docs.rs/emit/0.11.0-alpha.17/emit/trait.Props.html)): A set of key-value pairs associated with the event.

## Event construction and emission

When the [`emit!`](https://docs.rs/emit/0.11.0-alpha.17/emit/macro.emit.html) macro is called, an [`Event`](https://docs.rs/emit/0.11.0-alpha.17/emit/struct.Event.html) is constructed using features of the runtime before being emitted through it. The following diagram demonstrates this flow in more detail:

```mermaid
flowchart
    start((start)) --> macro["`<code>emit!('a {x}', y)</code>`"]

    macro --> tpl["`<code>Template('a {x}')</code>`"]
    macro --> macro_props["`<code>Props { x, y }</code>`"]
    
    ctxt{{"`<code>Ctxt::Current</code>`"}} --> ctxt_props["`<code>Props { z }</code>`"]
    
    props["`<code>Props { x, y, z }</code>`"]
    macro_props --> props
    ctxt_props --> props

    clock{{"`<code>Clock::now</code>`"}} --> ts["`<code>Timestamp</code>`"] --> extent["`<code>Extent::point</code>`"]

    mdl_path["`<code>module_path!()</code>`"] --> mdl["`<code>Path('a::b::c')</code>`"]

    event["`<code>Event</code>`"]
    props -- props --> event
    extent -- extent --> event
    tpl -- tpl --> event
    mdl -- mdl --> event

    filter{"`<code>Filter::matches</code>`"}

    event --> filter
    filter -- false --> filter_no(((discard)))

    emitter{{"`<code>Emitter::emit</code>`"}}

    filter -- true --> emitter

    emitter --> END(((end)))

    click macro href "https://docs.rs/emit/0.11.0-alpha.17/emit/macro.emit.html"

    click tpl href "https://docs.rs/emit/0.11.0-alpha.17/emit/struct.Template.html"

    click macro_props href "https://docs.rs/emit/0.11.0-alpha.17/emit/trait.Props.html"
    click ctxt_props href "https://docs.rs/emit/0.11.0-alpha.17/emit/trait.Props.html"
    click props href "https://docs.rs/emit/0.11.0-alpha.17/emit/trait.Props.html"

    click mdl href "https://docs.rs/emit/0.11.0-alpha.17/emit/struct.Path.html"

    click ts href "https://docs.rs/emit/0.11.0-alpha.17/emit/struct.Timestamp.html"
    click extent href "https://docs.rs/emit/0.11.0-alpha.17/emit/struct.Extent.html"

    click event href "https://docs.rs/emit/0.11.0-alpha.17/emit/struct.Event.html"

    click emitter href "https://docs.rs/emit/0.11.0-alpha.17/emit/trait.Emitter.html"
    click filter href "https://docs.rs/emit/0.11.0-alpha.17/emit/trait.Filter.html"
    click ctxt href "https://docs.rs/emit/0.11.0-alpha.17/emit/trait.Ctxt.html"
    click clock href "https://docs.rs/emit/0.11.0-alpha.17/emit/trait.Clock.html"
```

In the above diagram, runtime components have angled borders. When constructing an event, the runtime provides the current timestamp and any ambient context. When emitting an event, the runtime filters out events to discard and emits the ones that remain.

See [Architecture](./architecture.md) for details on what these components are.

## Extensions

Well-known props and `evt_kind`.

## Value data model

`serde`, `sval` -> `Value` data model

## Extents and timestamps

Points vs ranges
