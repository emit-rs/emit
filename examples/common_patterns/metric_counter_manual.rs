/*!
This example is a variant of `metric_counter` that constructs metric samples manually.
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
    emit::sample(emit::Metric::new(
        emit::mdl!(),
        emit::Empty,
        "metric_a",
        "count",
        metric_a.load(Ordering::Relaxed),
        emit::Empty,
    ));

    // Emit a metric sample with the current value of `metric_a`

    // We'll increment the metric here to observe a new value for it
    metric_a.fetch_add(3, Ordering::Relaxed);

    // Emit another sample for `metric_a`
    emit::sample(emit::Metric::new(
        emit::mdl!(),
        emit::Empty,
        "metric_a",
        "count",
        metric_a.load(Ordering::Relaxed),
        emit::Empty,
    ));

    rt.blocking_flush(Duration::from_secs(5));
}
