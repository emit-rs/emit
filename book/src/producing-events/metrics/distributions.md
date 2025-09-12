# Distributions

Distributions are an optional extension to metric samples that tell you more about the underlying data that produced them. Properties with a `dist_` prefix describe the data that contributed to a sample in more detail. Each of these properties is optional.

## Sums and Extrema

Say we have the following metric sample:

```json
{
    "metric_agg": "count",
    "metric_value": 100,
}
```

This tells us we've seen 100 values, but doesn't tell us anything about those 100 values themselves.

Attaching the sum of values with `dist_sum` gives us some more information:

```json
{
    "metric_agg": "count",
    "metric_value": 100,
    "dist_sum": 350,
}
```

With both the sum and the count, we can compute the mean as `3.5`.

The mean gives us a central point for the dataset, but the same mean could come from very different bounds.

Attaching the extrema with `dist_min` and `dist_max` further tells us what the range of values is:

```json
{
    "metric_agg": "count",
    "metric_value": 100,
    "dist_sum": 350,
    "dist_min": 0,
    "dist_max": 50,
}
```

### Distribution properties vs separate metrics

The `dist_sum`, `dist_count`, `dist_min`, and `dist_max` properties each have a corresponding value for `metric_agg`. For example, if we take the final sample from earlier:

```json
{
    "metric_agg": "count",
    "metric_value": 100,
    "dist_sum": 350,
    "dist_min": 0,
    "dist_max": 50,
}
```

we could split it into 4 individual samples instead:

```json
{
    "metric_agg": "count",
    "metric_value": 100,
}
{
    "metric_agg": "sum",
    "metric_value": 350,
}
{
    "metric_agg": "min",
    "metric_value": 0,
}
{
    "metric_agg": "max",
    "metric_value": 50,
}
```

The difference between these two representations is whether those individual samples are valuable in their own right.

## Exponential histograms

A histogram is a compression of the underlying data source that buckets nearby values together and counts them, rather than storing the raw values themselves. Histograms give you an idea of how values are distributed across their range.

An exponential histogram automatically sizes its buckets using an exponential function, so buckets closer to zero are smaller (more accurate) than buckets further away from zero. They're good for _light-tail_ distributions, where values are clustered near the front and extremes are rare. Typical web request latencies are an example of a light-tail distribution where most requests for a given endpoint complete around the same time, but in very rare circumstances they may take much longer.

`emit` supports attaching an exponential histogram to a metric sample with the `dist_scale` and `dist_bucket` properties:

```json
{
    "metric_agg": "count",
    "metric_value": 100,
    "dist_buckets": [
        [1, 1],
        [1, 1],
    ],
    "dist_scale": 5,
}
```

### Managing accuracy and memory usage

Pick a large scale, say `20`, and a maximum number of buckets, say `160`. Store samples at the target scale. Once the number of stored buckets overflows the maximum, decrement the scale and re-ingest the samples at the new scale. You'll end up with half the number of buckets.

### Computing γ

From `scale`:

\\[ γ = 2^{2^{-scale}} \\]

From `error`:

\\[ γ = \frac{1 + error}{1 - error} \\]

### Computing `index`

\\[ index = \lceil{\log_γ (\lvert{value}\rvert)}\rceil \\]

### Computing `midpoint`

\\[ midpoint = \frac{γ^{index - 1} + γ^{index}}{2} \\]

### Rescaling

\\[ scale_1 = scale_0 - \lceil{\log_2 \left({\frac{index_{max} - index_{min}}{size}}\right)}\rceil \\]

### Computing `error` from `scale`

\\[ error = \frac{2^{2^{-scale}} - 1}{2^{2^{-scale}} + 1} \\]
