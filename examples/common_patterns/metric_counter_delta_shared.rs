/*!
This example demonstrates one strategy for synchronizing delta metrics.

It uses an `AtomicUsize` and a `RwLock`. Sampling requires an exclusive write lock, but incrementing only
needs a read lock.
*/

use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        LazyLock, RwLock,
    },
    thread,
    time::Duration,
};

// Define our shared metric as a static, so it can be reached throughout the program
// We force initialization of the metric after `emit::setup()` in `main`.
static METRIC_A: LazyLock<RwLock<emit::metric::Delta<AtomicUsize>>> = LazyLock::new(|| {
    RwLock::new(emit::metric::Delta::new(
        emit::clock().now(),
        AtomicUsize::new(0),
    ))
});

fn main() {
    let rt = emit::setup().emit_to(emit_term::stdout()).init();

    // Initialize our metric
    LazyLock::force(&METRIC_A);

    // Spawn some independent background workers that operate on the metric
    for _ in 0..3 {
        let _ = thread::spawn(move || loop {
            METRIC_A
                .read()
                .unwrap()
                .current_value()
                .fetch_add(1, Ordering::Relaxed);

            thread::sleep(Duration::from_millis(117));
        });
    }

    // Sample the delta each second
    for _ in 0..5 {
        thread::sleep(Duration::from_secs(1));

        let mut guard = METRIC_A.write().unwrap();
        let (extent, metric_a) = guard.advance(emit::clock().now());
        let metric_a = metric_a.swap(0, Ordering::Relaxed);

        // Drop the guard before emitting, just to shorten the length
        // of our write lock as much as possible
        drop(guard);

        // Emit a metric sample with the delta of `metric_a`
        emit::count_sample!(extent, name: "metric_a", value: metric_a);
    }

    rt.blocking_flush(Duration::from_secs(5));
}
