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
