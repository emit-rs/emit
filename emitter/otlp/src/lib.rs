/*!
Emit diagnostic events via the OpenTelemetry Protocol (OTLP).

This library provides [`Otlp`], an [`emit::Emitter`] that sends export requests directly to some remote OTLP receiver. If you need to integrate [`emit`] with the OpenTelemetry SDK, see [`emit-opentelemetry`](https://docs.rs/emit_opentelemetry/0.11.0-alpha.16/emit_opentelemetry/index.html).

# How it works

```text
┌────────────────────────────────────────┐  ┌─────────────┐    ┌─────────────────────────────┐
│                caller                  │  │   channel   │    │     background worker       │
│                                        │  │             │    │                             │
│ emit::Event─┬─*─►is trace?──►Span──────┼──┼──►Trace─────┼─┐  │ ExportTraceServiceRequest   │
│             │                          │  │             │ │  │                             │
│             ├─*─►is metric?─►Metric────┼──┼──►Metrics───┼─┼──► ExportMetricsServiceRequest │
│             │                          │  │             │ │  │                             │
│             └─*─────────────►LogRecord─┼──┼──►Logs──────┼─┘  │ ExportLogsServiceRequest    │
└────────────────────────────────────────┘  └─────────────┘    └─────────────────────────────┘
 * Only if the logs/trace/metrics signal is configured
```

The emitter is based on an asynchronous, batching channel. A diagnostic event makes its way from [`emit::emit!`] through to the remote OTLP receiver in the following key steps:

1. Determine what kind of signal the event belongs to:
    - If the event carries [`emit::Kind::Span`], and the trace signal is configured, then treat it as a span.
    - If the event carries [`emit::Kind::Metric`], and the metrics signal is configured, then treat it as a metric.
    - In any other case, if the logs signal is configured, then treat it as a log record.
2. Serialize the event into the OTLP datastructure in the target format (JSON/protobuf).
3. Put the serialized event into a channel. Each signal has its own internal queue in the channel.
4. On a background worker, process the events in the channel by forming them up into OTLP export requests and sending them using the target protocol (HTTP/gRPC).

This library is based on `hyper` with `tokio` for HTTP, and `rustls` with `ring` for TLS. These dependencies are not configurable and can't be swapped for alternative implementations.

# Getting started

Add `emit` and `emit_otlp` to your `Cargo.toml`:

```toml
[dependencies.emit]
version = "0.11.0-alpha.16"

[dependencies.emit_otlp]
version = "0.11.0-alpha.16"
```

Initialize `emit` at the start of your `main.rs` using an OTLP emitter:

```
fn main() {
    let rt = emit::setup()
        .emit_to(emit_otlp::new()
            // Add required resource properties for OTLP
            .resource(emit::props! {
                #[emit::key("service.name")]
                service_name: emit::pkg!(),
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

The [`new`] method returns an [`OtlpBuilder`], which can be configured with endpoints for the desired signals through its [`OtlpBuilder::logs`], [`OtlpBuilder::traces`], and [`OtlpBuilder::metrics`] methods.

You don't need to configure all signals, but you should at least configure [`OtlpBuilder::logs`].

Once the builder is configured, call [`OtlpBuilder::spawn`] and pass the resulting [`Otlp`] to [`emit::Setup::emit_to`].

# Where the background worker is spawned

The [`Otlp`] emitter doesn't do any work directly. That's all handled by a background worker created through [`OtlpBuilder::spawn`]. The worker will spawn on a background thread with a single-threaded `tokio` executor on it.

## Configuring for gRPC+protobuf

The [`logs_grpc_proto`], [`traces_grpc_proto`], and [`metrics_grpc_proto`] functions produce builders for gRPC+protobuf:

```
# fn build() -> emit_otlp::Otlp {
emit_otlp::new()
    .resource(emit::props! {
        #[emit::key("service.name")]
        service_name: emit::pkg!(),
    })
    .logs(emit_otlp::logs_grpc_proto("http://localhost:4319"))
    .traces(emit_otlp::traces_grpc_proto("http://localhost:4319"))
    .metrics(emit_otlp::metrics_grpc_proto("http://localhost:4319"))
    .spawn()
# }
```

gRPC is based on HTTP and internally uses well-known URI paths to route RPC requests. These paths are appended automatically to the endpoint, so you don't need to specify them during configuration.

# Configuring for HTTP+JSON

The [`logs_http_json`], [`traces_http_json`], and [`metrics_http_json`] functions produce builders for HTTP+JSON:

```
# fn build() -> emit_otlp::Otlp {
emit_otlp::new()
    .resource(emit::props! {
        #[emit::key("service.name")]
        service_name: emit::pkg!(),
    })
    .logs(emit_otlp::logs_http_json("http://localhost:4318/v1/logs"))
    .traces(emit_otlp::traces_http_json("http://localhost:4318/v1/traces"))
    .metrics(emit_otlp::metrics_http_json("http://localhost:4318/v1/metrics"))
    .spawn()
# }
```

# Configuring for HTTP+protobuf

The [`logs_http_proto`], [`traces_http_proto`], and [`metrics_http_proto`] functions produce builders for HTTP+protobuf:

```
# fn build() -> emit_otlp::Otlp {
emit_otlp::new()
    .resource(emit::props! {
        #[emit::key("service.name")]
        service_name: emit::pkg!(),
    })
    .logs(emit_otlp::logs_http_proto("http://localhost:4318/v1/logs"))
    .traces(emit_otlp::traces_http_proto("http://localhost:4318/v1/traces"))
    .metrics(emit_otlp::metrics_http_proto("http://localhost:4318/v1/metrics"))
    .spawn()
# }
```

# Configuring TLS

If the `tls` Cargo feature is enabled, and the scheme of your endpoint uses the `https://` scheme then it will use TLS from `rustls` and `rustls-native-certs`.

# Configuring compression

If the `gzip` Cargo feature is enabled then gzip compression will be applied automatically to all export requests.

You can disable any compression through an [`OtlpTransportBuilder`]:

```
# fn build() -> emit_otlp::OtlpBuilder {
emit_otlp::new()
   .logs(emit_otlp::logs_proto(emit_otlp::http("http://localhost:4318/v1/logs")
      .allow_compression(false))
   )
# }
```

# Customizing HTTP headers

You can specify custom headers to be used for HTTP or gRPC requests through an [`OtlpTransportBuilder`]:

```
# fn build() -> emit_otlp::OtlpBuilder {
emit_otlp::new()
   .logs(emit_otlp::logs_proto(emit_otlp::http("http://localhost:4318/v1/logs")
      .headers([
         ("X-ApiKey", "abcd"),
      ]))
   )
# }
```

# Configuring a resource

The [`OtlpBuilder::resource`] method configures the OTLP resource to send with each export request. Some OTLP receivers accept data without a resource but the OpenTelemetry specification itself mandates it.

At a minimum, you should add the `service.name` property:

```
# fn build() -> emit_otlp::OtlpBuilder {
emit_otlp::new()
    .resource(emit::props! {
        #[emit::key("service.name")]
        service_name: emit::pkg!(),
    })
# }
```

You should also consider setting other well-known resource properties:

```
# fn build() -> emit_otlp::OtlpBuilder {
emit_otlp::new()
    .resource(emit::props! {
        #[emit::key("service.name")]
        service_name: emit::pkg!(),
        #[emit::key("telemetry.sdk.language")]
        language: emit_otlp::telemetry_sdk_language(),
        #[emit::key("telemetry.sdk.name")]
        sdk: emit_otlp::telemetry_sdk_name(),
        #[emit::key("telemetry.sdk.version")]
        version: emit_otlp::telemetry_sdk_version(),
    })
# }
```

# Logs

All [`emit::Event`]s can be represented as OTLP log records. You should at least configure the logs signal to make sure all diagnostics are captured in some way. A minimal logging configuration for gRPC+Protobuf is:

```
# fn build() -> emit_otlp::Otlp {
emit_otlp::new()
    .resource(emit::props! {
        #[emit::key("service.name")]
        service_name: emit::pkg!(),
    })
    .logs(emit_otlp::logs_grpc_proto("http://localhost:4318"))
    .spawn()
# }
```

The following diagnostic:

```
emit::info!("Hello, OTLP!");
```

will produce the following HTTP+JSON export request:

```text
http://localhost:4318/v1/logs
```

```json
{
   "resourceLogs": [
      {
         "resource": {
            "attributes": [
               {
                  "key": "service.name",
                  "value": {
                     "stringValue": "my_app"
                  }
               }
            ]
         },
         "scopeLogs": [
            {
               "scope": {
                  "name": "my_app"
               },
               "logRecords": [
                  {
                     "timeUnixNano": 1716804019165847000,
                     "observedTimeUnixNano": 1716804019165847000,
                     "body": {
                        "stringValue": "Hello, OTLP!"
                     },
                     "attributes": [],
                     "severityNumber": 9,
                     "severityText": "info"
                  }
               ]
            }
         ]
      }
   ]
}
```

When the traces signal is not configured, diagnostic events for spans are represented as regular OTLP log records. The following diagnostic:

```
#[emit::span("Compute {a} + {b}")]
fn add(a: i32, b: i32) -> i32 {
    let r = a + b;

    emit::info!("Produced {r}", r);

    r
}

add(1, 3);
```

will produce the following HTTP+JSON export request:

```text
http://localhost:4318/v1/logs
```

```json
{
   "resourceLogs": [
      {
         "resource": {
            "attributes": [
               {
                  "key": "service.name",
                  "value": {
                     "stringValue": "my_app"
                  }
               }
            ]
         },
         "scopeLogs": [
            {
               "scope": {
                  "name": "my_app"
               },
               "logRecords": [
                  {
                     "timeUnixNano": 1716804240222377000,
                     "observedTimeUnixNano": 1716804240222377000,
                     "body": {
                        "stringValue": "Produced 4"
                     },
                     "attributes": [
                        {
                           "key": "a",
                           "value": {
                              "intValue": 1
                           }
                        },
                        {
                           "key": "b",
                           "value": {
                              "intValue": 3
                           }
                        },
                        {
                           "key": "r",
                           "value": {
                              "intValue": 4
                           }
                        }
                     ],
                     "severityNumber": 9,
                     "severityText": "info",
                     "traceId": "489571cc6b94414ceb4a32ccc2c7df09",
                     "spanId": "a93239061c12aa4c"
                  },
                  {
                     "timeUnixNano": 1716804240222675000,
                     "observedTimeUnixNano": 1716804240222675000,
                     "body": {
                        "stringValue": "Compute 1 + 3"
                     },
                     "attributes": [
                        {
                           "key": "a",
                           "value": {
                              "intValue": 1
                           }
                        },
                        {
                           "key": "b",
                           "value": {
                              "intValue": 3
                           }
                        },
                        {
                           "key": "evt_kind",
                           "value": {
                              "stringValue": "span"
                           }
                        },
                        {
                           "key": "span_name",
                           "value": {
                              "stringValue": "Compute {a} + {b}"
                           }
                        }
                     ],
                     "severityNumber": 9,
                     "severityText": "info",
                     "traceId": "489571cc6b94414ceb4a32ccc2c7df09",
                     "spanId": "a93239061c12aa4c"
                  }
               ]
            }
         ]
      }
   ]
}
```

When the metrics signal is not configured, diagnostic events for metric samples are represented as regular OTLP log records. The following diagnostic:

```
emit::emit!(
    evt: emit::Metric::new(
        emit::mdl!(),
        "my_metric",
        "count",
        emit::Empty,
        42,
        emit::Empty,
    )
);
```

will produce the following HTTP+JSON export request:

```text
http://localhost:4318/v1/logs
```

```json
{
   "resourceLogs": [
      {
         "resource": {
            "attributes": [
               {
                  "key": "service.name",
                  "value": {
                     "stringValue": "my_app"
                  }
               }
            ]
         },
         "scopeLogs": [
            {
               "scope": {
                  "name": "my_app"
               },
               "logRecords": [
                  {
                     "timeUnixNano": 1716876516012074000,
                     "observedTimeUnixNano": 1716876516012074000,
                     "body": {
                        "stringValue": "count of my_metric is 42"
                     },
                     "attributes": [
                        {
                           "key": "evt_kind",
                           "value": {
                              "stringValue": "metric"
                           }
                        },
                        {
                           "key": "metric_agg",
                           "value": {
                              "stringValue": "count"
                           }
                        },
                        {
                           "key": "metric_name",
                           "value": {
                              "stringValue": "my_metric"
                           }
                        },
                        {
                           "key": "metric_value",
                           "value": {
                              "intValue": 42
                           }
                        }
                     ],
                     "severityNumber": 9,
                     "severityText": "info"
                  }
               ]
            }
         ]
      }
   ]
}
```

# Traces

When the traces signal is configured, [`emit::Event`]s can be represented as OTLP spans so long as they satisfy the following conditions:

- They have a valid [`emit::TraceId`] in the [`emit::well_known::KEY_TRACE_ID`] property and [`emit::SpanId`] in the [`emit::well_known::KEY_SPAN_ID`] property.
- Their [`emit::Event::extent`] is a span. That is, [`emit::Extent::is_span`] is `true`.
- They have an [`emit::Kind::Span`] in the [`emit::well_known::KEY_EVT_KIND`] property.

If any condition is not met, the event will be represented as an OTLP log record. If the logs signal is not configured then it will be discarded.

A minimal logging configuration for gRPC+Protobuf is:

```
# fn build() -> emit_otlp::Otlp {
emit_otlp::new()
    .resource(emit::props! {
        #[emit::key("service.name")]
        service_name: emit::pkg!(),
    })
    .traces(emit_otlp::traces_grpc_proto("http://localhost:4318"))
    .logs(emit_otlp::logs_grpc_proto("http://localhost:4318"))
    .spawn()
# }
```

The following diagnostic:

```
#[emit::span("Compute {a} + {b}")]
fn add(a: i32, b: i32) -> i32 {
    let r = a + b;

    emit::info!("Produced {r}", r);

    r
}

add(1, 3);
```

will produce the following HTTP+JSON export requests:

```text
http://localhost:4318/v1/traces
```

```json
{
   "resourceSpans": [
      {
         "resource": {
            "attributes": [
               {
                  "key": "service.name",
                  "value": {
                     "stringValue": "my_app"
                  }
               }
            ]
         },
         "scopeSpans": [
            {
               "scope": {
                  "name": "my_app"
               },
               "spans": [
                  {
                     "name": "Compute {a} + {b}",
                     "kind": 0,
                     "startTimeUnixNano": 1716888416629816000,
                     "endTimeUnixNano": 1716888416630814000,
                     "attributes": [
                        {
                           "key": "a",
                           "value": {
                              "intValue": 1
                           }
                        },
                        {
                           "key": "b",
                           "value": {
                              "intValue": 3
                           }
                        }
                     ],
                     "traceId": "0a85ccaf666e11aaca6bd5d469e2850d",
                     "spanId": "2b9caa35eaefed3a"
                  }
               ]
            }
         ]
      }
   ]
}
```

```text
http://localhost:4318/v1/logs
```

```json
{
   "resourceLogs": [
      {
         "resource": {
            "attributes": [
               {
                  "key": "service.name",
                  "value": {
                     "stringValue": "my_app"
                  }
               }
            ]
         },
         "scopeLogs": [
            {
               "scope": {
                  "name": "my_app"
               },
               "logRecords": [
                  {
                     "timeUnixNano": 1716888416630507000,
                     "observedTimeUnixNano": 1716888416630507000,
                     "body": {
                        "stringValue": "Produced 4"
                     },
                     "attributes": [
                        {
                           "key": "a",
                           "value": {
                              "intValue": 1
                           }
                        },
                        {
                           "key": "b",
                           "value": {
                              "intValue": 3
                           }
                        },
                        {
                           "key": "r",
                           "value": {
                              "intValue": 4
                           }
                        }
                     ],
                     "severityNumber": 9,
                     "severityText": "info",
                     "traceId": "0a85ccaf666e11aaca6bd5d469e2850d",
                     "spanId": "2b9caa35eaefed3a"
                  }
               ]
            }
         ]
      }
   ]
}
```

If the [`emit::well_known::KEY_ERR`] property is set, then the resulting OTLP span will carry the semantic exception event:

```
#[emit::span(guard: span, "Compute {a} + {b}")]
fn add(a: i32, b: i32) -> i32 {
   let r = a + b;

   if r == 4 {
      span.complete_with(|evt| {
            emit::error!(
               evt,
               "Compute {a} + {b} failed",
               a,
               b,
               r,
               err: "Invalid result",
            );
      });
   }

   r
}

add(1, 3);
```

```text
http://localhost:4318/v1/traces
```

```json
{
   "resourceSpans": [
      {
         "resource": {
            "attributes": [
               {
                  "key": "service.name",
                  "value": {
                     "stringValue": "my_app"
                  }
               }
            ]
         },
         "scopeSpans": [
            {
               "scope": {
                  "name": "my_app"
               },
               "spans": [
                  {
                     "name": "Compute {a} + {b}",
                     "kind": 0,
                     "startTimeUnixNano": 1716936430882852000,
                     "endTimeUnixNano": 1716936430883250000,
                     "attributes": [
                        {
                           "key": "a",
                           "value": {
                              "intValue": 1
                           }
                        },
                        {
                           "key": "b",
                           "value": {
                              "intValue": 3
                           }
                        },
                        {
                           "key": "r",
                           "value": {
                              "intValue": 4
                           }
                        }
                     ],
                     "traceId": "6499bc190add060dad8822600ba65226",
                     "spanId": "b72c5152c32cc432",
                     "events": [
                        {
                           "name": "exception",
                           "timeUnixNano": 1716936430883250000,
                           "attributes": [
                              {
                                 "key": "exception.message",
                                 "value": {
                                    "stringValue": "Invalid result"
                                 }
                              }
                           ]
                        }
                     ],
                     "status": {
                        "message": "Invalid result",
                        "code": 2
                     }
                  }
               ]
            }
         ]
      }
   ]
}
```

# Metrics

When the metrics signal is configured, [`emit::Event`]s can be represented as OTLP metrics so long as they satisfy the following conditions:

- They have a [`emit::well_known::KEY_METRIC_AGG`] properties.
- They have a [`emit::well_known::KEY_METRIC_VALUE`] property with a numeric value or sequence of numeric values.
- They have an [`emit::Kind::Metric`] in the [`emit::well_known::KEY_EVT_KIND`] property.

If any condition is not met, the event will be represented as an OTLP log record. If the logs signal is not configured then it will be discarded.

A minimal logging configuration for gRPC+Protobuf is:

```
# fn build() -> emit_otlp::Otlp {
emit_otlp::new()
    .resource(emit::props! {
        #[emit::key("service.name")]
        service_name: emit::pkg!(),
    })
    .metrics(emit_otlp::metrics_grpc_proto("http://localhost:4318"))
    .logs(emit_otlp::logs_grpc_proto("http://localhost:4318"))
    .spawn()
# }
```

If the metric aggregation is `"count"` then the resulting OTLP metric is a monotonic sum:

```
emit::emit!(
    evt: emit::Metric::new(
        emit::mdl!(),
        "my_metric",
        "count",
        emit::Empty,
        42,
        emit::props! {
            a: true
        },
    )
);
```

```text
http://localhost:4318/v1/metrics
```

```json
{
   "resourceMetrics": [
      {
         "resource": {
            "attributes": [
               {
                  "key": "service.name",
                  "value": {
                     "stringValue": "my_app"
                  }
               }
            ]
         },
         "scopeMetrics": [
            {
               "scope": {
                  "name": "my_app"
               },
               "metrics": [
                  {
                     "name": "my_metric",
                     "unit": null,
                     "sum": {
                        "dataPoints": [
                           {
                              "attributes": [
                                 {
                                    "key": "a",
                                    "value": {
                                       "boolValue": true
                                    }
                                 }
                              ],
                              "startTimeUnixNano": 1716889540249854000,
                              "timeUnixNano": 1716889540249854000,
                              "value": 42
                           }
                        ],
                        "aggregationTemporality": 2,
                        "isMonotonic": true
                     }
                  }
               ]
            }
         ]
      }
   ]
}
```

If the metric aggregation is `"sum"` then the resulting OTLP metric is a non-monotonic sum:

```
emit::emit!(
    evt: emit::Metric::new(
        emit::mdl!(),
        "my_metric",
        "sum",
        emit::Empty,
        -8,
        emit::props! {
            a: true
        },
    )
);
```

```text
http://localhost:4318/v1/metrics
```

```json
{
   "resourceMetrics": [
      {
         "resource": {
            "attributes": [
               {
                  "key": "service.name",
                  "value": {
                     "stringValue": "my_app"
                  }
               }
            ]
         },
         "scopeMetrics": [
            {
               "scope": {
                  "name": "my_app"
               },
               "metrics": [
                  {
                     "name": "my_metric",
                     "unit": null,
                     "sum": {
                        "dataPoints": [
                           {
                              "attributes": [
                                 {
                                    "key": "a",
                                    "value": {
                                       "boolValue": true
                                    }
                                 }
                              ],
                              "startTimeUnixNano": 1716889891391075000,
                              "timeUnixNano": 1716889891391075000,
                              "value": -8
                           }
                        ],
                        "aggregationTemporality": 2,
                        "isMonotonic": false
                     }
                  }
               ]
            }
         ]
      }
   ]
}
```

Any other aggregation will be represented as an OTLP gauge:

```
emit::emit!(
    evt: emit::Metric::new(
        emit::mdl!(),
        "my_metric",
        "last",
        emit::Empty,
        615,
        emit::props! {
            a: true
        },
    )
);
```

```text
http://localhost:4318/v1/metrics
```

```json
{
   "resourceMetrics": [
      {
         "resource": {
            "attributes": [
               {
                  "key": "service.name",
                  "value": {
                     "stringValue": "my_app"
                  }
               }
            ]
         },
         "scopeMetrics": [
            {
               "scope": {
                  "name": "my_app"
               },
               "metrics": [
                  {
                     "name": "my_metric",
                     "unit": null,
                     "gauge": {
                        "dataPoints": [
                           {
                              "attributes": [
                                 {
                                    "key": "a",
                                    "value": {
                                       "boolValue": true
                                    }
                                 }
                              ],
                              "startTimeUnixNano": 1716890230856380000,
                              "timeUnixNano": 1716890230856380000,
                              "value": 615
                           }
                        ]
                     }
                  }
               ]
            }
         ]
      }
   ]
}
```

If the metric aggregation is `"count"` or `"sum"`, and value is a sequence, then each value will be summed to produce a single data point:

```
let start = emit::Timestamp::from_unix(std::time::Duration::from_secs(1716890420));
let end = emit::Timestamp::from_unix(std::time::Duration::from_secs(1716890425));

emit::emit!(
    evt: emit::Metric::new(
        emit::mdl!(),
        "my_metric",
        "count",
        start..end,
        &[
            1.0,
            1.0,
            1.0,
            1.0,
            1.0,
        ],
        emit::props! {
            a: true
        },
    )
);
```

```text
http://localhost:4318/v1/metrics
```

```json
{
   "resourceMetrics": [
      {
         "resource": {
            "attributes": [
               {
                  "key": "service.name",
                  "value": {
                     "stringValue": "my_app"
                  }
               }
            ]
         },
         "scopeMetrics": [
            {
               "scope": {
                  "name": "my_app"
               },
               "metrics": [
                  {
                     "name": "my_metric",
                     "unit": null,
                     "sum": {
                        "dataPoints": [
                           {
                              "attributes": [
                                 {
                                    "key": "a",
                                    "value": {
                                       "boolValue": true
                                    }
                                 }
                              ],
                              "startTimeUnixNano": 1716890420000000000,
                              "timeUnixNano": 1716890425000000000,
                              "value": 5
                           }
                        ],
                        "aggregationTemporality": 1,
                        "isMonotonic": true
                     }
                  }
               ]
            }
         ]
      }
   ]
}
```

# Limitations

This library is not an alternative to the OpenTelemetry SDK. It's specifically targeted at emitting diagnostic events to OTLP-compatible services. It has some intentional limitations:

- **No propagation.** This is the responsibility of the application to manage.
- **No histogram metrics.** `emit`'s data model for metrics is simplistic compared to OpenTelemetry's, so it doesn't support histograms or exponential histograms.
- **No span events.** Only the conventional exception event is supported. Standalone log events are not converted into span events. They're sent via the logs endpoint instead.
- **No tracestate.** `emit`'s data model for spans doesn't include the W3C tracestate.

# Troubleshooting

If you're not seeing diagnostics appear in your OTLP receiver, you can rule out configuration issues in `emit_otlp` by configuring `emit`'s internal logger, and collect metrics from it:

```
# mod emit_term {
#     pub fn stdout() -> impl emit::runtime::InternalEmitter + Send + Sync + 'static {
#        emit::runtime::AssertInternal(emit::emitter::from_fn(|_| {}))
#     }
# }
use emit::metric::Source;

fn main() {
    // 1. Initialize the internal logger
    //    Diagnostics produced by `emit_otlp` itself will go here
    let internal = emit::setup()
        .emit_to(emit_term::stdout())
        .init_internal();

    let mut reporter = emit::metric::Reporter::new();

    let rt = emit::setup()
        .emit_to({
            let otlp = emit_otlp::new()
                .resource(emit::props! {
                    #[emit::key("service.name")]
                    service_name: emit::pkg!(),
                })
                .logs(emit_otlp::logs_grpc_proto("http://localhost:4319"))
                .traces(emit_otlp::traces_grpc_proto("http://localhost:4319"))
                .metrics(emit_otlp::metrics_grpc_proto("http://localhost:4319"))
                .spawn();

            // 2. Add `emit_otlp`'s metrics to a reporter so we can see what it's up to
            //    You can do this independently of the internal emitter
            reporter.add_source(otlp.metric_source());

            otlp
        })
        .init();

    // Your app code goes here

    rt.blocking_flush(std::time::Duration::from_secs(30));

    // 3. Report metrics after attempting to flush
    //    You could also do this periodically as your application runs
    reporter.emit_metrics(&internal.emitter());
}
```

Diagnostics include when batches are emitted, and any failures observed along the way.
*/

