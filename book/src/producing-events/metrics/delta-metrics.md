## Delta metrics

Metric events with a span extent are deltas. Their `metric_value` is the result of applying their `metric_agg` over the underlying stream within the extent.

The following metric reports that the number of bytes written changed by 17 over the last 30 seconds:

```rust
# extern crate emit;
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
