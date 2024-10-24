/*!
An example of emitting traces to Zipkin.

You can emit traces as OTLP to the OpenTelemetry Collector, and forward from there to Zipkin.
Here's an example Collector configuration that does this:

```yaml
receivers:
  otlp:
    protocols:
      http:
        endpoint: 0.0.0.0:4319

exporters:
  zipkin:
    endpoint: "http://localhost:9411/api/v2/spans"
    format: proto
    default_service_name: emit-sample

service:
  pipelines:
    traces:
      receivers: [otlp]
      exporters: [zipkin]
```
*/

fn main() {
    let rt = emit::setup()
        .emit_to(
            emit_otlp::new()
                .resource(emit::props! {
                    #[emit::key("service.name")]
                    service_name: "emit-sample",
                })
                .traces(emit_otlp::traces_http_proto(
                    "http://localhost:4319/v1/traces",
                ))
                .spawn(),
        )
        .init();

    let _ = add("1", "3");

    rt.blocking_flush(std::time::Duration::from_secs(5));
}

#[emit::span(err: err_as_ref, "add {a} and {b}")]
fn add(a: &str, b: &str) -> Result<String, Error> {
    let a = parse(a)?;
    let b = parse(b)?;

    let r = a + b;

    Ok(format(r))
}

#[emit::span(err: err_as_ref, "parse {n}")]
fn parse(n: &str) -> Result<i32, Error> {
    Ok(n.parse()?)
}

#[emit::span("format {n}")]
fn format(n: i32) -> String {
    n.to_string()
}

type Error = Box<dyn std::error::Error + 'static>;

fn err_as_ref(err: &Error) -> &(dyn std::error::Error + 'static) {
    &**err
}
