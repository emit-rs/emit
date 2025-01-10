# Filtering events

`emit` supports client-side filtering using a [`Filter`](https://docs.rs/emit/0.11.4/emit/trait.Filter.html).

## Setup

Filters are configured through the [`setup`](https://docs.rs/emit/0.11.4/emit/setup/fn.setup.html) function at the start of your application by calling [`emit_when`](https://docs.rs/emit/0.11.4/emit/setup/struct.Setup.html#method.emit_when):

```rust
# extern crate emit;
# extern crate emit_term;
fn main() {
    let rt = emit::setup()
        // This filter accepts any event with a level over warn
        .emit_when(emit::level::min_filter(emit::Level::Warn))
        .emit_to(emit_term::stdout())
        .init();

    // Your app code goes here

    rt.blocking_flush(std::time::Duration::from_secs(5));
}
```

Filters can be combined with [`and_when`](https://docs.rs/emit/0.11.4/emit/trait.Filter.html#method.and_when) and [`or_when`](https://docs.rs/emit/0.11.4/emit/trait.Filter.html#method.or_when):

```rust
# extern crate emit;
use emit::Filter;

# extern crate emit_term;
fn main() {
    let rt = emit::setup()
        // This filter accepts any event with a level over warn or where the module path is `my_module`
        .emit_when(emit::level::min_filter(emit::Level::Warn)
            .or_when(emit::filter::from_fn(|evt| evt.mdl() == "my_module"))
        )
        .emit_to(emit_term::stdout())
        .init();

    // Your app code goes here

    rt.blocking_flush(std::time::Duration::from_secs(5));
}
```

## Wrapping emitters in filters

You can also wrap an emitter in [`emit_to`](https://docs.rs/emit/0.11.4/emit/setup/struct.Setup.html#method.emit_to) in a filter:

```rust
# extern crate emit;
# extern crate emit_term;
use emit::Emitter;

fn main() {
    let rt = emit::setup()
        // Wrap the emitter in a filter instead of setting it independently
        .emit_to(emit_term::stdout()
            .wrap_emitter(emit::emitter::wrapping::from_filter(
                emit::level::min_filter(emit::Level::Warn))
            )
        )
        .init();

    // Your app code goes here

    rt.blocking_flush(std::time::Duration::from_secs(5));
}
```

Wrapping an emitter in a filter is not the same as providing an emitter and filter independently.

When you use `emit_when`, the filter may be by-passed using the `when` [control parameter](./reference/control-parameters.md) on the [`emit!`](https://docs.rs/emit/0.11.4/emit/macro.emit.html) or [`#[span]`](https://docs.rs/emit/0.11.4/emit/attr.span.html) macros to emit an event even if the filter wouldn't match it. However, the filter specified by `emit_when` doesn't allow you to filter differently if you specify multiple emitters.

When you wrap an emitter in a filter, the filter cannot by by-passed, but each emitter can use its own filter.

Also see [Wrapping emitters](./emitting-events.md#wrapping-emitters) for more details on wrappings.

## Filtering by level

You can use the [`level::min_filter`](https://docs.rs/emit/0.11.4/emit/level/fn.min_filter.html) function to create a filter that matches events based on their level:

```rust
# extern crate emit;
# extern crate emit_term;
fn main() {
    let rt = emit::setup()
        // This filter accepts any event with a level over warn
        .emit_when(emit::level::min_filter(emit::Level::Warn))
        .emit_to(emit_term::stdout())
        .init();

    // Your app code goes here

    rt.blocking_flush(std::time::Duration::from_secs(5));
}
```

See [the crate docs](https://docs.rs/emit/0.11.4/emit/level/struct.MinLevelFilter.html) for more details.

## Filtering by module

You can use the [`level::min_by_path_filter`](https://docs.rs/emit/0.11.4/emit/level/fn.min_by_path_filter.html) function to create a filter that matches events based on their module path and level:

```rust
# extern crate emit;
# extern crate emit_term;
fn main() {
    let rt = emit::setup()
        // This filter accepts any event with a level over warn
        .emit_when(emit::level::min_by_path_filter([
            (emit::path!("noisy_module"), emit::Level::Warn),
            (emit::path!("noisy_module::important_sub_module"), emit::Level::Info),
            (emit::path!("important_module"), emit::Level::Debug),
        ]))
        .emit_to(emit_term::stdout())
        .init();

    // Your app code goes here

    rt.blocking_flush(std::time::Duration::from_secs(5));
}
```

See [the crate docs](https://docs.rs/emit/0.11.4/emit/level/struct.MinLevelPathMap.html) for more details.

## Filtering spans

When you use the [`#[span]`](https://docs.rs/emit/0.11.4/emit/attr.span.html) macro, `emit` will apply the filter to determine whether the span should be created. If the span doesn't match the filter then no trace context will be generated for it. This isn't the same as trace sampling. `emit` doesn't have the concept of a trace that is not recorded. See [Sampling and filtering traces](./producing-events/tracing/sampling.md) for more details.
