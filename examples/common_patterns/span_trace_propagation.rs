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
    let rt = emit::setup().emit_to(emit_term::stdout()).init();

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
            span.complete_with(|evt| {
                emit::error!(evt, "HTTP {method} {path} matched no route");
            });
        }
    }
}

// This is a portable traceparent parser
// You may want to use it, or if you've got another one handy use it instead
//
// We may include this in `emit` itself at some point.
pub mod traceparent {
    pub fn parse(
        traceparent: &str,
    ) -> Result<(emit::TraceId, emit::SpanId), Box<dyn std::error::Error + Send + Sync>> {
        let mut parts = traceparent.split('-');

        let version = parts.next().ok_or("missing version")?;

        let "00" = version else {
            return Err(
                format!("unexpected version {version:?}. Only version '00' is supported").into(),
            );
        };

        let trace_id = parts.next().ok_or("missing trace id")?;
        let span_id = parts.next().ok_or("missing span id")?;
        let flags = parts.next().ok_or("missing flags")?;

        let None = parts.next() else {
            return Err(format!("traceparent {traceparent:?} is in an invalid format").into());
        };

        let ("00" | "01") = flags else {
            return Err(format!("unexpected flags {flags:?}").into());
        };

        let trace_id = trace_id.parse()?;
        let span_id = span_id.parse()?;

        Ok((trace_id, span_id))
    }

    pub fn format(
        trace_id: Option<emit::TraceId>,
        span_id: Option<emit::SpanId>,
    ) -> Option<String> {
        let (Some(trace_id), Some(span_id)) = (trace_id, span_id) else {
            return None;
        };

        Some(format!("00-{trace_id}-{span_id}-01"))
    }
}

pub mod http {
    use emit::Props;
    use std::collections::HashMap;

    #[derive(serde::Serialize)]
    pub struct HttpRequest {
        pub method: String,
        pub path: String,
        pub headers: HashMap<String, String>,
    }

    pub fn incoming(request: HttpRequest, route: impl Fn(&str, &str)) {
        // Pulling the traceparent from a HTTP request into the current context

        emit::debug!("Inbound {#[emit::as_serde] request}");

        // 1. Pull the trace and span ids from the incoming traceparent
        let mut trace_id = None;
        let mut span_id = None;

        if let Some((parsed_trace_id, parsed_span_id)) = request
            .headers
            .get("traceparent")
            .and_then(|traceparent| crate::traceparent::parse(traceparent).ok())
        {
            trace_id = Some(parsed_trace_id);
            span_id = Some(parsed_span_id);
        }

        // 2. Push the trace and span ids to the current emit context
        //    This ensures any spans created in the request use the same
        //    trace id, and set their parent span ids appropriately
        emit::Frame::push(
            emit::ctxt(),
            emit::props! {
                trace_id,
                span_id,
            },
        )
        .call(|| {
            // 3. Handle your request within the frame
            route(&request.method, &request.path)
        });
    }

    pub fn outgoing(mut request: HttpRequest) {
        // Adding the traceparent from the current context onto a HTTP request

        // 1. Pull the trace and span ids from the current context
        let (trace_id, span_id) = emit::Frame::current(emit::ctxt()).with(|current| {
            (
                current.pull(emit::well_known::KEY_TRACE_ID),
                current.pull(emit::well_known::KEY_SPAN_ID),
            )
        });

        // 2. Format them as a traceparent header
        if let Some(traceparent) = crate::traceparent::format(trace_id, span_id) {
            // 3. Add the traceparent to the outgoing request
            request.headers.insert("traceparent".into(), traceparent);
        }

        emit::debug!("Outbound {#[emit::as_serde] request}");
    }
}
