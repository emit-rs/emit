/*!
This example is a variant of `metric_distribution_resampling` that reports histogram deltas instead
of cumulative values.

This example also shared similarities with `metric_counter_delta_shared`. Synchronization here is handled
by a `Mutex`, and the delta value is borrowed rather than consumed each time it's sampled.
*/

use std::{
    sync::{LazyLock, Mutex},
    thread,
    time::{Duration, Instant},
};

// Define our shared metric as a static, so it can be reached throughout the program
// We force initialization of the metric after `emit::setup()` in `main`.
static METRIC_A: LazyLock<Mutex<emit::metric::Delta<emit::metric::exp::Distribution>>> =
    LazyLock::new(|| {
        Mutex::new(emit::metric::Delta::new(
            emit::clock().now(),
            emit::metric::exp::Distribution::new(20),
        ))
    });

fn main() {
    let rt = emit::setup()
        .emit_to(emit::emitter::from_fn(|evt| println!("{evt:?}")))
        .init();

    // Initialize our metric
    LazyLock::force(&METRIC_A);

    // Spawn some independent background workers that operate on the metric
    for _ in 0..3 {
        let _ = thread::spawn(move || loop {
            let start = Instant::now();
            for _ in 0..3977 {
                let sample = start.elapsed().as_millis() as f64;

                let mut guard = METRIC_A.lock().unwrap();
                guard.current_value_mut().observe(sample);
                drop(guard);

                thread::sleep(Duration::from_micros(117));
            }
        });
    }

    // Sample the delta each second
    for _ in 0..5 {
        thread::sleep(Duration::from_secs(1));

        // In this example we borrow rather than consume
        // It means we'll hold the lock while the data is emitted,
        // which may be undesirable for some users
        let mut guard = METRIC_A.lock().unwrap();
        let (extent, metric_a) = guard.advance(emit::clock().now());

        emit::count_sample!(
            extent,
            name: "metric_a",
            value: metric_a.count(),
            props: &*metric_a,
        );

        metric_a.reset();
    }

    rt.blocking_flush(Duration::from_secs(5));
}
