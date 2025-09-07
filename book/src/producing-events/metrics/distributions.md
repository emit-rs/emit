# Distributions

Distributions are an optional extension to metric samples that tell you more about the underlying data that produced them. Properties with a `dist_` prefix describe the data that contributed to a sample in more detail.

## Sums and Extrema

Say we have the following metric sample:

```rust
{
    "metric_agg": "count",
    "metric_value": 100,
}
```

This tells us we've seen 100 values, but doesn't tell us anything about those hundred values themselves.

By attaching the sum of values with `dist_sum` we can now compute the mean as `3.5`:

```rust
{
    "metric_agg": "count",
    "metric_value": 100,
    "dist_sum": 350,
}
```

Attaching the extrema with `dist_min` and `dist_max` further tells us what the range of values is:

```rust
{
    "metric_agg": "count",
    "metric_value": 100,
    "dist_sum": 350,
    "dist_min": 0,
    "dist_max": 50,
}
```

These `dist_sum`, `dist_min`, and `dist_max` properties are all optional, and can be supplied or omitted in any configuration.

## Exponential histograms

A histogram is a compression of the underlying data source that buckets nearby values together and counts them, rather than storing the raw values themselves. Histograms give you an idea of how values are distributed across their range.

An exponential histogram automatically sizes its buckets using an exponential function, so buckets closer to zero are smaller (more accurate) than buckets further away from zero. They're good for distributions with a long tail, like typical web request latencies.

`emit` supports attaching an exponential histogram to a metric sample with the `dist_scale` and `dist_bucket` properties:

```rust
{
    "metric_agg": "count",
    "metric_value": 100,
    "dist_sum": 350,
    "dist_min": 0,
    "dist_max": 50,
    "dist_buckets": [
        (x, y),
        (x, y),
    ],
    "dist_scale": 5,
}
```

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
