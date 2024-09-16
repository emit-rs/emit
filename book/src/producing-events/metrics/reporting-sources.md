# Reporting metric sources

The [`Source`] trait represents some underlying data source that can be sampled to provide [`Metric`]s. You can sample sources directly, or combine them into a [`Reporter`] to sample all the sources of metrics in your application together:

```rust
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
        // You could also use `sample_metrics` here and tweak the extents of metrics
        // to ensure they're all aligned together
        reporter.emit_metrics(emit::runtime::shared());

        std::thread::sleep(std::time::Duration::from_secs(30));
    }
});
```
