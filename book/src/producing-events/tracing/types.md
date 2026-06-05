# Key tracing types

The key types involved in tracing live in the [`tracing`](https://docs.rs/emit/1.20.0/emit/span/index.html) module. They include:

- [`Span`](https://docs.rs/emit/1.20.0/emit/span/struct.Span.html): A kind of event representing a span in a distributed trace.
- [`TraceId`](https://docs.rs/emit/1.20.0/emit/span/struct.TraceId.html): A 128bit trace identifier.
- [`SpanId`](https://docs.rs/emit/1.20.0/emit/span/struct.SpanId.html): A 64bit span identifier.
- [`SpanCtxt`](https://docs.rs/emit/1.20.0/emit/span/struct.SpanCtxt.html): A combination of trace id, parent span id, and span id carried by each span. See [Manual span creation](./manual-span-creation.md) for more details.
- [`SpanGuard`](https://docs.rs/emit/1.20.0/emit/span/struct.SpanGuard.html): A handle to a currently executing span. The guard takes care of completing and emitting a span event when its instrumented function returns. See [Manual span completion](./manual-span-completion.md) for more details.
- [`Completion`](https://docs.rs/emit/1.20.0/emit/span/completion/trait.Completion.html): Called by `SpanGuard`s with a `Span` on completion. A `Completion` will typically forward on to an [`Emitter`](https://docs.rs/emit/1.20.0/emit/trait.Emitter.html), perhaps enriching the `Span` along the way.
