# Reporting metric sources

The [`Source`](https://docs.rs/emit/1.8.1/emit/metric/source/trait.Source.html) trait represents some underlying data source that can be sampled to provide [`Metric`](https://docs.rs/emit/1.8.1/emit/metric/struct.Metric.html)s. You can sample sources directly, or combine them into a [`Reporter`](https://docs.rs/emit/1.8.1/emit/metric/struct.Reporter.html) to sample all the sources of metrics in your application together:

```rust
# extern crate emit;
use emit::metric::{Source as _, Sampler as _};

// Create some metric sources
let source_1 = emit::metric::source::from_fn(|sampler| {
    sampler.metric(emit::Metric::new(
        emit::path!("source_1"),
        "bytes_written",
        emit::well_known::METRIC_AGG_COUNT,
        emit::Empty,
        1,
        emit::Empty,
    ));
});

let source_2 = emit::metric::source::from_fn(|sampler| {
    sampler.metric(emit::Metric::new(
        emit::path!("source_2"),
        "bytes_written",
        emit::well_known::METRIC_AGG_COUNT,
        emit::Empty,
        2,
        emit::Empty,
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

## Normalization of timestamps

The [`Reporter`](https://docs.rs/emit/1.8.1/emit/metric/struct.Reporter.html) type will attempt to normalize the extents of any metrics sampled from its sources. Normalization will:

1. Take the current timestamp, `now`, when sampling metrics.
2. If the metric sample has no extent, or has a point extent, it will be replaced with `now`.
3. If the metric sample has a range extent, the end will be set to `now` and the start will be `now` minus the original length. If this would produce an invalid range then the original is kept.

When the `std` Cargo feature is enabled this will be done automatically. In other cases, normalization won't happen unless it's configured by [`Reporter::normalize_with_clock`](https://docs.rs/emit/1.8.1/emit/metric/struct.Reporter.html#method.normalize_with_clock).

Normalization can be disabled by calling [`Reporter::without_normalization`](https://docs.rs/emit/1.8.1/emit/metric/struct.Reporter.html#method.without_normalization).
