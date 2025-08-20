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

        /*
        emit::sample!(sampler, extent: ts, mdl: "my_app", metric_agg: "count", metric_name: "metric_a", metric_value: metric_a, props: emit::props! {});
        emit::sample_count!(sampler, extent: ts, metric_a);

        emit::new_sample_count!(metric_a);
                                --------
                                infer:
                                  - metric_name: "metric_a"
                                  - metric_value: metric_a
        - sampler: default to `emit::sampler()`
        - extent: default to `emit::clock().now()`
        - mdl: default to `emit::mdl!()`
        - metric_agg: default to "last"
        - metric_name: infer from trailing prop, or require along with metric_value
        - metric_value: infer from trailing prop, or require along with metric_name

        Note that using a trailing prop is a compatibility hazard; we can't introduce any new control parameters.

        --> emit::count_sample!(metric_value: metric_a);

        If `metric_value` is an identifier, then infer `metric_name` from it. That seems like the cleanest API.
        In other cases, you'll need to specify `metric_name` as well.
        The `metric_value` is _always_ required.

        -----
        */

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
