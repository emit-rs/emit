/*!
Support configuration via environment variables.

The set of variables is defined by the [OpenTelemetry spec](https://opentelemetry.io/docs/languages/sdk-configuration/otlp-exporter/).

If a variable is missing or invalid, then its default value is used.

All signal-specific variables override generic ones, except for headers, where the results are merged.
Header values defined in a signal-specific value override values defined in the generic one.
*/

use std::{borrow::Cow, collections::HashMap, env, ops::ControlFlow, sync::LazyLock};

use sval_derive::Value;

use crate::{
    baggage, Error, OtlpBuilder, OtlpLogsBuilder, OtlpMetricsBuilder, OtlpTracesBuilder,
    OtlpTransportBuilder,
};

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

const OTEL_SERVICE_NAME: &'static str = "OTEL_SERVICE_NAME";

const OTEL_RESOURCE_ATTRIBUTES: &'static str = "OTEL_RESOURCE_ATTRIBUTES";

impl OtlpBuilder {
    /**
    Create a builder with configuration from OpenTelemetry's environment variables.
    */
    pub fn from_env() -> Self {
        let config = &*CONFIG;

        OtlpBuilder::new()
            .resource(&config.resource)
            .logs(OtlpLogsBuilder::from_env())
            .traces(OtlpTracesBuilder::from_env())
            .metrics(OtlpMetricsBuilder::from_env())
    }
}

impl OtlpLogsBuilder {
    /**
    Create a builder with configuration from OpenTelemetry's environment variables.
    */
    pub fn from_env() -> OtlpLogsBuilder {
        let config = &*CONFIG;

        let transport = transport(&config.logs, &config.base, "v1/logs");

        match config.logs.protocol(&config.base) {
            ProtocolConfig::Grpc => OtlpLogsBuilder::proto(transport),
            ProtocolConfig::HttpProtobuf => OtlpLogsBuilder::proto(transport),
            ProtocolConfig::HttpJson => OtlpLogsBuilder::json(transport),
        }
    }
}

impl OtlpTracesBuilder {
    /**
    Create a builder with configuration from OpenTelemetry's environment variables.
    */
    pub fn from_env() -> OtlpTracesBuilder {
        let config = &*CONFIG;

        let transport = transport(&config.traces, &config.base, "v1/traces");

        match config.traces.protocol(&config.base) {
            ProtocolConfig::Grpc => OtlpTracesBuilder::proto(transport),
            ProtocolConfig::HttpProtobuf => OtlpTracesBuilder::proto(transport),
            ProtocolConfig::HttpJson => OtlpTracesBuilder::json(transport),
        }
    }
}

impl OtlpMetricsBuilder {
    /**
    Create a builder with configuration from OpenTelemetry's environment variables.
    */
    pub fn from_env() -> OtlpMetricsBuilder {
        let config = &*CONFIG;

        let transport = transport(&config.metrics, &config.base, "v1/metrics");

        match config.metrics.protocol(&config.base) {
            ProtocolConfig::Grpc => OtlpMetricsBuilder::proto(transport),
            ProtocolConfig::HttpProtobuf => OtlpMetricsBuilder::proto(transport),
            ProtocolConfig::HttpJson => OtlpMetricsBuilder::json(transport),
        }
    }
}

fn transport(signal: &SignalConfig, base: &SignalConfig, path: &str) -> OtlpTransportBuilder {
    let transport = match signal.protocol(&base) {
        ProtocolConfig::Grpc => OtlpTransportBuilder::grpc(signal.endpoint(&base, path)),
        ProtocolConfig::HttpProtobuf => OtlpTransportBuilder::http(signal.endpoint(&base, path)),
        ProtocolConfig::HttpJson => OtlpTransportBuilder::http(signal.endpoint(&base, path)),
    };

    transport.headers(signal.headers(&base))
}

static CONFIG: LazyLock<OtlpConfig> = LazyLock::new(|| OtlpConfig::from_env(env::vars()));

#[derive(Value, Default, Debug)]
pub(crate) struct OtlpConfig {
    base: SignalConfig,
    logs: SignalConfig,
    traces: SignalConfig,
    metrics: SignalConfig,
    resource: HashMap<emit::Str<'static>, emit::value::OwnedValue>,
}

