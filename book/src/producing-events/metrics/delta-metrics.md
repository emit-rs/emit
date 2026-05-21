# Delta metrics

Metric events where their extent is a time range are deltas. Their `metric_value` is the result of applying their `metric_agg` over the underlying stream within the extent.

The following metric reports that the number of bytes written changed by 17 over the last 30 seconds:

```rust
# extern crate emit;
// Compute a time range that the sample covers
let now = emit::clock().now();
let last_sample = now.map(|now| now - std::time::Duration::from_secs(30));

// Passing the time range as the sample's extent creates a delta
emit::count_sample!(extent: last_sample..now, name: "bytes_written", value: 17);
```

```text
Event {
    mdl: "my_app",
    tpl: "{metric_agg} of {metric_name} is {metric_value}",
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

## The `Delta` type

You can use the [`Delta`](https://docs.rs/emit/1.18.0/emit/metric/struct.Delta.html) type to implement metric sources that track deltas instead of cumulative values. The `Delta` type tracks the range the value covers, automatically updating it when sampled.

```rust
# extern crate emit;
// Wrap a value in a delta
// The delta accepts a timestamp for the start of its initial time interval
let mut delta = emit::metric::Delta<usize>::new_default(emit::clock().now());

// Update the value for the current time period
delta.current_value_mut() += 1;
delta.current_value_mut() += 1;

// At some regular interval, pull the built up value and emit it
// Advancing the delta returns a tuple of:
//   1. The extent covering the time since it was last advanced. In this example, that's the two calls to `emit::clock().now()`
//   2. The value that was built up over the interval
let (extent, my_metric) = delta.advance_default(emit::clock().now());

// Emit the metric sample as an event
emit::sample(extent, value: my_metric);
```

Internally, [`Delta`](https://docs.rs/emit/1.18.0/emit/metric/struct.Delta.html) just tracks the last [`Timestamp`](https://docs.rs/emit/1.18.0/emit/struct.Timestamp.html) passed to `advance` (which is the start of the current interval) and the value for the current interval. `Delta` relies on external mutability, so you'll need to wrap it in a mutex to share it, but can be used for arbitrarily complex metric sources, like [`Distribution`](https://docs.rs/emit/1.18.0/emit/metric/exp/struct.Distribution.html)s.

See [Reporting sources](./reporting-sources.md) for details on how to sample a [`Source`](https://docs.rs/emit/1.18.0/emit/metric/source/trait.Source.html) containing a `Delta`.