#![doc(html_logo_url = "https://raw.githubusercontent.com/emit-rs/emit/main/asset/logo.svg")]
#![deny(missing_docs)]

#[macro_use]
mod internal_metrics;
mod client;
mod data;
mod error;

pub use self::{client::*, error::*, internal_metrics::*};

/**
A value to use as `telemetry.sdk.name` in [`OtlpBuilder::resource`].
*/
pub const fn telemetry_sdk_name() -> &'static str {
    env!("CARGO_PKG_NAME")
}

/**
A value to use as `telemetry.sdk.version` in [`OtlpBuilder::resource`].
*/
pub const fn telemetry_sdk_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/**
A value to use as `telemetry.sdk.language` in [`OtlpBuilder::resource`].
*/
pub const fn telemetry_sdk_language() -> &'static str {
    "rust"
}

/**
Start a builder for an [`Otlp`] emitter.

Signals can be configured on the builder through [`OtlpBuilder::logs`], [`OtlpBuilder::traces`], and [`OtlpBuilder::metrics`].

Once the builder is configured, call [`OtlpBuilder::spawn`] to complete it, passing the resulting [`Otlp`] to [`emit::Setup::emit_to`].

See the crate root documentation for more details.
*/
pub fn new() -> OtlpBuilder {
    OtlpBuilder::new()
}

