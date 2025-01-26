# Histogram metrics

Count metrics (those where their `metric_agg` is `"count"`) can carry the following additional properties that better describe the underlying data:

- `bucket_values`
- `bucket_ranges` or `bucket_scale`

## Bucketing

The total count of underlying values in a histogram metric is its `metric_value` property. That count can be broken down into buckets in the `bucket_values` property. `bucket_values` is a sequence, where each element has the same type as `metric_value`. Here's an example:

```rust
# extern crate emit;
emit::emit!(
    "{metric_agg} of {metric_name} is {metric_value}",
    evt_kind: "metric",
    metric_agg: "count",
    metric_name: "request_time",
    metric_unit: "ms",
    metric_value: 100,
    bucket_values: [
        10,
        30,
        20,
        20,
        10,
        10,
    ],
);
```

`bucket_values` breaks the overall count into smaller buckets, but that still doesn't tell us much. To make sense of the buckets, we need to know what range of the underlying data they're aggregating. This is where the `bucket_ranges` and `bucket_scale` properties come in.

`bucket_scale` is a numeric value that determines the size of buckets in a log scale.

`bucket_ranges` is a numeric sequence the same length as `bucket_values` that gives each bucket its upper bound.

## Time-series histograms

If the metric is a [time-series](./time-series-metrics.md), then `bucket_values` will carry a sequence of sequences. The outer sequence is the histgram of each bucket in the time-series. The inner sequence is the buckets of that histogram.
