# Architecture

```mermaid
classDiagram
    class AmbientSlot {
        +get() Runtime
    }

    Runtime ..> AmbientSlot

    class Runtime {
        +emitter() Emitter
        +filter() Filter
        +ctxt() Ctxt
        +clock() Clock
        +rng() Rng
    }

    Emitter ..> Runtime
    Filter ..> Runtime
    Ctxt ..> Runtime
    Clock ..> Runtime
    Rng ..> Runtime

    class Emitter {
        emit(Event)*
    }

    class Filter {
        matches() bool*
    }

    class Ctxt {
        open_root(Props)*
        with_current(FnOnce~Props~)*
    }

    class Clock {
        now() Timestamp*
    }

    class Rng {
        fill([u8])*
    }
```

`emit_core` -> `emit_macros` -> `emit` -> `emit_term`, `emit_file`, `emit_otlp`
