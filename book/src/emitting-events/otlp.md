# Emitting via OTLP

You can use [`emit_otlp`](https://docs.rs/emit_otlp/1.5.0/emit_otlp/index.html) to emit diagnostic events to remote OpenTelemetry-compatible services.

OpenTelemetry defines a wire protocol for exchanging diagnostic data called [OTLP](https://opentelemetry.io/docs/specs/otlp/). If you're using a modern telemetry backend then chances are it supports OTLP either directly or through [OpenTelemetry's Collector](https://opentelemetry.io/docs/collector/).

`emit_otlp` is an independent implementation of OTLP that maps `emit`'s events onto the OpenTelemetry data model. `emit_otlp` doesn't rely on the OpenTelemetry SDK or any gRPC or protobuf tooling, so can be added to any Rust application without requiring changes to your build process.

```toml
[dependencies.emit_otlp]
version = "1.5.0"
```

```rust
# extern crate emit;
# extern crate emit_otlp;
fn main() {
    let rt = emit::setup()
        .emit_to(emit_otlp::new()
            // Add required resource properties for OTLP
            .resource(emit::props! {
                #[emit::key("service.name")]
                service_name: "my_app",
            })
            // Configure endpoints for logs/traces/metrics using gRPC + protobuf
            .logs(emit_otlp::logs_grpc_proto("http://localhost:4319"))
            .traces(emit_otlp::traces_grpc_proto("http://localhost:4319"))
            .metrics(emit_otlp::metrics_grpc_proto("http://localhost:4319"))
            .spawn())
        .init();

    // Your app code goes here

    rt.blocking_flush(std::time::Duration::from_secs(30));
}
```

See [the crate docs](https://docs.rs/emit_otlp/1.5.0/emit_otlp/index.html) for more details.

## Logs

Any event can be treated as a [log event](https://opentelemetry.io/docs/specs/otel/logs/). You need to configure a logs endpoint in your `emit_otlp` setup for this to happen. See [the crate docs](https://docs.rs/emit_otlp/1.5.0/emit_otlp/index.html#logs) for details.

## Traces

Events in `emit`'s [tracing data model](../producing-events/tracing/data-model.md) can be treated as a [trace span](https://opentelemetry.io/docs/specs/otel/trace/). You need to configure a traces endpoint in your `emit_otlp` setup for this to happen, otherwise they'll be treated as logs. See [the crate docs](https://docs.rs/emit_otlp/1.5.0/emit_otlp/index.html#traces) for details.

## Metrics

Events in `emit`'s [metrics data model](../producing-events/metrics/data-model.md) can be treated as a [metric](https://opentelemetry.io/docs/specs/otel/metrics/). You need to configure a metrics endpoint in your `emit_otlp` setup for this to happen, otherwise they'll be treated as logs. See [the crate docs](https://docs.rs/emit_otlp/1.5.0/emit_otlp/index.html#metrics) for details.

## Supported protocols

`emit_otlp` supports sending OTLP using [gRPC](https://docs.rs/emit_otlp/1.5.0/emit_otlp/index.html#configuring-for-grpcprotobuf), [HTTP+protobuf](https://docs.rs/emit_otlp/1.5.0/emit_otlp/index.html#configuring-for-httpprotobuf), and [HTTP+JSON](https://docs.rs/emit_otlp/1.5.0/emit_otlp/index.html#configuring-for-httpjson).

## TLS

`emit_otlp` supports TLS using the default Cargo features when your endpoint uses the `https` scheme. See [the crate docs](https://docs.rs/emit_otlp/1.5.0/emit_otlp/index.html#configuring-tls) for details.

## Compression

`emit_otlp` will compress payloads using gzip using the default Cargo features. See [the crate docs](https://docs.rs/emit_otlp/1.5.0/emit_otlp/index.html#configuring-compression) for details.

## HTTP headers

`emit_otlp` supports custom HTTP headers per endpoint. See [the crate docs](https://docs.rs/emit_otlp/1.5.0/emit_otlp/index.html#customizing-http-headers) for details.
