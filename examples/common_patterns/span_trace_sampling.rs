/*
An example of how to perform trace sampling using `emit_traceparent`.

This example differs from `span_trace_filtering` by letting you propagate the sampling decision
to downstream services.
*/

use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

fn main() {
    // Setup using `emit_traceparent` instead of `emit`
    let rt = emit_traceparent::setup_with_sampler({
        let counter = AtomicUsize::new(0);

        move |_| {
            // Sample 1 in every 10 traces
            counter.fetch_add(1, Ordering::Relaxed) % 10 == 0
        }
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

    // These events will still be emitted, they just won't
    // be part of unsampled traces
    emit::debug!("computed {r} = {i} + 1");

    r
}
