use emit::metric::{Sampler, Source};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

// #[derive(emit::Source)]
#[derive(Default)]
struct MyMetrics {
    // #[emit(sample: |x| x.load(Ordering::Relaxed))]
    metric_a: AtomicUsize,
    // #[emit(sample: |x| x.load(Ordering::Relaxed))]
    metric_b: AtomicUsize,
}

impl emit::metric::Source for MyMetrics {
    fn sample_metrics<S: Sampler>(&self, sampler: S) {
        let ts = emit::clock().now();

        let metric_a = self.metric_a.load(Ordering::Relaxed);
        let metric_b = self.metric_b.load(Ordering::Relaxed);

        sampler.metric(emit::Metric::new(
            emit::mdl!(),
            "metric_a",
            "count",
            ts,
            metric_a,
            emit::props! {},
        ));
        sampler.metric(emit::Metric::new(
            emit::mdl!(),
            "metric_b",
            "count",
            ts,
            metric_b,
            emit::props! {},
        ));
    }
}

fn main() {
    let rt = emit::setup().emit_to(emit_term::stdout()).init();

    let metrics = MyMetrics::default();

    emit::sample_metrics(&metrics);

    metrics.metric_a.fetch_add(3, Ordering::Relaxed);
    metrics.metric_b.fetch_add(7, Ordering::Relaxed);

    emit::sample_metrics(&metrics);

    rt.blocking_flush(Duration::from_secs(5));
}
