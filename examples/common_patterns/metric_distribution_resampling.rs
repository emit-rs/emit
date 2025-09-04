/*!
This example is an extension of `metric_distribution` that resamples if
the total number of stored buckets gets too big.

Buckets can be resampled by picking a smaller value for `scale` and resampling
their midpoints through the new scale. Buckets from higher scales always perfectly
fit into buckets with smaller ones, so there's no need to interpolate counts across
buckets.
*/

use std::{
    collections::BTreeMap,
    thread,
    time::{Duration, Instant},
};

struct MyDistribution {
    scale: i32,
    total: u64,
    max_buckets: usize,
    buckets: BTreeMap<emit::metric::dist::Point, u64>,
}

impl MyDistribution {
    fn observe(&mut self, value: f64) {
        *self
            .buckets
            .entry(emit::metric::dist::midpoint(value, self.scale))
            .or_default() += 1;
        self.total += 1;

        // If we've overflowed then reduce our scale and resample
        // Each time `scale` is decremented, our number of buckets will be halved
        if self.buckets.len() >= self.max_buckets {
            self.scale -= 1;

            let mut resampled = BTreeMap::new();

            for (value, count) in &self.buckets {
                *resampled
                    .entry(emit::metric::dist::midpoint(value.get(), self.scale))
                    .or_default() += *count;
            }

            self.buckets = resampled;
        }
    }
}

fn main() {
    let rt = emit::setup()
        .emit_to(emit::emitter::from_fn(|evt| println!("{evt:?}")))
        .init();

    let mut metric_a = MyDistribution {
        scale: 10,
        max_buckets: 20,
        total: 0,
        buckets: BTreeMap::new(),
    };

    // Observe some values
    // In this case, every value is unique, so if we don't resample
    // we'll end up with a lot of buckets
    let start = Instant::now();
    for _ in 0..3977 {
        let sample = start.elapsed().as_millis() as f64;

        metric_a.observe(sample);

        thread::sleep(Duration::from_micros(117));
    }

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
