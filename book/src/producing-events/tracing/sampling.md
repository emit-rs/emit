# Sampling and filtering traces

Sampling is a tool to help limit the volume of ingested trace data. It's typically applied when a trace begins by making an upfront decision about whether to produce and/or emit the trace. This is usually called "head sampling" and is limited to probablistic methods. Tail sampling, or deciding whether to ingest a trace after it's completed is much harder to implement, because there's no absolute way to know when a particular trace is finished, or how long it will take.

Sampling and [propagation](./propagating-across-services.md) are tied together. If a service decides not to sample a given trace then it _must_ propagate that decision to downstream services. Otherwise you'll end up with a broken trace.

## Using `emit_traceparent` for sampling

[`emit_traceparent`](https://docs.rs/emit_traceparent/1.10.0/emit_traceparent/) is a library that implements trace sampling and propagation. Using `setup_with_sampler`, you can configure `emit` with a function that's run at the start of each trace to determine whether to emit it or not. Any other diagnostics produced within an unsampled trace will still be emitted, but won't be associated with that trace.

This example is a simple sampler that includes one in every 10 traces:

```rust
# extern crate emit;
# extern crate emit_term;
# extern crate emit_traceparent;
# use std::sync::atomic::{AtomicUsize, Ordering};
fn main() {
    let rt = emit_traceparent::setup_with_sampler({
        let counter = AtomicUsize::new(0);

        move |_| {
            // Sample 1 in every 10 traces
            counter.fetch_add(1, Ordering::Relaxed) % 10 == 0
        }
    })
    .emit_to(emit_term::stdout())
    .init();

    // Your code goes here

    rt.blocking_flush(std::time::Duration::from_secs(5));
}
```

## Using the OpenTelemetry SDK for sampling

If you're using the OpenTelemetry SDK, [`emit_opentelemetry`](https://docs.rs/emit_opentelemetry) will respect its sampling.

## Manual sampling

You can use `emit`'s [filters](../../filtering-events.md) to implement sampling. This example excludes all diagnostics produced outside of sampled traces, and only includes one in every 10 traces:

```rust
# extern crate emit;
# extern crate emit_term;
# use std::sync::atomic::{AtomicUsize, Ordering};
use emit::{Filter, Props};

fn main() {
    let rt = emit::setup()
        .emit_when({
            // Only include events in sampled traces
            let is_in_sampled_trace = emit::filter::from_fn(|evt| {
                evt.props().get("trace_id").is_some() && evt.props().get("span_id").is_some()
            });

            // Only keep 1 in every n traces
            let one_in_n_traces = emit::filter::from_fn({
                let counter = AtomicUsize::new(0);

                move |evt| {
                    // If the event is not a span then include it
                    let Some(emit::Kind::Span) = evt.props().pull::<emit::Kind, _>("evt_kind")
                    else {
                        return true;
                    };

                    // If the span is not the root of a new trace then include it
                    if evt.props().get("span_parent").is_some() {
                        return true;
                    };

                    // Keep 1 in every 10 traces
                    counter.fetch_add(1, Ordering::Relaxed) % 10 == 0
                }
            });

            is_in_sampled_trace.and_when(one_in_n_traces)
        })
        .emit_to(emit_term::stdout())
        .init();

    // Your code goes here

    rt.blocking_flush(std::time::Duration::from_secs(5));
}
```
