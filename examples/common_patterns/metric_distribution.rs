/*!
This example demonstrates attaching a distribution to a metric sample.

Distributions give you an idea of what the individual values that made up your sample look like
without actually storing all of them. Values that are close together are bucketed together by
a single `emit::metric::dist::midpoint`. The `scale` parameter decides how big these buckets are.
Bigger buckets take up less space, because there are fewer of them, but are less accurate than
smaller buckets. There's no single right `scale` to use, it depends on the shape of your input data.

The guide has more details on distributions.
*/

use std::{
    collections::BTreeMap,
    thread,
    time::{Duration, Instant},
};

// Define our distribution container
//
// `emit` doesn't put any constraints on how you track and store your buckets.
// In this example, we use a regular sorted map of bucket midpoints, which is
// sufficient for most cases.
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

    let mut metric_a = MyDistribution {
        // Pick a scale of `2`, which is the largest value that gives us
        // an error within 10%. That is, any observed value will be in a bucket
        // that is within 10% of its actual value
        scale: 2,
        ..Default::default()
    };

    // Observe some values
    // In this case, we're just making up some repeating values to show
    // how similar values are bucketed together
    let start = Instant::now();
    for _ in 0..1177 {
        let sample = (start.elapsed().as_millis() as f64).sin() * 100.0;

        metric_a.observe(sample);

        thread::sleep(Duration::from_micros(317));
    }

    // Sample our `MyDistribution` metric as a count using the `total` value we've been tracking.
    //
    // When written out by `emit_term`, we'll see an extra line that summarizes our distribution
    // into quartiles.
    //
    // If emitted to `emit_otlp`, we'll get an exponential histogram metric instead of a sum.
    emit::count_sample!(
        name: "metric_a",
        value: metric_a.total,
        props: emit::props! {
            // `dist_exp_buckets` are our `(midpoint, count)` pairs
            // They can be attached either as an array of tuples, or as a map
            // where keys are midpoints, and values are counts.
            //
            // The shape of `dist_exp_buckets` is complex, so we need to use `#[emit::as_sval]`
            // or `#[emit::as_serde]` to capture them.
            #[emit::as_sval]
            dist_exp_buckets: metric_a.buckets,
            // `dist_exp_scale` is our scale parameter, which tells us how big our buckets are.
            //
            // Buckets can be resampled into smaller scales, but can't be correctly split
            // into larger ones.
            dist_exp_scale: metric_a.scale,
        },
    );

    rt.blocking_flush(Duration::from_secs(5));
}
