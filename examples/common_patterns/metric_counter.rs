/*!
This example demonstrates emitting metric samples through `emit`.

`emit` doesn't prescribe any API for collecting metrics. You can use atomic integers, local variables, or anything
else that best suits your application's needs. Once you have a metric sample, you can emit it as an event in
a standard schema using one of the following macros:

- `emit::count_sample!` for count metrics.
- `emit::sum_sample!` for sum metrics.
- `emit::min_sample!` for min metrics.
- `emit::max_sample!` for max metrics.
- `emit::last_sample!` for last metrics.

The metrics data model is described in more detail in the book.
*/

use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

fn main() {
    let rt = emit::setup().emit_to(emit_term::stdout()).init();

    // Define a metric
    // `emit` doesn't require any particular strategy for defining or collecting metric values
    let metric_a = AtomicUsize::new(0);

    // Emit a metric sample with the current value of `metric_a`
    emit::count_sample!(name: "metric_a", value: metric_a.load(Ordering::Relaxed));

    // We'll increment the metric here to observe a new value for it
    metric_a.fetch_add(3, Ordering::Relaxed);

    // Emit another sample for `metric_a`
    emit::count_sample!(name: "metric_a", value: metric_a.load(Ordering::Relaxed));

    rt.blocking_flush(Duration::from_secs(5));
}