/**
Get a transport builder for gRPC.

The builder can be used by [`OtlpLogsBuilder`], [`OtlpTracesBuilder`], and [`OtlpMetricsBuilder`] to configure a signal to send OTLP via gRPC.
*/
pub fn grpc(dst: impl Into<String>) -> OtlpTransportBuilder {
    OtlpTransportBuilder::grpc(dst)
}

/**
Get a transport builder for HTTP.

The builder can be used by [`OtlpLogsBuilder`], [`OtlpTracesBuilder`], and [`OtlpMetricsBuilder`] to configure a signal to send OTLP via HTTP.

The `dst` argument should include the complete path to the OTLP endpoint for the given signal, like:

- `http://localhost:4318/v1/logs` for the logs signal.
- `http://localhost:4318/v1/traces` for the traces signal.
- `http://localhost:4318/v1/metrics` for the metrics signal.
*/
pub fn http(dst: impl Into<String>) -> OtlpTransportBuilder {
    OtlpTransportBuilder::http(dst)
}

/**
Get a logs signal builder for gRPC+protobuf.

The `dst` argument should include just the root of the target gRPC service, like `http://localhost:4319`.
*/
pub fn logs_grpc_proto(dst: impl Into<String>) -> OtlpLogsBuilder {
    OtlpLogsBuilder::grpc_proto(dst)
}

