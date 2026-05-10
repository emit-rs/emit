/*!
This example demonstrates attaching a distribution to a metric sample.

Distributions give you an idea of what the individual values that made up your sample look like
without actually storing all of them.

The guide has more details on distributions.
*/

use std::{
    thread,
    time::{Duration, Instant},
};

fn main() {
    let rt = emit::setup().emit_to(emit_term::stdout()).init();

    let mut metric_a = emit::metric::exp::Distribution::new(160);

    // Observe some values
    // In this case, we're just making up some repeating values to show
    // how similar values are bucketed together
    let start = Instant::now();
    for _ in 0..1177 {
        let sample = (start.elapsed().as_millis() as f64).sin() * 100.0;

        metric_a.observe(sample);

        thread::sleep(Duration::from_micros(317));
    }

    // Sample our metric as a count using the total value
    // We also include the metric itself to include its buckets and scale
    emit::count_sample!(
        name: "metric_a",
        value: metric_a.count(),
        props: metric_a,
    );

    rt.blocking_flush(Duration::from_secs(5));
}
