# Reporting metric sources

The [`Source`](https://docs.rs/emit/1.18.0/emit/metric/source/trait.Source.html) trait represents some underlying data source that can be sampled through a [`Sampler`](https://docs.rs/emit/1.18.0/emit/metric/source/trait.Sampler.html) to provide [`Metric`](https://docs.rs/emit/1.18.0/emit/metric/struct.Metric.html)s. You can sample sources directly, or combine them into a [`Reporter`](https://docs.rs/emit/1.18.0/emit/metric/struct.Reporter.html) to sample all the sources of metrics in your application together.

This example defines two metric sources, each producing a fixed value when sampled:

```rust
# extern crate emit;
use emit::metric::{Source as _, Sampler as _};

// Create some metric sources
let source_1 = emit::metric::source::from_fn(|sampler| {
    sampler.metric(emit::count_metric!(
        extent: sampler.sampled_at(),
        mdl: emit::path!("source_1"),
        name: "bytes_written",
        value: 1,
    ));
});

let source_2 = emit::metric::source::from_fn(|sampler| {
    sampler.metric(emit::count_metric!(
        extent: sampler.sampled_at(),
        mdl: emit::path!("source_2"),
        name: "bytes_written",
        value: 2,
    ));
});

// Collect them into a reporter
let mut reporter = emit::metric::Reporter::new();

reporter.add_source(source_1);
reporter.add_source(source_2);

// You'll probably want to run this task in your async runtime
// so it observes cancellation etc, but works for this illustration.
std::thread::spawn(move || {
    loop {
        // You could also use `sample_metrics` here instead of `emit_metrics`
        // to do something else with the `Metric` values
        reporter.emit_metrics(emit::runtime::shared());

        std::thread::sleep(std::time::Duration::from_secs(30));
    }
});
```

The [`count_metric!`](https://docs.rs/emit/1.18.0/emit/macro.count_metric.html) macro is a convenient way to construct a `Metric` for a counter. See [Metric creation](./creation.md) for more details.

The [`Sampler`](https://docs.rs/emit/1.18.0/emit/metric/source/trait.Sampler.html) passed to a [`Source`](https://docs.rs/emit/1.18.0/emit/metric/source/trait.Source.html) carries a `sampled_at` [`Timestamp`](https://docs.rs/emit/1.18.0/emit/struct.Timestamp.html) for the point in time when the sample is being collected. Sources are encouraged to use this timestamp instead of computing one themselves.

## Multiple metrics per source

There's no requirement that a single [`Source`](https://docs.rs/emit/1.18.0/emit/metric/source/trait.Source.html) will produce exactly one [`Metric`](https://docs.rs/emit/1.18.0/emit/metric/struct.Metric.html) when sampled. A [`Source`](https://docs.rs/emit/1.18.0/emit/metric/source/trait.Source.html) can produce multiple metrics, which can be used to reduce synchronization costs when locks are involved:

```rust
# extern crate emit;
# use std::sync::Mutex;
use emit::metric::{Source as _, Sampler as _};

struct MyMetrics {
    metric_a: usize,
    metric_b: usize,
}

let metrics = Mutex::new(MyMetrics {
    metric_a: 0,
    metric_b: 0,
});

// Create some metric sources
let source = emit::metric::source::from_fn(|sampler| {
    let metrics = metrics.lock().unwrap();

    sampler.metric(emit::count_metric!(
        extent: sampler.sampled_at(),
        name: "metric_a",
        value: metrics.metric_a,
    ));

    sampler.metric(emit::count_metric!(
        extent: sampler.sampled_at(),
        name: "metric_b",
        value: metrics.metric_b,
    ));
});
```

## Delta sources

You can use the [`Delta`](https://docs.rs/emit/1.18.0/emit/metric/struct.Delta.html) type to implement sources that track deltas instead of cumulative values. The `Delta` type tracks the range the value covers, automatically updating it when sampled.

```rust
# extern crate emit;
use std::sync::Mutex;

// This example synchronizes with a `Mutex`. Other strategies are also possible,
// like `RwLock` with `AtomicUsize`, depending on the underlying value type.
pub struct BytesWritten(Mutex<emit::metric::Delta<usize>>);

impl BytesWritten {
    // Accumulate into the metric
    pub fn extend(&self, value: usize) {
        *self.0.lock().unwrap().current_value_mut() += value;
    }
}

impl emit::metric::Source for BytesWritten {
    fn sample_metrics<S: emit::metric::Sampler>(&self, sampler: S) {
        let mut guard = self.0.lock().unwrap();

        // Get the value for the current time period and an extent covering it
        let (extent, value) = guard.advance(sampler.sampled_at().or_else(|| emit::clock().now()));
        let bytes_written = *value;
        
        // Reset the delta for the new time period
        *value = 0;

        drop(guard);

        // Report the delta
        sampler.metric(emit::count_metric!(
            extent,
            value: bytes_written,
        ));
    }
}
```

See [Delta metrics](./delta-metrics.md) for more details on sampling deltas.

## Normalization of timestamps

The [`Reporter`](https://docs.rs/emit/1.18.0/emit/metric/struct.Reporter.html) type will attempt to normalize the extents of any metrics sampled from its sources. Normalization will:

1. Take the current timestamp, `now`, when sampling metrics.
2. If the metric sample has no extent, or has a point extent, it will be replaced with `now`.
3. If the metric sample has a range extent, the end will be set to `now` and the start will be `now` minus the original length. If this would produce an invalid range then the original is kept.

When the `std` Cargo feature is enabled this will be done automatically. In other cases, normalization won't happen unless it's configured by [`Reporter::normalize_with_clock`](https://docs.rs/emit/1.18.0/emit/metric/struct.Reporter.html#method.normalize_with_clock).

Normalization can be disabled by calling [`Reporter::without_normalization`](https://docs.rs/emit/1.18.0/emit/metric/struct.Reporter.html#method.without_normalization).
