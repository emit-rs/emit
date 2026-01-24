# Attaching properties to metrics

Metric events can carry other properties in addition to their metadata:

```rust
# extern crate emit;
emit::sample!(
    name: "bytes_written",
    value: 591,
    props: emit::props! {
        // Additional properties
        file: "./log.1.txt",
        version: "1.2.3-dev",
    },
);
```

```text
Event {
    mdl: "my_app",
    tpl: "{metric_agg} of {metric_name} is {metric_value}",
    extent: Some(
        "2024-04-30T06:53:41.069203000Z",
    ),
    props: {
        "evt_kind": metric,
        "metric_name": "bytes_written",
        "metric_agg": "count",
        "metric_value": 591,
        "file": "./log.1.txt",
        "version": "1.2.3-dev",
    },
}
```

The [`Metric`](https://docs.rs/emit/1.16.1/emit/metric/struct.Metric.html) type accepts additional properties as an argument to its constructor:

```rust
# extern crate emit;
emit::emit!(
    evt: emit::Metric::new(
        // Metadata
        emit::mdl!(),
        "bytes_written",
        "count",
        emit::clock().now(),
        591,
        // Additional properties
        emit::props! {
            file: "./log.1.txt",
            version: "1.2.3-dev",
        },
    ),
);
```
