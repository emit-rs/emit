# Metrics data model

The data model of metrics is an extension of [`emit`'s events](../../reference/events.md). Metric events are points or buckets in a time-series. They don't model the underlying instruments collecting metrics like counters or gauges. They instead model the aggregation of readings from those instruments over their lifetime. Metric events include the following [well-known properties](https://docs.rs/emit/1.11.1/emit/well_known/index.html):

- `evt_kind`: with a value of `"metric"` to indicate that the event is a metric sample.
- `metric_agg`: the aggregation over the underlying data stream that produced the sample.
    - `"count"`: A monotonic sum of `1`'s for defined values, and `0`'s for undefined values.
    - `"sum"`: A potentially non-monotonic sum of defined values.
    - `"min"`: The lowest ordered value.
    - `"max"`: The largest ordered value.
    - `"last"`: The most recent value.
- `metric_name`: the name of the underlying data stream.
- `metric_value`: the sample itself. These values are expected to be numeric.
