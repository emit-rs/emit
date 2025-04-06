/*!
Support configuration via environment variables.

The set of variables is defined by the [OpenTelemetry spec](https://opentelemetry.io/docs/languages/sdk-configuration/otlp-exporter/).

If a variable is missing or invalid, then its default value is used.

All signal-specific variables override generic ones, except for headers, where the results are merged.
Header values defined in a signal-specific value override values defined in the generic one.
*/

use std::{borrow::Cow, collections::HashMap, ops::ControlFlow, str::FromStr};

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

const OTEL_SERVICE_NAME: &'static str = "OTEL_SERVICE_NAME";

const OTEL_RESOURCE_ATTRIBUTES: &'static str = "OTEL_RESOURCE_ATTRIBUTES";

#[derive(Value, Default)]
struct OtlpConfig {
    base: SignalConfig,
    logs: SignalConfig,
    traces: SignalConfig,
    metrics: SignalConfig,
    resource: HashMap<emit::Str<'static>, emit::value::OwnedValue>,
    service_name: Option<String>,
}

impl OtlpConfig {
    fn from_env<K: AsRef<str>, V: AsRef<str>>(env: impl Iterator<Item = (K, V)>) -> OtlpConfig {
        fn endpoint(v: &str) -> Option<String> {
            let v = trim(v);

            Some(v.to_owned())
        }

        fn protocol(v: &str) -> Option<ProtocolConfig> {
            let v = trim(v);

            if v.eq_ignore_ascii_case("grpc") {
                return Some(ProtocolConfig::Grpc);
            }

            if v.eq_ignore_ascii_case("http/protobuf") {
                return Some(ProtocolConfig::HttpProtobuf);
            }

            if v.eq_ignore_ascii_case("http/json") {
                return Some(ProtocolConfig::HttpJson);
            }

            let err = Error::msg(format!("{v} is not a valid protocol"));

            emit::warn!(rt: emit::runtime::internal(), "failed to parse protocol: {err}");

            None
        }

        fn headers(v: &str) -> HashMap<String, Vec<String>> {
            let v = trim(v);

            let mut headers = HashMap::<String, Vec<String>>::new();

            match baggage::parse(v) {
                Ok(baggage) => {
                    for (k, v) in baggage {
                        let v = match v {
                            baggage::Value::Single(v) => v.into_owned(),
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

        fn timeout(v: &str) -> Option<u64> {
            let v = trim(v);

            match v.parse() {
                Ok(timeout) => Some(timeout),
                Err(err) => {
                    emit::warn!(rt: emit::runtime::internal(), "failed to parse timeout: {err}");

                    None
                }
            }
        }

        fn service_name(v: &str) -> Option<String> {
            let v = trim(v);

            Some(v.to_owned())
        }

        let mut config = OtlpConfig::default();

        for (k, v) in env {
            let k = k.as_ref();

            // Protocol

            if k.eq_ignore_ascii_case(OTEL_EXPORTER_OTLP_PROTOCOL) {
                config.base.protocol = protocol(v.as_ref());
                continue;
            }

            if k.eq_ignore_ascii_case(OTEL_EXPORTER_OTLP_LOGS_PROTOCOL) {
                config.logs.protocol = protocol(v.as_ref());
                continue;
            }

            if k.eq_ignore_ascii_case(OTEL_EXPORTER_OTLP_TRACES_PROTOCOL) {
                config.traces.protocol = protocol(v.as_ref());
                continue;
            }

            if k.eq_ignore_ascii_case(OTEL_EXPORTER_OTLP_METRICS_PROTOCOL) {
                config.metrics.protocol = protocol(v.as_ref());
                continue;
            }

            // Endpoint

            if k.eq_ignore_ascii_case(OTEL_EXPORTER_OTLP_ENDPOINT) {
                config.base.endpoint = endpoint(v.as_ref());
                continue;
            }

            if k.eq_ignore_ascii_case(OTEL_EXPORTER_OTLP_LOGS_ENDPOINT) {
                config.logs.endpoint = endpoint(v.as_ref());
                continue;
            }

            if k.eq_ignore_ascii_case(OTEL_EXPORTER_OTLP_TRACES_ENDPOINT) {
                config.traces.endpoint = endpoint(v.as_ref());
                continue;
            }

            if k.eq_ignore_ascii_case(OTEL_EXPORTER_OTLP_METRICS_ENDPOINT) {
                config.metrics.endpoint = endpoint(v.as_ref());
                continue;
            }

            // Headers

            if k.eq_ignore_ascii_case(OTEL_EXPORTER_OTLP_HEADERS) {
                config.base.headers = headers(v.as_ref());
                continue;
            }

            if k.eq_ignore_ascii_case(OTEL_EXPORTER_OTLP_LOGS_HEADERS) {
                config.logs.headers = headers(v.as_ref());
                continue;
            }

            if k.eq_ignore_ascii_case(OTEL_EXPORTER_OTLP_TRACES_HEADERS) {
                config.traces.headers = headers(v.as_ref());
                continue;
            }

            if k.eq_ignore_ascii_case(OTEL_EXPORTER_OTLP_METRICS_HEADERS) {
                config.metrics.headers = headers(v.as_ref());
                continue;
            }

            // Timeout

            if k.eq_ignore_ascii_case(OTEL_EXPORTER_OTLP_TIMEOUT) {
                config.base.timeout = timeout(v.as_ref());
                continue;
            }

            if k.eq_ignore_ascii_case(OTEL_EXPORTER_OTLP_LOGS_TIMEOUT) {
                config.logs.timeout = timeout(v.as_ref());
                continue;
            }

            if k.eq_ignore_ascii_case(OTEL_EXPORTER_OTLP_TRACES_TIMEOUT) {
                config.traces.timeout = timeout(v.as_ref());
                continue;
            }

            if k.eq_ignore_ascii_case(OTEL_EXPORTER_OTLP_METRICS_TIMEOUT) {
                config.metrics.timeout = timeout(v.as_ref());
                continue;
            }

            // Service name

            if k.eq_ignore_ascii_case(OTEL_SERVICE_NAME) {
                config.service_name = service_name(v.as_ref());
                continue;
            }

            // Resource

            if k.eq_ignore_ascii_case(OTEL_RESOURCE_ATTRIBUTES) {
                use emit::Props as _;

                let mut resource = HashMap::new();

                let _ = ResourceConfig::from_env(v.as_ref()).for_each(|k, v| {
                    if resource.get(k.get()).is_some() {
                        return ControlFlow::Continue(());
                    }

                    resource.insert(k.to_owned(), v.to_owned());

                    ControlFlow::Continue(())
                });

                config.resource = resource;
                continue;
            }
        }

        config
    }
}

#[derive(Value, Default)]
struct SignalConfig {
    protocol: Option<ProtocolConfig>,
    endpoint: Option<String>,
    headers: HashMap<String, Vec<String>>,
    timeout: Option<u64>,
}

#[derive(Value, Clone, Copy)]
enum ProtocolConfig {
    #[sval(label = "grpc")]
    Grpc,
    #[sval(label = "http/protobuf")]
    HttpProtobuf,
    #[sval(label = "http/json")]
    HttpJson,
}

impl Default for ProtocolConfig {
    fn default() -> Self {
        ProtocolConfig::Grpc
    }
}

impl SignalConfig {
    fn protocol(&self, base: &Self) -> ProtocolConfig {
        self.protocol.or(base.protocol).unwrap_or_default()
    }

    fn endpoint<'a>(&'a self, base: &'a Self, http_subpath: &str) -> Cow<'a, str> {
        if let Some(endpoint) = self.endpoint.as_ref().or_else(|| base.endpoint.as_ref()) {
            return Cow::Borrowed(endpoint);
        }

        match self.protocol(base) {
            ProtocolConfig::Grpc => {
                Cow::Borrowed(base.endpoint.as_deref().unwrap_or("http://localhost:4317"))
            }
            ProtocolConfig::HttpJson | ProtocolConfig::HttpProtobuf => {
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

struct ResourceConfig<'a>(Vec<(&'a str, baggage::Value<'a>)>);

impl<'a> ResourceConfig<'a> {
    fn from_env(v: &'a str) -> Self {
        let v = trim(v);

        match baggage::parse(v) {
            Ok(baggage) => ResourceConfig(baggage),
            Err(err) => {
                emit::warn!(rt: emit::runtime::internal(), "failed to parse resource: {err}");

                ResourceConfig(Vec::new())
            }
        }
    }
}

impl<'a> emit::Props for ResourceConfig<'a> {
    fn for_each<'kv, F: FnMut(emit::Str<'kv>, emit::Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        mut for_each: F,
    ) -> ControlFlow<()> {
        enum ResourceValue {
            Integer(u64),
            Double(f64),
            Boolean(bool),
        }

        impl FromStr for ResourceValue {
            type Err = ();

            fn from_str(v: &str) -> Result<Self, Self::Err> {
                if let Ok(integer) = v.parse::<u64>() {
                    return Ok(ResourceValue::Integer(integer));
                }

                if let Ok(double) = v.parse::<f64>() {
                    return Ok(ResourceValue::Double(double));
                }

                if let Ok(boolean) = v.parse::<bool>() {
                    return Ok(ResourceValue::Boolean(boolean));
                }

                Err(())
            }
        }

        #[repr(transparent)]
        struct ResourceValueMap<'a>(Vec<(&'a str, baggage::Property<'a>)>);

        impl<'a> ResourceValueMap<'a> {
            fn new_ref(v: &'a Vec<(&'a str, baggage::Property<'a>)>) -> &'a Self {
                // SAFETY: `Vec<(&'a str, baggage::Property<'a>)>` and `ResourceValueMap<'a>` have the same ABI
                unsafe {
                    &*(v as *const Vec<(&'a str, baggage::Property<'a>)>
                        as *const ResourceValueMap<'a>)
                }
            }
        }

        impl<'a> sval::Value for ResourceValueMap<'a> {
            fn stream<'sval, S: sval::Stream<'sval> + ?Sized>(
                &'sval self,
                stream: &mut S,
            ) -> sval::Result {
                stream.map_begin(Some(self.0.len()))?;

                for (k, v) in &self.0 {
                    stream.map_key_begin()?;
                    stream.value(k)?;
                    stream.map_key_end()?;

                    stream.map_value_begin()?;
                    match v {
                        baggage::Property::None => stream.bool(true)?,
                        baggage::Property::Single(v) => match v.parse::<ResourceValue>() {
                            Ok(ResourceValue::Integer(v)) => stream.u64(v)?,
                            Ok(ResourceValue::Double(v)) => stream.f64(v)?,
                            Ok(ResourceValue::Boolean(v)) => stream.bool(v)?,
                            Err(()) => stream.value(&**v)?,
                        },
                    }
                    stream.map_value_end()?;
                }

                stream.map_end()
            }
        }

        for (k, v) in &self.0 {
            match v {
                baggage::Value::Single(v) => {
                    for_each(
                        emit::Str::new_ref(k),
                        match v.parse::<ResourceValue>() {
                            Ok(ResourceValue::Integer(v)) => emit::Value::from(v),
                            Ok(ResourceValue::Double(v)) => emit::Value::from(v),
                            Ok(ResourceValue::Boolean(v)) => emit::Value::from(v),
                            Err(()) => emit::Value::from(v),
                        },
                    )?;
                }
                baggage::Value::List(v) => {
                    for_each(
                        emit::Str::new_ref(k),
                        emit::Value::from_sval(ResourceValueMap::new_ref(v)),
                    )?;
                }
            }
        }

        ControlFlow::Continue(())
    }
}

fn trim(v: &str) -> &str {
    v.trim()
}
