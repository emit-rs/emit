# Span creation

`emit`'s tracing infrastructure is largely concerned with the lifecycle of spans. At the start of a traced operation, a span is created as a [`SpanGuard`](https://docs.rs/emit/2.22.4/emit/span/struct.SpanGuard.html), carrying the start time and properties for the eventual [`Span`](https://docs.rs/emit/2.22.4/emit/span/struct.Span.html) event that will be emitted at the end.

Spans can be created in a number of ways, depending on the needs of the application.

## Using macros

The standard method to create a span is using the [`#[span]`](https://docs.rs/emit/2.22.4/emit/attr.span.html) macro on a function definition:

```rust
# extern crate emit;
#[emit::span("wait a bit", sleep_ms)]
fn wait_a_bit(sleep_ms: u64) {
    std::thread::sleep(std::time::Duration::from_millis(sleep_ms))
}

wait_a_bit(1200);
```

The same attribute works for `async` functions too.

The `#[span]` macro creates an implicit [`SpanGuard`](https://docs.rs/emit/2.22.4/emit/span/struct.SpanGuard.html) that will be completed when the annotated function returns. You can also use and complete this guard manually. See [Manual span completion](./manual-span-completion.md) for details.

## Manually

Some applications and frameworks have operation lifecycles that outlive a single function call. In these cases, you can create spans manually using the [`span_guard!`](https://docs.rs/emit/2.22.4/emit/macro.span_guard.html) attribute:

```rust
# extern crate emit;
let (mut span, frame) = emit::span_guard!("manual span");

frame.call(move || {
    span.start();

    // Your code goes here
})
```

The `span` type in the above example is a [`SpanGuard`](https://docs.rs/emit/2.22.4/emit/span/struct.SpanGuard.html) that will complete automatically when it goes out of scope. You can also use and complete this guard manually. See [Manual span completion](./manual-span-completion.md) for details.

There are other ways to manually create spans too. See [Manual span creation](./manual-span-creation.md) for details.
