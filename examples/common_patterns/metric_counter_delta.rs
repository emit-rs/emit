/*!
This example demonstrates emitting metric samples as deltas instead of cumulative values.

`emit` defines the `Delta` type as a general wrpaper for tracking deltas. It doesn't rely
on any particular aggregation or strategy. It just keeps track of the last time its value
was sampled and returns an extent covering that period.
*/

use std::{thread, time::Duration};

fn main() {
    let rt = emit::setup().emit_to(emit_term::stdout()).init();

    // Define a metric wrapped in a `Delta` wrapper
    let mut metric_a = emit::metric::Delta::new(emit::clock().now(), 0);

    // Update the current delta value
    *metric_a.current_value_mut() += 4;

    // Wait a bit so the delta we're about to sample covers some amount of time
    thread::sleep(Duration::from_secs(1));

    {
        // Sample the metric, getting the accumulated change since the last time we read it
        let (extent, metric_a) = metric_a.advance(emit::clock().now());

        // Emit a metric sample with the delta of `metric_a`
        emit::count_sample!(extent, name: "metric_a", value: *metric_a);
    }

    rt.blocking_flush(Duration::from_secs(5));
}
