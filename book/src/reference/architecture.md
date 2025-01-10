# Architecture

This section describes `emit`'s key components and how they fit together.

## Crate organization

`emit` is split into a few subcrates:

```mermaid
classDiagram
    direction RL
    
    emit_core <.. emit_macros
    emit_core <.. emit
    emit_macros <.. emit

    class emit_macros {
        emit_core = "0.11.3"
        proc-macro2 = "1"
        quote = "1"
        syn = "2"
    }

    emit <.. emit_term
    emit <.. emit_file
    emit <.. emit_otlp
    emit <.. emit_custom

    emit <.. app : required

    class emit {
        emit_core = "0.11.3"
        emit_macros = "0.11.3"
    }

    emit_term .. app : optional
    emit_file .. app : optional
    emit_otlp .. app : optional
    emit_custom .. app : optional

    class emit_term {
        emit = "0.11.3"
    }

    class emit_file {
        emit = "0.11.3"
    }

    class emit_otlp {
        emit = "0.11.3"
    }

    class emit_custom["other emitter"] {
        emit = "0.11.3"
    }

    class app["your app"] {
        emit = "0.11.3"
        emit_term = "0.11.3"*
        emit_file = "0.11.3"*
        emit_otlp = "0.11.3"*
    }

    click emit_core href "https://docs.rs/emit_core/0.11.3/emit_core/index.html"
    click emit_macros href "https://docs.rs/emit_macros/0.11.3/emit_macros/index.html"
    click emit href "https://docs.rs/emit/0.11.3/emit/index.html"
    click emit_term href "https://docs.rs/emit_term/0.11.3/emit_term/index.html"
    click emit_file href "https://docs.rs/emit_file/0.11.3/emit_file/index.html"
    click emit_otlp href "https://docs.rs/emit_otlp/0.11.3/emit_otlp/index.html"
```

