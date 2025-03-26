# Manual span creation

## Creating `SpanGuard`s

The `#[span]` attribute [includes a `guard` control parameter](./manual-span-completion.md) that gives you access to a [`SpanGuard`](https://docs.rs/emit/1.4.1/emit/span/struct.SpanGuard.html) to manually complete it. The `#[span]` attribute takes care of constructing the `SpanGuard` for you and ensuring any ambient span properties are active in the body of your annotated function.

You can also create `SpanGuard`s manually if you can't or don't want to use the `#[span]` attribute:

```rust
# extern crate emit;
let (mut span, frame) = emit::new_span!("manual span");

frame.call(move || {
    span.start();

    // Your code goes here
})
```

The `new_span!` macro returns a tuple of [`SpanGuard`](https://docs.rs/emit/1.4.1/emit/span/struct.SpanGuard.html) for completing the span, and [`Frame`](https://docs.rs/emit/1.4.1/emit/frame/struct.Frame.html) for activating the span's ambient trace and span ids for correlation.

The syntax accepted by the `new_span!` macro is the same as the [`#[span]`](https://docs.rs/emit/1.4.1/emit/attr.span.html) attribute.

**Make sure you call [`SpanGuard::start`](https://docs.rs/emit/1.4.1/emit/span/struct.SpanGuard.html#method.start) in the closure in [`Frame::call`](https://docs.rs/emit/1.4.1/emit/frame/struct.Frame.html#method.call) or async block in [`Frame::in_future`](https://docs.rs/emit/1.4.1/emit/frame/struct.Frame.html#method.in_future)**. If you don't call `SpanGuard::start`, the span won't be emitted. If you don't call it within the frame, the span may be emitted early and without its ambient context.

Using `SpanGuard`s is the recommended way to trace code with `emit`. It applies filtering for you, so the span is only created if it matches the configured filter. It also ensures a span is emitted even if the traced code panics or otherwise returns without explicitly completing.

## Creating spans without a `SpanGuard`

In cases where sampling or filtering aren't used, or when execution of a single span is split across multiple functions, you can create spans without using a `SpanGuard`:

```rust
# extern crate emit;
// Create a new span context that is a child of the current one
// This context can be freely copied or stored elsewhere
let ctxt = emit::SpanCtxt::current(emit::ctxt())
    .new_child(emit::rng());

// Push the span onto the current context when you're about to execute
// some code within it
ctxt.push(emit::ctxt())
    .call(move || {
        let timer = emit::Timer::start(emit::clock());

        // Your code goes here
        let sleep_ms = 1200;
        std::thread::sleep(std::time::Duration::from_millis(sleep_ms));

        // Make sure you complete the span in the frame.
        // This is especially important for futures, otherwise the span may
        // complete before the future does
        emit::emit!(
            extent: timer,
            "wait a bit",
            evt_kind: "span",
            sleep_ms,
        );
    });
```

This approach can be used, for example, in web frameworks that split request handling across multiple independent function calls.

## Creating spans without any ambient context

Trace and span ids don't need to be managed by `emit` if you have another scheme in mind. In these cases, they can be attached as regular properties to the span event:

```rust
# extern crate emit;
let timer = emit::Timer::start(emit::clock());

// Your code goes here
let sleep_ms = 1200;
std::thread::sleep(std::time::Duration::from_millis(sleep_ms));

emit::emit! {
    extent: timer,
    "wait a bit",
    evt_kind: "span",
    trace_id: "4bf92f3577b34da6a3ce929d0e0e4736",
    span_id: "00f067aa0ba902b7",
    sleep_ms,
}
```

Note that when emitting spans as regular events that you still thread the trace context around somehow, otherwise other events emitted within its execution won't be correlated with it.
