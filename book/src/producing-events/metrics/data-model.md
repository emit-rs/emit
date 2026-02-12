# Metrics data model

The data model of metrics is an extension of [`emit`'s events](../../reference/events.md). Metric events are points or buckets in a time-series. They don't model the underlying instruments collecting metrics like counters or gauges. They instead model the aggregation of readings from those instruments over their lifetime. Metric events include the following [well-known properties](https://docs.rs/emit/1.17.2/emit/well_known/index.html):

- `evt_kind`: with a value of `"metric"` to indicate that the event is a metric sample.
- `metric_agg`: the aggregation over the underlying data stream that produced the sample.
    - `"count"`: a monotonic sum of `1`'s for defined values, and `0`'s for undefined values.
    - `"sum"`: a potentially non-monotonic sum of defined values.
    - `"min"`: the lowest ordered value.
    - `"max"`: the largest ordered value.
    - `"last"`: the most recent value.
- `metric_name`: the name of the underlying data stream.
- `metric_value`: the sample itself. These values are expected to be numeric.
- `metric_unit`: the unit of the metric sample.
- `metric_description`: an end-user description of the data stream.

## The type of `metric_value`

The type of `metric_value` isn't constrained in any way, but will enjoy the broadest compatibility when it's numeric. Integral counts are best represented as `usize`, and derived statistics and other fractional values as `f64`. Keep in mind the potential for rounding errors and precision loss particularly when processing `f64` samples.

## Distributions

Metric samples can optionally carry additional properties that describe the distribution of the data that produced them. The well-known properties for distributions are:

- `dist_count`: the count of values, like `metric_value` when `metric_agg` is `"count"`.
- `dist_sum`: the sum of values, like `metric_value` when `metric_agg` is `"sum"`.
- `dist_min`: the minimum value, like `metric_value` when `metric_agg` is `"min"`.
- `dist_count`: the maximum value, like `metric_value` when `metric_agg` is `"max"`.
- Exponential histograms as described in [Distributions](./distributions.md#exponential-histograms), using _both_ of the following properties:
    - `dist_exp_scale`: the scale of the histogram. The value is expected to be an integer.
    - `dist_exp_buckets`: the bucket midpoints and counts as a `[(f64, u64)]` sequence.