impl OtlpConfig {
    pub(crate) fn from_env<K: AsRef<str>, V: AsRef<str>>(
        env: impl Iterator<Item = (K, V)>,
    ) -> OtlpConfig {
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

        fn service_name(v: &str) -> Option<String> {
            let v = trim(v);

            Some(v.to_owned())
        }

        let mut config = OtlpConfig::default();
        let mut config_service_name = None;

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

            // Service name

            if k.eq_ignore_ascii_case(OTEL_SERVICE_NAME) {
                config_service_name = service_name(v.as_ref());
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

        if let Some(service_name) = config_service_name {
            config.resource.insert(
                emit::Str::new("service.name"),
                emit::Value::from(&service_name).to_owned(),
            );
        } else if config.resource.get("service.name").is_none() {
            config.resource.insert(
                emit::Str::new("service.name"),
                emit::Value::from("unknown_service").to_owned(),
            );
        }

        config
    }
}

#[derive(Value, Default, Debug, PartialEq)]
struct SignalConfig {
    protocol: Option<ProtocolConfig>,
    endpoint: Option<String>,
    headers: HashMap<String, Vec<String>>,
}

#[derive(Value, Clone, Copy, Debug, PartialEq)]
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
                        baggage::Property::Single(v) => stream.value(&**v)?,
                    }
                    stream.map_value_end()?;
                }

                stream.map_end()
            }
        }

        for (k, v) in &self.0 {
            match v {
                baggage::Value::Single(v) => {
                    for_each(emit::Str::new_ref(k), emit::Value::from(v))?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_from_env_empty() {
        let env = Vec::<(String, String)>::new();

        let config = OtlpConfig::from_env(env.into_iter());

        assert_eq!(SignalConfig::default(), config.base);
        assert_eq!(SignalConfig::default(), config.logs);
        assert_eq!(SignalConfig::default(), config.traces);
        assert_eq!(SignalConfig::default(), config.metrics);

        assert_eq!(0, config.resource.len());
    }

    #[test]
    fn config_from_env() {
        let env = vec![
            ("OTEL_EXPORTER_OTLP_PROTOCOL", "http/protobuf"),
            ("OTEL_EXPORTER_OTLP_TRACES_PROTOCOL", "grpc"),
            ("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT", "http://traces.local"),
            ("OTEL_EXPORTER_OTLP_HEADERS", "X-ApiKey=Api-46D99F4427AB40069F573B7C9032827B,X-Service=myhost"),
            ("OTEL_EXPORTER_OTLP_TRACES_HEADERS", "X-ApiKey=tracekey,X-TraceKind=server"),
            ("OTEL_SERVICE_NAME", "myservice"),
            ("OTEL_RESOURCE_ATTRIBUTES", "service.namespace=tutorial,service.version=1.0,service.instance.id=46D99F44-27AB-4006-9F57-3B7C9032827B,host.name=myhost,host.type=arm64,os.name=linux,os.version=6.0"),
        ];

        let config = OtlpConfig::from_env(env.into_iter());

        assert_eq!("tutorial", config.resource["service.namespace"].to_string());
        assert_eq!("1.0", config.resource["service.version"].to_string());
        assert_eq!(
            "46D99F44-27AB-4006-9F57-3B7C9032827B",
            config.resource["service.instance.id"].to_string()
        );
        assert_eq!("myhost", config.resource["host.name"].to_string());
        assert_eq!("arm64", config.resource["host.type"].to_string());
        assert_eq!("linux", config.resource["os.name"].to_string());
        assert_eq!("6.0", config.resource["os.version"].to_string());

        assert_eq!(
            ProtocolConfig::HttpProtobuf,
            config.logs.protocol(&config.base)
        );

        assert_eq!(ProtocolConfig::Grpc, config.traces.protocol(&config.base));

        assert_eq!(
            "http://localhost:4318/v1/logs",
            config.logs.endpoint(&config.base, "v1/logs")
        );

        assert_eq!(
            "http://traces.local",
            config.traces.endpoint(&config.base, "v1/traces")
        );

        assert_eq!(
            {
                let mut map = HashMap::new();

                map.insert("X-ApiKey", "Api-46D99F4427AB40069F573B7C9032827B");
                map.insert("X-Service", "myhost");

                map
            },
            config.logs.headers(&config.base).collect::<HashMap<_, _>>(),
        );

        assert_eq!(
            {
                let mut map = HashMap::new();

                map.insert("X-ApiKey", "tracekey");
                map.insert("X-Service", "myhost");
                map.insert("X-TraceKind", "server");

                map
            },
            config
                .traces
                .headers(&config.base)
                .collect::<HashMap<_, _>>(),
        );
    }

    #[test]
    fn config_from_env_default() {
        let env = Vec::<(String, String)>::new();

        let config = OtlpConfig::from_env(env.into_iter());

        assert_eq!(
            "unknown_service",
            config.resource["service.name"].to_string()
        );
    }
}
