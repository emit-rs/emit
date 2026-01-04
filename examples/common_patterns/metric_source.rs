/*!
This example demonstrates how to implement your own metric sources.

You don't need to implement a `Source` to sample metrics through `emit`, but can be a useful way
of sampling a set of metrics together.
*/

use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

// Define a type to hold your metrics values
// `emit` doesn't prescribe any particular strategy for metrics,
// but `Source` requires they can be sampled through a shared reference
#[derive(Default)]
struct MyMetrics {
    metric_a: AtomicUsize,
    metric_b: AtomicUsize,
}

impl emit::metric::Source for MyMetrics {
    fn sample_metrics<S: emit::metric::Sampler>(&self, sampler: S) {
        // Sample your metrics
        // We're collecting samples for all metrics before emitting any of them
        // because the sampler may take time to process each one
        let metric_a = self.metric_a.load(Ordering::Relaxed);
        let metric_b = self.metric_b.load(Ordering::Relaxed);

        // Pass each sample to the sampler
        // The `emit::count_metric!` macro produces a metric sample value
        // with `count` as its aggregation. Other variants of this macro
        // exist for other well-known aggregations
        sampler.metric(emit::count_metric!(extent: sampler.sampled_at(), value: metric_a));
        sampler.metric(emit::count_metric!(extent: sampler.sampled_at(), value: metric_b));
    }
}

fn main() {
    let rt = emit::setup().emit_to(emit_term::stdout()).init();

    let metrics = MyMetrics::default();

    // Sample our metric source
    emit::sample(&metrics);

    // Increment our metrics so we can observe new values for them
    metrics.metric_a.fetch_add(3, Ordering::Relaxed);
    metrics.metric_b.fetch_add(7, Ordering::Relaxed);

    // Sample our metric source again
    emit::sample(&metrics);

    rt.blocking_flush(Duration::from_secs(5));
}