/**
Get a logs signal builder for HTTP+protobuf.

The `dst` argument should include the complete path to the OTLP endpoint for the given signal, like `http://localhost:4318/v1/logs`.
*/
pub fn logs_http_proto(dst: impl Into<String>) -> OtlpLogsBuilder {
    OtlpLogsBuilder::http_proto(dst)
}

/**
Get a logs signal builder for HTTP+JSON.

The `dst` argument should include the complete path to the OTLP endpoint for the given signal, like `http://localhost:4318/v1/logs`.
*/
pub fn logs_http_json(dst: impl Into<String>) -> OtlpLogsBuilder {
    OtlpLogsBuilder::http_json(dst)
}

/**
Get a logs signal builder for the given transport with protobuf encoding.
*/
pub fn logs_proto(transport: OtlpTransportBuilder) -> OtlpLogsBuilder {
    OtlpLogsBuilder::proto(transport)
}

/**
Get a logs signal builder for the given transport with JSON encoding.
*/
pub fn logs_json(transport: OtlpTransportBuilder) -> OtlpLogsBuilder {
    OtlpLogsBuilder::json(transport)
}

/**
Get a traces signal builder for gRPC+protobuf.

The `dst` argument should include just the root of the target gRPC service, like `http://localhost:4319`.
*/
pub fn traces_grpc_proto(dst: impl Into<String>) -> OtlpTracesBuilder {
    OtlpTracesBuilder::grpc_proto(dst)
}

