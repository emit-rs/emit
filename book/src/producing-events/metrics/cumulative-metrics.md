## Cumulative metrics

Metric events where their extent is a point are cumulative. Their `metric_value` is the result of applying their `metric_agg` over the entire underlying stream up to that point.

The following metric reports the current number of bytes written as 591:

```rust
# extern crate emit;
emit::emit!(
    "{metric_agg} of {metric_name} is {metric_value}",
    evt_kind: "metric",
    metric_agg: "count",
    metric_name: "bytes_written",
    metric_value: 591,
);
```

```text
Event {
    mdl: "my_app",
    tpl: "{metric_agg} of {metric_name} is {metric_value}",
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
