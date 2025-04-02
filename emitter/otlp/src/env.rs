/*!
Support configuration via environment variables.

The set of variables is defined by the [OpenTelemetry spec](https://opentelemetry.io/docs/languages/sdk-configuration/otlp-exporter/).

If a variable is missing or invalid, then its default value is used.

All signal-specific variables override generic ones, except for headers, where the results are merged.
Header values defined in a signal-specific value override values defined in the generic one.
*/

use std::{borrow::Cow, collections::HashMap};

use sval_derive::Value;

use crate::{baggage, Error};

const OTEL_EXPORTER_OTLP_PROTOCOL: &'static str = "OTEL_EXPORTER_OTLP_PROTOCOL";
const OTEL_EXPORTER_OTLP_TRACES_PROTOCOL: &'static str = "OTEL_EXPORTER_OTLP_TRACES_PROTOCOL";
const OTEL_EXPORTER_OTLP_METRICS_PROTOCOL: &'static str = "OTEL_EXPORTER_OTLP_METRICS_PROTOCOL";
const OTEL_EXPORTER_OTLP_LOGS_PROTOCOL: &'static str = "OTEL_EXPORTER_OTLP_LOGS_PROTOCOL";

const OTEL_EXPORTER_OTLP_ENDPOINT: &'static str = "OTEL_EXPORTER_OTLP_ENDPOINT";
const OTEL_EXPORTER_OTLP_TRACES_ENDPOINT: &'static str = "OTEL_EXPORTER_OTLP_TRACES_ENDPOINT";
const OTEL_EXPORTER_OTLP_METRICS_ENDPOINT: &'static str = "OTEL_EXPORTER_OTLP_METRICS_ENDPOINT";
const OTEL_EXPORTER_OTLP_LOGS_ENDPOINT: &'static str = "OTEL_EXPORTER_OTLP_LOGS_ENDPOINT";

const OTEL_EXPORTER_OTLP_HEADERS: &'static str = "OTEL_EXPORTER_OTLP_HEADERS";
const OTEL_EXPORTER_OTLP_TRACES_HEADERS: &'static str = "OTEL_EXPORTER_OTLP_TRACES_HEADERS";
const OTEL_EXPORTER_OTLP_METRICS_HEADERS: &'static str = "OTEL_EXPORTER_OTLP_METRICS_HEADERS";
const OTEL_EXPORTER_OTLP_LOGS_HEADERS: &'static str = "OTEL_EXPORTER_OTLP_LOGS_HEADERS";

const OTEL_EXPORTER_OTLP_TIMEOUT: &'static str = "OTEL_EXPORTER_OTLP_TIMEOUT";
const OTEL_EXPORTER_OTLP_TRACES_TIMEOUT: &'static str = "OTEL_EXPORTER_OTLP_TRACES_TIMEOUT";
const OTEL_EXPORTER_OTLP_METRICS_TIMEOUT: &'static str = "OTEL_EXPORTER_OTLP_METRICS_TIMEOUT";
const OTEL_EXPORTER_OTLP_LOGS_TIMEOUT: &'static str = "OTEL_EXPORTER_OTLP_LOGS_TIMEOUT";

#[derive(Value, Default)]
struct OtlpSignal {
    protocol: Option<OtlpProtocol>,
    endpoint: Option<String>,
    headers: HashMap<String, Vec<String>>,
    timeout: Option<u64>,
}

#[derive(Value, Clone, Copy)]
enum OtlpProtocol {
    #[sval(label = "grpc")]
    Grpc,
    #[sval(label = "http/protobuf")]
    HttpProtobuf,
    #[sval(label = "http/json")]
    HttpJson,
}

impl Default for OtlpProtocol {
    fn default() -> Self {
        OtlpProtocol::Grpc
    }
}

#[derive(Value, Default)]
struct OtlpConfiguration {
    base: OtlpSignal,
    logs: OtlpSignal,
    traces: OtlpSignal,
    metrics: OtlpSignal,
}

impl OtlpSignal {
    fn protocol(&self, base: &Self) -> OtlpProtocol {
        self.protocol.or(base.protocol).unwrap_or_default()
    }

    fn endpoint<'a>(&'a self, base: &'a Self, http_subpath: &str) -> Cow<'a, str> {
        if let Some(endpoint) = self.endpoint.as_ref().or_else(|| base.endpoint.as_ref()) {
            return Cow::Borrowed(endpoint);
        }

        match self.protocol(base) {
            OtlpProtocol::Grpc => {
                Cow::Borrowed(base.endpoint.as_deref().unwrap_or("http://localhost:4317"))
            }
            OtlpProtocol::HttpJson | OtlpProtocol::HttpProtobuf => {
                let mut endpoint = base
                    .endpoint
                    .as_deref()
                    .unwrap_or("http://localhost:4318")
                    .to_string();

                crate::push_path(&mut endpoint, http_subpath);

                Cow::Owned(endpoint)
            }
        }
    }

    fn headers<'a>(&'a self, base: &'a Self) -> impl Iterator<Item = (&'a str, &'a str)> + 'a {
        base.headers
            .iter()
            .filter(|&(k, _)| !self.headers.contains_key(k))
            .chain(self.headers.iter())
            .flat_map(|(k, v)| v.iter().map(|v| (&**k, &**v)))
    }

    fn timeout(&self, base: &Self) -> u64 {
        self.timeout.or(base.timeout).unwrap_or(10_000)
    }
}

