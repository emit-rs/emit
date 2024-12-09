/*
An example of how to pull a W3C traceparent header from an incoming HTTP request and propagate it to outgoing HTTP requests.

This example doesn't use any specific web frameworks, so it stubs out a few bits. The key pieces are:

- The `traceparent` module. This implements a simple parser and formatter for the traceparent header.
- The `http::incoming` function. This demonstrates pulling a traceparent off an incoming HTTP request.
- The `http::outgoing` function. This demonstrates pulling a traceparent off the current `emit` context and adding it to an outgoing request.

Applications using the OpenTelemetry SDK should use its propagation mechanisms instead of this approach.
*/

use std::{collections::HashMap, time::Duration};

fn main() {
    // 1. Setup using `emit_traceparent` instead of `emit`
    let rt = emit_traceparent::setup()
        .emit_to(emit_term::stdout())
        .init();

    // A sampled request
    http::incoming(
        http::HttpRequest {
            method: "GET".into(),
            path: "/api/route-1".into(),
            headers: {
                let mut map = HashMap::new();
                map.insert(
                    "traceparent".into(),
                    "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01".into(),
                );
                map
            },
        },
        routes,
    );

    // An unsampled request
    http::incoming(
        http::HttpRequest {
            method: "GET".into(),
            path: "/api/route-1".into(),
            headers: {
                let mut map = HashMap::new();
                map.insert(
                    "traceparent".into(),
                    // Try changing the last digit to 0
                    // This will cause the trace to be unsampled
                    "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-00".into(),
                );
                map
            },
        },
        routes,
    );

    rt.blocking_flush(Duration::from_secs(5));
}

#[emit::span("API Route 1")]
fn api_route_1() {
    http::outgoing(http::HttpRequest {
        method: "GET".into(),
        path: "/somewhere".into(),
        headers: Default::default(),
    });
}

#[emit::span(guard: span, "HTTP {method} {path}", method, path)]
fn routes(method: &str, path: &str) {
    match path {
        "/api/route-1" => api_route_1(),
        _ => {
            span.complete_with(emit::span::completion::from_fn(|evt| {
                emit::error!(evt, "HTTP {method} {path} matched no route");
            }));
        }
    }
}

pub mod http {
    use std::collections::HashMap;

    #[derive(serde::Serialize)]
    pub struct HttpRequest {
        pub method: String,
        pub path: String,
        pub headers: HashMap<String, String>,
    }

    pub fn incoming(request: HttpRequest, route: impl Fn(&str, &str)) {
        emit::debug!("Inbound {#[emit::as_serde] request}");

        // 1. Pull the incoming traceparent
        //    If the request doesn't specify one then use an empty sampled context
        let traceparent = request
            .headers
            .get("traceparent")
            .and_then(|traceparent| emit_traceparent::Traceparent::try_from_str(traceparent).ok())
            .unwrap_or_else(|| emit_traceparent::Traceparent::current());

        // 2. Push the traceparent onto the context
        traceparent.push().call(move || {
            // 3. Handle your request within the frame
            route(&request.method, &request.path)
        })
    }

    pub fn outgoing(mut request: HttpRequest) {
        // 1. Get the current traceparent
        let traceparent = emit_traceparent::Traceparent::current();

        if traceparent.is_valid() {
            // 2. Add the traceparent to the outgoing request
            request
                .headers
                .insert("traceparent".into(), traceparent.to_string());
        }

        emit::debug!("Outbound {#[emit::as_serde] request}");
    }
}
