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
    collections::BTreeMap,
    thread,
    time::{Duration, Instant},
};

#[derive(Default)]
struct MyDistribution {
    scale: i32,
    total: u64,
    buckets: BTreeMap<emit::metric::dist::Point, u64>,
}

impl MyDistribution {
    fn observe(&mut self, value: f64) {
        *self
            .buckets
            .entry(emit::metric::dist::midpoint(value, self.scale))
            .or_default() += 1;
        self.total += 1;
    }
}

fn main() {
    let rt = emit::setup().emit_to(emit_term::stdout()).init();

    // Define a metric
    // `emit` doesn't require any particular strategy for defining or collecting metric values
    let mut metric_a = MyDistribution {
        scale: 2,
        ..Default::default()
    };

    // Observe some values
    let start = Instant::now();
    for _ in 0..1177 {
        let sample = (start.elapsed().as_millis() as f64).sin() * 100.0;

        metric_a.observe(sample);

        thread::sleep(Duration::from_micros(317));
    }

    // Sample our `MyDistribution` metric as a count using the `total` value we've been tracking
    //
    // We also include `dist_buckets` and `dist_scale` on our metric sample to attach a distribution
    emit::count_sample!(
        name: "metric_a",
        value: metric_a.total,
        props: emit::props! {
            #[emit::as_sval]
            dist_buckets: metric_a.buckets,
            dist_scale: metric_a.scale,
        },
    );

    rt.blocking_flush(Duration::from_secs(5));
}