impl OtlpConfiguration {
    fn from_env<K: AsRef<str>, V: AsRef<str>>(
        env: impl Iterator<Item = (K, V)>,
    ) -> OtlpConfiguration {
        let mut config = OtlpConfiguration::default();

        for (k, v) in env {
            let k = k.as_ref();

            // Protocol

            if k.eq_ignore_ascii_case(OTEL_EXPORTER_OTLP_PROTOCOL) {
                config.base.protocol = read_protocol(v.as_ref());
                continue;
            }

            if k.eq_ignore_ascii_case(OTEL_EXPORTER_OTLP_LOGS_PROTOCOL) {
                config.logs.protocol = read_protocol(v.as_ref());
                continue;
            }

            if k.eq_ignore_ascii_case(OTEL_EXPORTER_OTLP_TRACES_PROTOCOL) {
                config.traces.protocol = read_protocol(v.as_ref());
                continue;
            }

            if k.eq_ignore_ascii_case(OTEL_EXPORTER_OTLP_METRICS_PROTOCOL) {
                config.metrics.protocol = read_protocol(v.as_ref());
                continue;
            }

            // Endpoint

            if k.eq_ignore_ascii_case(OTEL_EXPORTER_OTLP_ENDPOINT) {
                config.base.endpoint = read_endpoint(v.as_ref());
                continue;
            }

            if k.eq_ignore_ascii_case(OTEL_EXPORTER_OTLP_LOGS_ENDPOINT) {
                config.logs.endpoint = read_endpoint(v.as_ref());
                continue;
            }

            if k.eq_ignore_ascii_case(OTEL_EXPORTER_OTLP_TRACES_ENDPOINT) {
                config.traces.endpoint = read_endpoint(v.as_ref());
                continue;
            }

            if k.eq_ignore_ascii_case(OTEL_EXPORTER_OTLP_METRICS_ENDPOINT) {
                config.metrics.endpoint = read_endpoint(v.as_ref());
                continue;
            }

            // Headers

            if k.eq_ignore_ascii_case(OTEL_EXPORTER_OTLP_HEADERS) {
                config.base.headers = read_headers(v.as_ref());
                continue;
            }

            if k.eq_ignore_ascii_case(OTEL_EXPORTER_OTLP_LOGS_HEADERS) {
                config.logs.headers = read_headers(v.as_ref());
                continue;
            }

            if k.eq_ignore_ascii_case(OTEL_EXPORTER_OTLP_TRACES_HEADERS) {
                config.traces.headers = read_headers(v.as_ref());
                continue;
            }

            if k.eq_ignore_ascii_case(OTEL_EXPORTER_OTLP_METRICS_HEADERS) {
                config.metrics.headers = read_headers(v.as_ref());
                continue;
            }

            // Timeout

            if k.eq_ignore_ascii_case(OTEL_EXPORTER_OTLP_TIMEOUT) {
                config.base.timeout = read_timeout(v.as_ref());
                continue;
            }

            if k.eq_ignore_ascii_case(OTEL_EXPORTER_OTLP_LOGS_TIMEOUT) {
                config.logs.timeout = read_timeout(v.as_ref());
                continue;
            }

            if k.eq_ignore_ascii_case(OTEL_EXPORTER_OTLP_TRACES_TIMEOUT) {
                config.traces.timeout = read_timeout(v.as_ref());
                continue;
            }

            if k.eq_ignore_ascii_case(OTEL_EXPORTER_OTLP_METRICS_TIMEOUT) {
                config.metrics.timeout = read_timeout(v.as_ref());
                continue;
            }
        }

        config
    }
}

fn read_endpoint(v: &str) -> Option<String> {
    let v = trim(v);

    Some(v.to_owned())
}

fn read_protocol(v: &str) -> Option<OtlpProtocol> {
    let v = trim(v);

    if v.eq_ignore_ascii_case("grpc") {
        return Some(OtlpProtocol::Grpc);
    }

    if v.eq_ignore_ascii_case("http/protobuf") {
        return Some(OtlpProtocol::HttpProtobuf);
    }

    if v.eq_ignore_ascii_case("http/json") {
        return Some(OtlpProtocol::HttpJson);
    }

    let err = Error::msg(format!("{v} is not a valid protocol"));

    emit::warn!(rt: emit::runtime::internal(), "failed to parse protocol: {err}");

    None
}

fn read_headers(v: &str) -> HashMap<String, Vec<String>> {
    let v = trim(v);

    let mut headers = HashMap::<String, Vec<String>>::new();

    match baggage::parse(v) {
        Ok(baggage) => {
            for (k, v) in baggage {
                let v = match v {
                    baggage::Value::Borrowed(v) => v.to_owned(),
                    baggage::Value::Owned(v) => v,
                    baggage::Value::List(_) => {
                        emit::warn!(rt: emit::runtime::internal(), "ignoring list-valued property {header: k}");

                        continue;
                    }
                };

                headers.entry(k.to_owned()).or_default().push(v);
            }
        }
        Err(err) => {
            emit::warn!(rt: emit::runtime::internal(), "failed to parse HTTP headers: {err}");
        }
    }

    headers
}

fn read_timeout(v: &str) -> Option<u64> {
    let v = trim(v);

    match v.parse() {
        Ok(timeout) => Some(timeout),
        Err(err) => {
            emit::warn!(rt: emit::runtime::internal(), "failed to parse timeout: {err}");

            None
        }
    }
}

fn trim(v: &str) -> &str {
    v.trim()
}
