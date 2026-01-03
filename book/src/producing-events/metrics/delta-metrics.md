# Delta metrics

Metric events where their extent is a time range are deltas. Their `metric_value` is the result of applying their `metric_agg` over the underlying stream within the extent.

The following metric reports that the number of bytes written changed by 17 over the last 30 seconds:

```rust
# extern crate emit;
let now = emit::clock().now();
let last_sample = now.map(|now| now - std::time::Duration::from_secs(30));

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

You can use the [`Delta`](https://docs.rs/emit/1.14.0/emit/metric/struct.Delta.html) type to implement metric sources that track deltas instead of cumulative values. The `Delta` type tracks the range the value covers, automatically updating it when sampled.

```rust
# extern crate emit;
use std::sync::Mutex;

// This example synchronizes with a `Mutex`. Other strategies are also possible,
// like `RwLock` with `AtomicUsize`, depending on the underlying value type.
pub struct BytesWritten(Mutex<emit::metric::Delta<usize>>);

impl BytesWritten {
    // Accumulate into the metric
    pub fn extend(&self, value: usize) {
        *self.0.lock().unwrap().current_value_mut() += value;
    }
}

impl emit::metric::Source for BytesWritten {
    fn sample_metrics<S: emit::metric::Sampler>(&self, sampler: S) {
        let mut guard = self.0.lock().unwrap();

        // Get the value for the current time period and an extent covering it
        let (extent, value) = guard.advance(sampler.now().or_else(|| emit::clock().now()));
        let bytes_written = *value;
        
        // Reset the delta for the new time period
        *value = 0;

        drop(guard);

        // Report the delta
        sampler.metric(emit::count_metric!(
            extent,
            value: bytes_written,
        ));
    }
}
```

See [Reporting sources](./reporting-sources.md) for details on how to sample a [`Source`](https://docs.rs/emit/1.14.0/emit/metric/source/trait.Source.html).
