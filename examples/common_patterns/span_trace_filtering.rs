/*
An example of how to perform trace sampling using `emit`'s filtering.

Prefer the approach from `span_trace_sampling` if you need to propagate sampling
decisions to any downstream services you call.
*/

use emit::{Filter, Props};

use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

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

    for i in 0..30 {
        run_work(i);
    }

    rt.blocking_flush(Duration::from_secs(5));
}

#[emit::span("run work")]
fn run_work(i: i32) -> i32 {
    let r = i + 1;

    emit::debug!("computed {r} = {i} + 1");

    r
}
