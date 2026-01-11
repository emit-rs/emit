# Manual span completion

The `guard` [control parameter](../../reference/control-parameters.md) can be applied to span macros to bind an identifier in the body of the annotated function for the span that's created for it. This span can be completed manually, changing properties of the span along the way:

```rust
# extern crate emit;
#[emit::span(guard: span, "wait a bit", sleep_ms)]
fn wait_a_bit(sleep_ms: u64) {
    std::thread::sleep(std::time::Duration::from_millis(sleep_ms));

    if sleep_ms > 500 {
        // The ident `span` here is what we used as the value for `guard`
        span.complete_with(emit::span::completion::from_fn(|span| {
            emit::warn!(
                when: emit::filter::always(),
                evt: span,
                "wait a bit took too long",
            );
        }));
    }
}

wait_a_bit(100);
wait_a_bit(1200);
```

```text
Event {
    mdl: "my_app",
    tpl: "wait a bit",
    extent: Some(
        "2024-04-28T21:12:20.497595000Z".."2024-04-28T21:12:20.603108000Z",
    ),
    props: {
        "evt_kind": span,
        "span_name": "wait a bit",
        "trace_id": 5b9ab977a530dfa782eedd6db08fdb66,
        "sleep_ms": 100,
        "span_id": 6f21f5ddc707f730,
    },
}
Event {
    mdl: "my_app",
    tpl: "wait a bit took too long",
    extent: Some(
        "2024-04-28T21:12:20.603916000Z".."2024-04-28T21:12:21.808502000Z",
    ),
    props: {
        "evt_kind": span,
        "span_name": "wait a bit",
        "lvl": warn,
        "trace_id": 9abad69ac8bf6d6ef6ccde8453226aa3,
        "sleep_ms": 1200,
        "span_id": c63632332de89ac3,
    },
}
```

Take care when completing spans manually that they always match the configured filter. This can be done using the `when` control parameter like in the above example. If a span is created it _must_ be emitted, otherwise the resulting trace will be incomplete.

## Completing `SpanGuard`s with `Completion`s

The type of the identifier bound by `guard` is a [`SpanGuard`](https://docs.rs/emit/1.16.0/emit/span/struct.SpanGuard.html). When the guard goes out of scope or is manually completed, it constructs a [`Span`](https://docs.rs/emit/1.16.0/emit/span/struct.Span.html) and passes it to a [`Completion`](https://docs.rs/emit/1.16.0/emit/span/completion/trait.Completion.html). A completion that emits the span will be used by default, but a different completion can also be passed to [`complete_with`](https://docs.rs/emit/1.16.0/emit/span/struct.SpanGuard.html#method.complete_with).

Completions can be created using the [`completion::from_fn`](https://docs.rs/emit/1.16.0/emit/span/completion/fn.from_fn.html) function, or by implementing the [`Completion`](https://docs.rs/emit/1.16.0/emit/span/completion/trait.Completion.html) trait.
