# Metric sample creation

`emit`'s metric infrastructure works with [`Metric`](https://docs.rs/emit/1.18.0/emit/metric/struct.Metric.html) samples, which are a kind of [`Event`](https://docs.rs/emit/1.18.0/emit/struct.Event.html) specialized for carrying metric samples.

## Using `Metric` directly

[`Metric`](https://docs.rs/emit/1.18.0/emit/metric/struct.Metric.html)s can be constructed manually:

```rust
# extern crate emit;
// Construct a `Metric` manually
let metric = emit::Metric::new(
    // The module that owns the metric
    emit::mdl!(),
    // The name of the metric
    "my_metric",
    // The aggregation used to produce the sample
    "count",
    // The time when the sample was produced, or the span of time it covers
    emit::clock().now(),
    // The metric sample itself
    42,
    // Additional properties for the metric
    emit::Empty,
);
```

## Using macros

`emit` also defines macros for producing metric samples for specific aggregations. Each well-known aggregation has a corresponding macro:

- [`count_metric!`](https://docs.rs/emit/1.18.0/emit/macro.count_metric_.html) for samples of a monotonic counter.
- [`sum_metric!`](https://docs.rs/emit/1.18.0/emit/macro.sum_metric_.html) for samples of a non-monotonic sum.
- [`min_metric!`](https://docs.rs/emit/1.18.0/emit/macro.min_metric_.html) for samples of the minimum observed value.
- [`max_metric!`](https://docs.rs/emit/1.18.0/emit/macro.max_metric_.html) for samples of the maximum observed value.
- [`last_metric!`](https://docs.rs/emit/1.18.0/emit/macro.last_metric_.html) for samples of the latest value.

This example produces an equivalent [`Metric`](https://docs.rs/emit/1.18.0/emit/metric/struct.Metric.html)) to the manual one above:

```rust
# extern crate emit;
let metric = emit::count_metric!(name: "my_metric", value: 42);
```

The `name` and `value` control parameters are required. If the `value` is bound to an identifier then that identifier will be used as the `name` by default. The example below is equivalent to the ones above:

```rust
# extern crate emit;
let my_metric = 42;

let metric = emit::count_metric!(value: my_metric);
```
