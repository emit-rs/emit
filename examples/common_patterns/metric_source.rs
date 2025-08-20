use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

#[derive(Default)]
struct MyMetrics {
    metric_a: AtomicUsize,
    metric_b: AtomicUsize,
}

impl emit::metric::Source for MyMetrics {
    fn sample_metrics<S: emit::metric::Sampler>(&self, sampler: S) {
        let metric_a = self.metric_a.load(Ordering::Relaxed);
        let metric_b = self.metric_b.load(Ordering::Relaxed);

        sampler.metric(emit::new_count_sample!(value: metric_a));
        sampler.metric(emit::new_count_sample!(value: metric_b));
    }
}

fn main() {
    let rt = emit::setup().emit_to(emit_term::stdout()).init();

    let metrics = MyMetrics::default();

    emit::sample(&metrics);

    metrics.metric_a.fetch_add(3, Ordering::Relaxed);
    metrics.metric_b.fetch_add(7, Ordering::Relaxed);

    emit::sample(&metrics);

    rt.blocking_flush(Duration::from_secs(5));
}
