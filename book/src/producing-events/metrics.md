# Metrics

Metrics are an effective approach to monitoring applications at scale. They can be cheap to collect, making them suitable for performance sensitive operations. They can also be compact to report, making them suitable for high-volume scenarios. `emit` doesn't provide much infrastructure for collecting or sampling metrics. What it does provide is a standard way to report metric samples as events.

A standard kind of metric is a monotonic counter, which can be represented as an atomic integer. In this example, our counter is for the number of bytes written to a file, which we'll call `bytes_written`. We can report a sample of this counter as an event by wrapping it in a [`Metric`]:

```rust
# fn sample_bytes_written() -> usize { 4643 }
use emit::{well_known::METRIC_AGG_COUNT, Clock};

let sample = sample_bytes_written();

emit::emit!(
    evt: emit::Metric::new(
        emit::mdl!(),
        "bytes_written",
        METRIC_AGG_COUNT,
        emit::Empty,
        sample,
        emit::Empty,
    )
);
```

```text
Event {
    mdl: "my_app",
    tpl: "`metric_agg` of `metric_name` is `metric_value`",
    extent: Some(
        "2024-04-29T10:08:24.780230000Z",
    ),
    props: {
        "evt_kind": metric,
        "metric_name": "bytes_written",
        "metric_agg": "count",
        "metric_value": 4643,
    },
}
```

Metrics may also be emitted manually:

```rust
# fn sample_bytes_written() -> usize { 4643 }
use emit::well_known::{EVENT_KIND_METRIC, METRIC_AGG_COUNT};

let sample = sample_bytes_written();

emit::emit!(
    "{metric_agg} of {metric_name} is {metric_value}",
    evt_kind: EVENT_KIND_METRIC,
    metric_agg: METRIC_AGG_COUNT,
    metric_name: "bytes_written",
    metric_value: sample,
);
```