/**
Get a traces signal builder for HTTP+protobuf.

The `dst` argument should include the complete path to the OTLP endpoint for the given signal, like `http://localhost:4318/v1/traces`.
*/
pub fn traces_http_proto(dst: impl Into<String>) -> OtlpTracesBuilder {
    OtlpTracesBuilder::http_proto(dst)
}

/**
Get a traces signal builder for HTTP+JSON.

The `dst` argument should include the complete path to the OTLP endpoint for the given signal, like `http://localhost:4318/v1/traces`.
*/
pub fn traces_http_json(dst: impl Into<String>) -> OtlpTracesBuilder {
    OtlpTracesBuilder::http_json(dst)
}

/**
Get a traces signal builder for the given transport with protobuf encoding.
*/
pub fn traces_proto(transport: OtlpTransportBuilder) -> OtlpTracesBuilder {
    OtlpTracesBuilder::proto(transport)
}

/**
Get a traces signal builder for the given transport with JSON encoding.
*/
pub fn traces_json(transport: OtlpTransportBuilder) -> OtlpTracesBuilder {
    OtlpTracesBuilder::json(transport)
}

/**
Get a metrics signal builder for gRPC+protobuf.

The `dst` argument should include just the root of the target gRPC service, like `http://localhost:4319`.
*/
pub fn metrics_grpc_proto(dst: impl Into<String>) -> OtlpMetricsBuilder {
    OtlpMetricsBuilder::grpc_proto(dst)
}

