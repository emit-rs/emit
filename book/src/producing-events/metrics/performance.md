# Performance considerations for metrics

Also see [Performance considerations for logging](../logging/performance.md).

## Process samples before converting them to `Metric`s

`emit`'s infrastructure for metrics is largely concerned with emitting them, not with processing them. If you need to perform normalization or aggregation, do it before your samples reach `emit`. This way you can define a stronger data model around the kinds of metrics your application produces, instead of trying to work with `emit`'s loosely typed bags of properties.
