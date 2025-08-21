# Metrics data model

The data model of metrics is an extension of [`emit`'s events](../../reference/events.md). Metric events are points or buckets in a time-series. They don't model the underlying instruments collecting metrics like counters or gauges. They instead model the aggregation of readings from those instruments over their lifetime. Metric events include the following [well-known properties](https://docs.rs/emit/1.12.0/emit/well_known/index.html):

- `evt_kind`: with a value of `"metric"` to indicate that the event is a metric sample.
- `metric_agg`: the aggregation over the underlying data stream that produced the sample.
    - `"count"`: A monotonic sum of `1`'s for defined values, and `0`'s for undefined values.
    - `"sum"`: A potentially non-monotonic sum of defined values.
    - `"min"`: The lowest ordered value.
    - `"max"`: The largest ordered value.
    - `"last"`: The most recent value.
- `metric_name`: the name of the underlying data stream.
- `metric_value`: the sample itself. These values are expected to be numeric.

## The type of `metric_value`

The type of `metric_value` isn't constrained in any way, but will enjoy the broadest compatibility when it's numeric. Integral counts are best represented as `usize`, and derived statistics and other fractional values as `f64`. Keep in mind the potential for rounding errors and precision loss particularly when processing `f64` samples.
