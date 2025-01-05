# Manual span creation

## Creating `ActiveSpan`s

The `#[span]` attribute [includes a `guard` control parameter](./manual-span-completion.md) that gives you access to an [`ActiveSpan`](https://docs.rs/emit/0.11.0-alpha.21/emit/span/struct.ActiveSpan.html) to manually complete it. The `#[span]` attribute takes care of constructing the `ActiveSpan` for you and ensuring any ambient span properties are active in the body of your annotated function.

You can also create `ActiveSpan`s manually if you can't or don't want to use the `#[span]` attribute:

```rust
# extern crate emit;
let (span, frame) = emit::span::ActiveSpan::start(
    // A filter that determines whether the span is active or not
    emit::filter(),
    // The context to generate span context from, and store ambient context in
    emit::ctxt(),
    // The source of the span's internal timer
    emit::clock(),
    // The source of randomness to generate span context from
    emit::rng(),
    // 1. What to do with the span when the guard completes
    //    Typically you'll want to emit it
    emit::span::completion::from_fn(|evt| emit::emit!(evt)),
    // Any properties to put in the ambient context
    // This doesn't need to include any span or trace ids unless you want to
    // override whatever `ActiveSpan` generates for you
    emit::Empty,
    // The module that generated the span
    emit::mdl!(),
    // 2. The name of the span
    "manual span",
    // Any properties to include on the span on completion
    // These properties won't be added to the ambient context
    emit::Empty,
);

frame.call(move || {
    // Your code goes here

    span.complete();
})
```

**Make sure you pass ownership of the returned [`ActiveSpan`](https://docs.rs/emit/0.11.0-alpha.21/emit/span/struct.ActiveSpan.html) into the closure in [`Frame::call`](https://docs.rs/emit/0.11.0-alpha.21/emit/frame/struct.Frame.html#method.call) or async block in [`Frame::in_future`](https://docs.rs/emit/0.11.0-alpha.21/emit/frame/struct.Frame.html#method.in_future)**. If you don't, the span will complete early, without its ambient context.

The main properties you'll control when creating `ActiveSpan`s are annotated in the example above and are:

1. The [`Completion`](https://docs.rs/emit/0.11.0-alpha.21/emit/span/completion/trait.Completion.html) to use when the span is completed. Usually this will emit the span, but may also set a template or additional properties.
2. The name of the span.

## Creating spans without an `ActiveSpan`

In cases where sampling or filtering aren't used, or when execution of a single span is split across multiple functions, you can create spans without using an `ActiveSpan`:

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