/**
Get a metrics signal builder for HTTP+protobuf.

The `dst` argument should include the complete path to the OTLP endpoint for the given signal, like `http://localhost:4318/v1/metrics`.
*/
pub fn metrics_http_proto(dst: impl Into<String>) -> OtlpMetricsBuilder {
    OtlpMetricsBuilder::http_proto(dst)
}

/**
Get a metrics signal builder for HTTP+JSON.

The `dst` argument should include the complete path to the OTLP endpoint for the given signal, like `http://localhost:4318/v1/metrics`.
*/
pub fn metrics_http_json(dst: impl Into<String>) -> OtlpMetricsBuilder {
    OtlpMetricsBuilder::http_json(dst)
}

/**
Get a metrics signal builder for the given transport with protobuf encoding.
*/
pub fn metrics_proto(transport: OtlpTransportBuilder) -> OtlpMetricsBuilder {
    OtlpMetricsBuilder::proto(transport)
}

/**
Get a metrics signal builder for the given transport with JSON encoding.
*/
pub fn metrics_json(transport: OtlpTransportBuilder) -> OtlpMetricsBuilder {
    OtlpMetricsBuilder::json(transport)
}

#[cfg(test)]
pub(crate) mod util {
    use std::time::Duration;

    pub(crate) fn ts(unix_time: u64) -> emit::Timestamp {
        emit::Timestamp::from_unix(Duration::from_secs(unix_time)).unwrap()
    }
}
