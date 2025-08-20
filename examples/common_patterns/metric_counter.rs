use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

fn main() {
    let rt = emit::setup().emit_to(emit_term::stdout()).init();

    let metric_a = AtomicUsize::new(0);

    emit::count_sample!(name: "metric_a", value: metric_a.load(Ordering::Relaxed));

    metric_a.fetch_add(3, Ordering::Relaxed);

    emit::count_sample!(name: "metric_a", value: metric_a.load(Ordering::Relaxed));

    rt.blocking_flush(Duration::from_secs(5));
}
