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

## Cumulative metrics

Metric events with a point extent are cumulative. Their `metric_value` is the result of applying their `metric_agg` over the entire underlying stream up to that point.

The following metric reports the current number of bytes written as 591:

```rust
use emit::{Clock, well_known::METRIC_AGG_COUNT};

emit::emit!(
    evt: emit::Metric::new(
        emit::mdl!(),
        "bytes_written",
        METRIC_AGG_COUNT,
        emit::Empty,
        591,
        emit::Empty,
    )
);
```

```text
Event {
    mdl: "my_app",
    tpl: "`metric_agg` of `metric_name` is `metric_value`",
    extent: Some(
        "2024-04-30T06:53:41.069203000Z",
    ),
    props: {
        "evt_kind": metric,
        "metric_name": "bytes_written",
        "metric_agg": "count",
        "metric_value": 591,
    },
}
```

## Delta metrics

Metric events with a span extent are deltas. Their `metric_value` is the result of applying their `metric_agg` over the underlying stream within the extent.

The following metric reports that the number of bytes written changed by 17 over the last 30 seconds:

```rust
use emit::{Clock, well_known::METRIC_AGG_COUNT};

let now = emit::clock().now();
let last_sample = now.map(|now| now - std::time::Duration::from_secs(30));

emit::emit!(
    evt: emit::Metric::new(
        emit::mdl!(),
        "bytes_written",
        METRIC_AGG_COUNT,
        last_sample..now,
        17,
        emit::Empty,
    )
);
```

```text
Event {
    mdl: "my_app",
    tpl: "`metric_agg` of `metric_name` is `metric_value`",
    extent: Some(
        "2024-04-30T06:55:59.839770000Z".."2024-04-30T06:56:29.839770000Z",
    ),
    props: {
        "evt_kind": metric,
        "metric_name": "bytes_written",
        "metric_agg": "count",
        "metric_value": 17,
    },
}
```

## Time-series metrics

Metric events with a span extent, where the `metric_value` is an array are a complete time-series. Each element in the array is a bucket in the time-series. The width of each bucket is the length of the extent divided by the number of buckets.

The following metric is for a time-series with 15 buckets, where each bucket covers 1 second:

```rust
use emit::{Clock, well_known::METRIC_AGG_COUNT};

let now = emit::clock().now();
let last_sample = now.map(|now| now - std::time::Duration::from_secs(15));

emit::emit!(
    evt: emit::Metric::new(
        emit::mdl!(),
        "bytes_written",
        METRIC_AGG_COUNT,
        last_sample..now,
        &[
            0,
            5,
            56,
            0,
            0,
            221,
            7,
            0,
            0,
            5,
            876,
            0,
            194,
            0,
            18,
        ],
        emit::Empty,
    )
);
```

```text
Event {
    mdl: "my_app",
    tpl: "`metric_agg` of `metric_name` is `metric_value`",
    extent: Some(
        "2024-04-30T07:03:07.828185000Z".."2024-04-30T07:03:22.828185000Z",
    ),
    props: {
        "evt_kind": metric,
        "metric_name": "bytes_written",
        "metric_agg": "count",
        "metric_value": [
            0,
            5,
            56,
            0,
            0,
            221,
            7,
            0,
            0,
            5,
            876,
            0,
            194,
            0,
            18,
        ],
    },
}
```