- [`emit`](https://docs.rs/emit/0.11.3/emit/index.html): The main library that re-exports `emit_core` and `emit_macros`. This is the one your applications depend on.
- [`emit_core`](https://docs.rs/emit_core/0.11.3/emit_core/index.html): Just the fundamental APIs. It includes the `shared()` and `internal()` runtimes. The goal of this library is to remain stable, even if macro syntax evolves over time.
- [`emit_macros`](https://docs.rs/emit_macros/0.11.3/emit_macros/index.html): `emit!`, `#[span]`, and other procedural macros.

The `emit` library doesn't implement anywhere for you to send your diagnostics itself, but there are other libraries that do:

- [`emit_term`](https://docs.rs/emit_term/0.11.3/emit_term/index.html): Writes to the console. See [Emitting to the console](../emitting-events/console.md) for details.
- [`emit_file`](https://docs.rs/emit_file/0.11.3/emit_file/index.html): Writes to rolling files. See [Emitting to rolling files](../emitting-events/rolling-files.md) for details.
- [`emit_otlp`](https://docs.rs/emit_otlp/0.11.3/emit_otlp/index.html): Writes OpenTelemetry's wire protocol. See [Emitting via OTLP](../emitting-events/otlp.md) for details.

You can also write your own emitters by implementing the [`Emitter`](https://docs.rs/emit/0.11.3/emit/trait.Emitter.html) trait. See [Writing an Emitter](../for-developers/writing-an-emitter.md) for details.

## Events

[`Event`](https://docs.rs/emit/0.11.3/emit/struct.Event.html)s are the central data type in `emit` that all others hang off. They look like this:

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

    click Event href "https://docs.rs/emit/0.11.3/emit/struct.Event.html"
    click Timestamp href "https://docs.rs/emit/0.11.3/emit/struct.Timestamp.html"
    click Extent href "https://docs.rs/emit/0.11.3/emit/struct.Extent.html"
    click Str href "https://docs.rs/emit/0.11.3/emit/struct.Str.html"
    click Value href "https://docs.rs/emit/0.11.3/emit/struct.Value.html"
    click Props href "https://docs.rs/emit/0.11.3/emit/trait.Props.html"
    click Template href "https://docs.rs/emit/0.11.3/emit/struct.Template.html"
    click Path href "https://docs.rs/emit/0.11.3/emit/struct.Path.html"
```

Events include:

- A [`Path`](https://docs.rs/emit/0.11.3/emit/struct.Path.html) for the component that generated them.
- A [`Template`](https://docs.rs/emit/0.11.3/emit/struct.Template.html) for their human-readable description. Templates can also make good low-cardinality identifiers for a specific shape of event.
- An [`Extent`](https://docs.rs/emit/0.11.3/emit/struct.Extent.html) for the time the event is relevant. The extent itself may be a single [`Timestamp`](https://docs.rs/emit/0.11.3/emit/struct.Timestamp.html) for a point in time, or a pair of timestamps representing an active time range.
- [`Props`](https://docs.rs/emit/0.11.3/emit/trait.Props.html) for structured key-value pairs attached to the event. These can be lazily interpolated into the template.

See [Event data model](./events.md) for more details.

## Runtimes

In `emit`, a diagnostic pipeline is an instance of a [`Runtime`](https://docs.rs/emit/0.11.3/emit/runtime/struct.Runtime.html). Each runtime is an isolated set of components that help construct and emit diagnostic events in your applications. It looks like this:

```mermaid
classDiagram
    direction RL

    class AmbientSlot {
        get() Runtime
    }

    Runtime <.. AmbientSlot

    class Runtime {
        emitter() Emitter
        filter() Filter
        ctxt() Ctxt
        clock() Clock
        rng() Rng
    }

    Emitter <.. Runtime
    Filter <.. Runtime
    Ctxt <.. Runtime
    Clock <.. Runtime
    Rng <.. Runtime

    class Emitter {
        emit(Event)*
    }

    <<Trait>> Emitter

    class Filter {
        matches(Event) bool*
    }

    <<Trait>> Filter

    class Ctxt {
        open(Props)*
        with_current(FnOnce~Props~)*
    }

    <<Trait>> Ctxt

    class Clock {
        now() Timestamp*
    }

    <<Trait>> Clock

    class Rng {
        fill([u8])*
    }

    <<Trait>> Rng

    click Emitter href "https://docs.rs/emit/0.11.3/emit/trait.Emitter.html"
    click Filter href "https://docs.rs/emit/0.11.3/emit/trait.Filter.html"
    click Ctxt href "https://docs.rs/emit/0.11.3/emit/trait.Ctxt.html"
    click Clock href "https://docs.rs/emit/0.11.3/emit/trait.Clock.html"
    click Rng href "https://docs.rs/emit/0.11.3/emit/trait.Rng.html"
    click Runtime href "https://docs.rs/emit/0.11.3/emit/runtime/struct.Runtime.html"
    click AmbientSlot href "https://docs.rs/emit/0.11.3/emit/runtime/struct.AmbientSlot.html"
```

A [`Runtime`](https://docs.rs/emit/0.11.3/emit/runtime/struct.Runtime.html) includes:

- [`Emitter`](https://docs.rs/emit/0.11.3/emit/trait.Emitter.html): Responsible for sending events to some outside observer.
- [`Filter`](https://docs.rs/emit/0.11.3/emit/trait.Filter.html): Responsible for determining whether an event should be emitted or not.
- [`Ctxt`](https://docs.rs/emit/0.11.3/emit/trait.Ctxt.html): Responsible for storing ambient context that's appended to events as they're constructed.
- [`Clock`](https://docs.rs/emit/0.11.3/emit/trait.Clock.html): Responsible for assigning timestamps to events and running timers.
- [`Rng`](https://docs.rs/emit/0.11.3/emit/trait.Rng.html): Responsible for generating unique identifiers like trace and span ids.

An [`AmbientSlot`](https://docs.rs/emit/0.11.3/emit/runtime/struct.AmbientSlot.html) is a container for a `Runtime` that manages global initialization. `emit` includes two built-in ambient slots:

- [`shared()`](https://docs.rs/emit/0.11.3/emit/runtime/fn.shared_slot.html): The runtime used by default when not otherwise specified.
- [`internal()`](https://docs.rs/emit/0.11.3/emit/runtime/fn.internal_slot.html): The runtime used by other runtimes for self diagnostics.

You can also define your own `AmbientSlot`s or use `Runtime`s directly.

## Event construction and emission

When the [`emit!`](https://docs.rs/emit/0.11.3/emit/macro.emit.html) macro is called, an event is constructed using features of the runtime before being emitted through it. This is how it works:

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

    mdl_path["`<code>mdl!()</code>`"] --> mdl["`<code>Path('a::b::c')</code>`"]

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

    click macro href "https://docs.rs/emit/0.11.3/emit/macro.emit.html"

    click tpl href "https://docs.rs/emit/0.11.3/emit/struct.Template.html"

    click macro_props href "https://docs.rs/emit/0.11.3/emit/trait.Props.html"
    click ctxt_props href "https://docs.rs/emit/0.11.3/emit/trait.Props.html"
    click props href "https://docs.rs/emit/0.11.3/emit/trait.Props.html"

    click mdl href "https://docs.rs/emit/0.11.3/emit/struct.Path.html"

    click ts href "https://docs.rs/emit/0.11.3/emit/struct.Timestamp.html"
    click extent href "https://docs.rs/emit/0.11.3/emit/struct.Extent.html"

    click event href "https://docs.rs/emit/0.11.3/emit/struct.Event.html"

    click emitter href "https://docs.rs/emit/0.11.3/emit/trait.Emitter.html"
    click filter href "https://docs.rs/emit/0.11.3/emit/trait.Filter.html"
    click ctxt href "https://docs.rs/emit/0.11.3/emit/trait.Ctxt.html"
    click clock href "https://docs.rs/emit/0.11.3/emit/trait.Clock.html"
```

When constructing an event, the runtime provides the current timestamp and any ambient context. When emitting an event, the runtime filters out events to discard and emits the ones that remain.

Once an event is constructed, it no longer distinguishes properties attached directly from properties added by the ambient context.

You don't need to use macros to construct events. You can also do it manually to get more control over the data they contain.

## Span construction and emission

When the [`#[span]`](https://docs.rs/emit/0.11.3/emit/attr.span.html) macro is called, the annotated function is instrumented using features of the runtime before a span representing its execution is emitted through it. This is how it works:

```mermaid
flowchart
    start((start)) --> macro["`<code>#[span('a {x}', y)]</code>`"]

    macro --> tpl["`<code>Template('a {x}')</code>`"]
    macro --> macro_props["`<code>Props { x, y }</code>`"]

    ctxt{{"`<code>Ctxt</code>`"}}

    span_ctxt["`<code>SpanCtxt</code>`"]

    rng{{"`<code>Rng</code>`"}} -- new_child --> span_ctxt
    ctxt -- current --> span_ctxt

    clock{{"`<code>Clock</code>`"}} --> timer["`<code>Timer::start</code>`"]

    emitter{{"`<code>Emitter</code>`"}}

    completion["`<code>completion::Default</code>`"]
    tpl --> completion
    emitter --> completion
    ctxt_2{{"`<code>Ctxt</code>`"}} --> completion

    frame["`<code>Frame</code>`"]
    ctxt --> frame
    macro_props -- push --> frame
    span_ctxt -- push --> frame

    filter{"`<code>Filter::matches</code>`"}

    timer --> active_span

    active_span["`<code>SpanGuard</code>`"]

    active_span --> filter
    frame --> filter

    filter -- false --> filter_no(((disabled)))

    filter -- true --> active_span_2
    filter -- true --> frame_2

    frame_2["<code>Frame::call</code>"]

    active_span_2["`<code>SpanGuard::complete</code>`"] -- produces --> span["`<code>Span</code>`"] --> completion
    
    completion --> END(((end)))

    click macro href "https://docs.rs/emit/0.11.3/emit/attr.span.html"

    click tpl href "https://docs.rs/emit/0.11.3/emit/struct.Template.html"

    click macro_props href "https://docs.rs/emit/0.11.3/emit/trait.Props.html"

    click span_ctxt href "https://docs.rs/emit/0.11.3/emit/span/struct.SpanCtxt.html"
    click span href "https://docs.rs/emit/0.11.3/emit/span/struct.Span.html"
    click timer href "https://docs.rs/emit/0.11.3/emit/timer/struct.Timer.html"

    click frame href "https://docs.rs/emit/0.11.3/emit/frame/struct.Frame.html"
    click frame_2 href "https://docs.rs/emit/0.11.3/emit/frame/struct.Frame.html"

    click active_span href "https://docs.rs/emit/0.11.3/emit/span/struct.SpanGuard.html"
    click active_span_2 href "https://docs.rs/emit/0.11.3/emit/span/struct.SpanGuard.html"

    click completion href "https://docs.rs/emit/0.11.3/emit/span/completion/struct.Default.html"

    click emitter href "https://docs.rs/emit/0.11.3/emit/trait.Emitter.html"
    click filter href "https://docs.rs/emit/0.11.3/emit/trait.Filter.html"
    click ctxt href "https://docs.rs/emit/0.11.3/emit/trait.Ctxt.html"
    click ctxt_2 href "https://docs.rs/emit/0.11.3/emit/trait.Ctxt.html"
    click clock href "https://docs.rs/emit/0.11.3/emit/trait.Clock.html"
    click rng href "https://docs.rs/emit/0.11.3/emit/trait.Rng.html"
```

When constructing a span, the runtime generates random trace and span ids from the current ambient context and starts a timer using the clock.

If the runtime filter doesn't match an event representing the start of the span then it's disabled and won't be completed. If it does, an active span guard managing the completion of the span, and a frame containing its ambient trace and span ids are constructed.

At the end of the annotated function, the active span guard is completed, constructing a span event and passing it to its configured completion. From there, the event can be emitted back through the runtime.
