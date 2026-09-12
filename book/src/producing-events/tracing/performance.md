# Performance considerations for tracing

Also see [Performance considerations for logging](../logging/performance.md).

## Ambient span properties require buffering

Properties captured by the `#[span]` template and specified afterwards are added to ambient context. This likely involves buffering those properties into owned values and can be expensive. You can specify most properties as private to the span itself using the `evt_props` [control parameter](../../reference/control-parameters.md) instead to avoid buffering them. See [Visibility of properties on spans](./properties.md#visibility-of-properties-on-spans) for details.
