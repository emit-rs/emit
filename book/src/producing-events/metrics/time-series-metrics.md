## Time-series metrics

Metric events where their extent is a time range and the `metric_value` is an array are a complete time-series. Each element in the array is a bucket in the time-series. The width of each bucket is the length of the extent divided by the number of buckets.

The following metric is for a time-series with 15 buckets, where each bucket covers 1 second:

```rust
# extern crate emit;
let now = emit::clock().now();
let last_sample = now.map(|now| now - std::time::Duration::from_secs(15));

emit::count_sample!(
    extent: last_sample..now,
    name: "bytes_written",
    #[emit::as_value]
    value: [
        0,
        5,
        56,
        0,
        0,
        221,
        7,
        0,
        0,
        5,
        876,
        0,
        194,
        0,
        18,
    ],
);
```

```text
Event {
    mdl: "my_app",
    tpl: "{metric_agg} of {metric_name} is {metric_value}",
    extent: Some(
        "2024-04-30T07:03:07.828185000Z".."2024-04-30T07:03:22.828185000Z",
    ),
    props: {
        "evt_kind": metric,
        "metric_name": "bytes_written",
        "metric_agg": "count",
        "metric_value": [
            0,
            5,
            56,
            0,
            0,
            221,
            7,
            0,
            0,
            5,
            876,
            0,
            194,
            0,
            18,
        ],
    },
}
```
