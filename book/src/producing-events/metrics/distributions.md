# Distributions

Distributions are an optional extension to metric samples that tell you more about the underlying data that produced them. Properties with a `dist_` prefix describe the data that contributed to a sample in more detail. Each of these properties is optional.

Emitters that are distribution-aware may treat events that carry them differently. [`emit_otlp`](../../emitting-events/otlp.md) treats samples carrying an [exponential histogram](#exponential-histograms) as an [OTLP exponential histogram](https://opentelemetry.io/docs/specs/otel/metrics/data-model/#exponentialhistogram). [`emit_term`](../../emitting-events/console.md) summarizes these same samples with [quartiles](https://en.wikipedia.org/wiki/Quartile).

## Sums and Extrema

Say we have the following metric sample:

```rust
# extern crate emit;
emit::count_sample!(name: "http_response", value: 500);
```

This tells us we've seen 500 values, but doesn't tell us anything about those 500 values themselves.

Attaching the sum of values with `dist_sum` gives us some more information:

```rust
# extern crate emit;
emit::count_sample!(
    name: "http_response",
    value: 500,
    props: emit::props! {
        dist_sum: 1689628,
    },
);
```

With both the sum and the count, we can compute the mean as `3379.256`.

The mean gives us a central point for the dataset, but the same mean could come from very different bounds.

Attaching the extrema with `dist_min` and `dist_max` further tells us what the range of values is:

```rust
# extern crate emit;
emit::count_sample!(
    name: "http_response",
    value: 500,
    props: emit::props! {
        dist_sum: 1689628,
        dist_min: 100,
        dist_max: 29046,
    }
);
```

### Distribution properties vs separate metrics

The `dist_sum`, `dist_count`, `dist_min`, and `dist_max` properties each have a corresponding value for `metric_agg`. For example, if we take the final sample from earlier we could split it into 4 individual samples instead:

```rust
# extern crate emit;
emit::count_sample!(name: "http_response", value: 500);
emit::sum_sample!(name: "http_response", value: 1689628);
emit::min_sample!(name: "http_response", value: 100);
emit::max_sample!(name: "http_response", value: 29046);
```

The difference between these two representations is whether those individual samples are valuable in their own right. Emitters may ignore distribution properties, so if you want to track that aggregation, then prefer separate samples.

## Exponential histograms

A histogram is a compression of the underlying data source that buckets nearby values together and counts them, rather than storing the raw values themselves. Histograms give you an idea of how values are distributed across their range.

An exponential histogram automatically sizes its buckets using an exponential function, so buckets closer to zero are smaller (more accurate) than buckets further away from zero. They're good for _light-tail_ distributions, where values are clustered near the front and extremes are rare. Light-tail distributions have roughly this shape:

![An example of an exponential distribution](../../asset/exp-dist.svg)

Typical web request latencies follow this shape. Most requests for a given endpoint complete around the same time, but in rare circumstances they may take much longer.

`emit` supports attaching an exponential histogram to a metric sample with the `dist_exp_scale` and `dist_exp_buckets` properties:

```rust
# extern crate emit;
emit::count_sample!(
    name: "http_response",
    value: 500,
    props: emit::props! {
        dist_sum: 1689628,
        dist_min: 100,
        dist_max: 29046,
        dist_exp_scale: 2,
        #[emit::as_serde]
        dist_exp_buckets: [
            (99.07220457217667, 7),
            (117.81737057623761, 7),
            (140.10925536017402, 7),
            (166.61892335205206, 7),
            (198.14440914435335, 6),
            (235.63474115247521, 6),
            (280.218510720348, 9),
            (333.2378467041041, 9),
            (396.2888182887066, 12),
            (471.2694823049503, 11),
            (560.4370214406958, 13),
            (666.475693408208, 13),
            (792.5776365774132, 15),
            (942.5389646099006, 19),
            (1120.8740428813917, 24),
            (1332.951386816416, 24),
            (1585.1552731548263, 20),
            (1885.0779292198008, 21),
            (2241.748085762783, 32),
            (2665.9027736328317, 37),
            (3170.3105463096517, 28),
            (3770.1558584396016, 34),
            (4483.496171525566, 27),
            (5331.8055472656615, 34),
            (6340.621092619303, 34),
            (7540.311716879201, 19),
            (8966.99234305113, 5),
            (10663.611094531323, 2),
            (12681.242185238603, 2),
            (15080.623433758403, 4),
            (21327.222189062646, 3),
            (25362.484370477203, 8),
            (30161.2468675168, 1),
        ],
    }
);
```

### Managing accuracy and memory usage

Picking a bucket scale is a trade-off between memory usage and accuracy. Without knowing the underlying distribution of your data it's not possible to pick a correct value for your scale upfront. You can target a maximum number of buckets though, and reduce your scale as you overflow your maximum buckets. The process works like this:

1. Pick a maximum number of buckets, for example `160`.
2. Pick a high initial scale, for example `20`.
3. Store samples at the target scale.
4. Once the number of stored buckets overflows your maximum buckets, decrement the scale and re-ingest the samples at the new scale. You'll end up with half the number of buckets.

### Exponential histograms in detail

Exponential histograms internally use γ, a value close to `1`, as a log base for computing the bucket a sample belongs to. 

#### Computing γ

γ is a value close to 1 that's used as a log base for computing the bucket a value belongs to.

From `scale`:

\\[ γ = 2^{2^{-scale}} \\]

From `error`:

\\[ γ = \frac{1 + error}{1 - error} \\]

#### Computing `index`

The index is the bucket that a value belongs to. Values that are close together will share the same bucket.

\\[ index = \lceil{\log_γ (\lvert{value}\rvert)}\rceil \\]

#### Computing `midpoint`

The midpoint is a value at the center of a bucket index.

\\[ midpoint = \frac{γ^{index - 1} + γ^{index}}{2} \\]

#### Rescaling

If you have a scale and range of bucket indexes, you can compute a new scale that fits them into a target maximum number of buckets.

\\[ scale_1 = scale_0 - \lceil{\log_2 \left({\frac{index_{max} - index_{min}}{size}}\right)}\rceil \\]

#### Computing `error` from `scale`

If you have a scale, you can compute the error value from it, which gives you an idea of how accurate bucket values are. The error is slightly misleading because it's a percentage rather than an absolute value. Larger values can be further from their midpoint than smaller ones.

\\[ error = \frac{2^{2^{-scale}} - 1}{2^{2^{-scale}} + 1} \\]
