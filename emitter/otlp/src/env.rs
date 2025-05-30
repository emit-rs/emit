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
    baggage, telemetry_sdk_language, telemetry_sdk_name, telemetry_sdk_version, Error, OtlpBuilder,
    OtlpLogsBuilder, OtlpMetricsBuilder, OtlpTracesBuilder, OtlpTransportBuilder,
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

    See [Configuring from environment variables](index.html#configuring-from-environment-variables) for details.
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

    See [Configuring from environment variables](index.html#configuring-from-environment-variables) for details.
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

    See [Configuring from environment variables](index.html#configuring-from-environment-variables) for details.
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

    See [Configuring from environment variables](index.html#configuring-from-environment-variables) for details.
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

/**
Read resource attributes from OpenTelemetry's environment variables.

This function will return [`emit::Props`] containing attributes from the environment, as well as standard attributes for `telemetry.sdk`.

See [Configuring from environment variables](index.html#configuring-from-environment-variables) for details.
*/
pub fn resource_from_env() -> impl emit::Props {
    let config = &*CONFIG;

    &config.resource
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

        // Set resource values

        if let Some(service_name) = config_service_name {
            config.resource.insert(
                emit::Str::new("service.name"),
                emit::Value::from(&service_name).to_owned(),
            );
        } else {
            config
                .resource
                .entry(emit::Str::new("service.name"))
                .or_insert_with(|| emit::Value::from("unknown_service").to_owned());
        }

        config
            .resource
            .entry(emit::Str::new("telemetry.sdk.name"))
            .or_insert_with(|| emit::Value::from(telemetry_sdk_name()).to_owned());
        config
            .resource
            .entry(emit::Str::new("telemetry.sdk.version"))
            .or_insert_with(|| emit::Value::from(telemetry_sdk_version()).to_owned());
        config
            .resource
            .entry(emit::Str::new("telemetry.sdk.language"))
            .or_insert_with(|| emit::Value::from(telemetry_sdk_language()).to_owned());

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
        for (k, v) in &self.0 {
            match v {
                baggage::Value::Single(v) => {
                    for_each(emit::Str::new_ref(k), emit::Value::from(v))?;
                }
                baggage::Value::List(_) => (),
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

        assert_eq!(
            {
                let mut map = HashMap::new();

                map.insert("service.name".to_string(), "unknown_service".to_string());
                map.insert("telemetry.sdk.name".to_string(), "emit_otlp".to_string());
                map.insert(
                    "telemetry.sdk.version".to_string(),
                    telemetry_sdk_version().to_string(),
                );
                map.insert("telemetry.sdk.language".to_string(), "rust".to_string());

                map
            },
            config
                .resource
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect::<HashMap<_, _>>(),
        );
    }

    #[test]
    fn config_from_env_resource() {
        let env = vec![
            ("OTEL_RESOURCE_ATTRIBUTES", "service.namespace=tutorial,service.version=1.0,service.instance.id=46D99F44-27AB-4006-9F57-3B7C9032827B,host.name=myhost,host.type=arm64,os.name=linux,os.version=6.0"),
        ];

        let config = OtlpConfig::from_env(env.into_iter());

        assert_eq!(
            {
                let mut map = HashMap::new();

                map.insert("service.name".to_string(), "unknown_service".to_string());
                map.insert("telemetry.sdk.name".to_string(), "emit_otlp".to_string());
                map.insert(
                    "telemetry.sdk.version".to_string(),
                    telemetry_sdk_version().to_string(),
                );
                map.insert("telemetry.sdk.language".to_string(), "rust".to_string());

                map.insert("service.namespace".to_string(), "tutorial".to_string());
                map.insert("service.version".to_string(), "1.0".to_string());
                map.insert(
                    "service.instance.id".to_string(),
                    "46D99F44-27AB-4006-9F57-3B7C9032827B".to_string(),
                );
                map.insert("host.name".to_string(), "myhost".to_string());
                map.insert("host.type".to_string(), "arm64".to_string());
                map.insert("os.name".to_string(), "linux".to_string());
                map.insert("os.version".to_string(), "6.0".to_string());

                map
            },
            config
                .resource
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect::<HashMap<_, _>>(),
        );
    }

    #[test]
    fn config_from_env_resource_ignores_list_valued_properties() {
        let env = vec![
            ("OTEL_RESOURCE_ATTRIBUTES", "service.namespace=tutorial;service.version=1.0;service.instance.id=46D99F44-27AB-4006-9F57-3B7C9032827B,host.name=myhost"),
        ];

        let config = OtlpConfig::from_env(env.into_iter());

        let resource = config
            .resource
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect::<HashMap<_, _>>();

        assert!(resource.get("service.namespace").is_none());
        assert!(resource.get("service.version").is_none());
        assert!(resource.get("service.instance.id").is_none());

        assert!(resource.get("host.name").is_some());
    }

    #[test]
    fn config_from_env_service_name() {
        let env = vec![
            ("OTEL_SERVICE_NAME", "myservice"),
            ("OTEL_RESOURCE_ATTRIBUTES", "service.name=notmyservice"),
        ];

        let config = OtlpConfig::from_env(env.into_iter());

        assert_eq!("myservice", config.resource["service.name"].to_string());
    }

    #[test]
    fn config_from_env_protocol_grpc_default() {
        let env = vec![("OTEL_EXPORTER_OTLP_PROTOCOL", "http/protobuf")];

        let config = OtlpConfig::from_env(env.into_iter());

        assert_eq!(
            ProtocolConfig::HttpProtobuf,
            config.logs.protocol(&config.base)
        );

        assert_eq!(
            "http://localhost:4318/v1/logs",
            config.logs.endpoint(&config.base, "v1/logs")
        );

        assert_eq!(
            "http://localhost:4318/v1/traces",
            config.traces.endpoint(&config.base, "v1/traces")
        );

        assert_eq!(
            "http://localhost:4318/v1/metrics",
            config.metrics.endpoint(&config.base, "v1/metrics")
        );
    }

    #[test]
    fn config_from_env_protocol_http_default() {
        let env = vec![("OTEL_EXPORTER_OTLP_PROTOCOL", "http/protobuf")];

        let config = OtlpConfig::from_env(env.into_iter());

        assert_eq!(
            ProtocolConfig::HttpProtobuf,
            config.logs.protocol(&config.base)
        );

        assert_eq!(
            "http://localhost:4318/v1/logs",
            config.logs.endpoint(&config.base, "v1/logs")
        );

        assert_eq!(
            "http://localhost:4318/v1/traces",
            config.traces.endpoint(&config.base, "v1/traces")
        );

        assert_eq!(
            "http://localhost:4318/v1/metrics",
            config.metrics.endpoint(&config.base, "v1/metrics")
        );
    }

    #[test]
    fn config_from_env_protocol_override() {
        let env = vec![
            ("OTEL_EXPORTER_OTLP_PROTOCOL", "http/protobuf"),
            ("OTEL_EXPORTER_OTLP_TRACES_PROTOCOL", "grpc"),
        ];

        let config = OtlpConfig::from_env(env.into_iter());

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
            "http://localhost:4317",
            config.traces.endpoint(&config.base, "v1/traces")
        );
    }

    #[test]
    fn config_from_env_endpoint_override() {
        let env = vec![("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT", "http://traces.local")];

        let config = OtlpConfig::from_env(env.into_iter());

        assert_eq!(
            "http://traces.local",
            config.traces.endpoint(&config.base, "v1/traces")
        );
    }

    #[test]
    fn config_from_env_headers() {
        let env = vec![
            (
                "OTEL_EXPORTER_OTLP_HEADERS",
                "X-ApiKey=Api-46D99F4427AB40069F573B7C9032827B,X-Service=myhost",
            ),
            (
                "OTEL_EXPORTER_OTLP_TRACES_HEADERS",
                "X-ApiKey=tracekey,X-TraceKind=server",
            ),
        ];

        let config = OtlpConfig::from_env(env.into_iter());

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
    fn config_from_env_headers_ignores_list_valued_properties() {
        let env = vec![
            (
                "OTEL_EXPORTER_OTLP_HEADERS",
                "X-ApiKey=Api-46D99F4427AB40069F573B7C9032827B;X-ApiEndpoint=localhost,X-Service=myhost",
            ),
        ];

        let config = OtlpConfig::from_env(env.into_iter());

        let headers = config.logs.headers(&config.base).collect::<HashMap<_, _>>();

        assert!(headers.get("X-ApiKey").is_none());
        assert!(headers.get("X-Service").is_some());
    }
}
