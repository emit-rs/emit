use std::{fmt, sync::Arc};

use crate::{
    data::logs::{self, LogsEventEncoder, LogsRequestEncoder},
    internal_metrics::InternalMetrics,
    Encoding, Error, OtlpTransportBuilder,
};

use super::{
    encode_resource, ClientEventEncoder, ClientRequestEncoder, OtlpTransport, Protocol, Resource,
};

/**
A builder for the logs signal.

Pass the resulting builder to [`crate::OtlpBuilder::logs`] to configure the logs signal for an OTLP pipeline.
*/
pub struct OtlpLogsBuilder {
    event_encoder: logs::LogsEventEncoder,
    request_encoder: logs::LogsRequestEncoder,
    encoding: Encoding,
    transport: OtlpTransportBuilder,
}

impl OtlpLogsBuilder {
    fn new(encoding: Encoding, transport: OtlpTransportBuilder) -> Self {
        OtlpLogsBuilder {
            event_encoder: logs::LogsEventEncoder::default(),
            request_encoder: logs::LogsRequestEncoder::default(),
            encoding,
            transport,
        }
    }

    /**
    Get a logs signal builder with the given transport with protobuf encoding.
    */
    pub fn proto(mut transport: OtlpTransportBuilder) -> Self {
        if let Protocol::Grpc = transport.protocol {
            transport.url_path = Some("opentelemetry.proto.collector.logs.v1.LogsService/Export");
        }

        Self::new(Encoding::Proto, transport)
    }

    /**
    Get a logs signal builder for HTTP+protobuf.

    The `dst` argument should include the complete path to the OTLP endpoint for the given signal, like `http://localhost:4318/v1/logs`.
    */
    pub fn http_proto(dst: impl Into<String>) -> Self {
        Self::proto(OtlpTransportBuilder::http(dst))
    }

    /**
    Get a logs signal builder for gRPC+protobuf.

    The `dst` argument should include just the root of the target gRPC service, like `http://localhost:4319`.
    */
    pub fn grpc_proto(dst: impl Into<String>) -> Self {
        Self::proto(OtlpTransportBuilder::grpc(dst))
    }

    /**
    Get a logs signal builder with the given transport with JSON encoding.
    */
    pub fn json(transport: OtlpTransportBuilder) -> Self {
        Self::new(Encoding::Json, transport)
    }

    /**
    Get a logs signal builder for HTTP+JSON.

    The `dst` argument should include the complete path to the OTLP endpoint for the given signal, like `http://localhost:4318/v1/logs`.
    */
    pub fn http_json(dst: impl Into<String>) -> Self {
        Self::json(OtlpTransportBuilder::http(dst))
    }

    /**
    Use the given `writer` function to format the string body of the OTLP log record for a given [`emit::Event`].
    */
    pub fn body(
        mut self,
        writer: impl Fn(
                &emit::event::Event<&dyn emit::props::ErasedProps>,
                &mut fmt::Formatter,
            ) -> fmt::Result
            + Send
            + Sync
            + 'static,
    ) -> Self {
        self.event_encoder.body = Box::new(writer);
        self
    }

    pub(in crate::client) fn build(
        self,
        metrics: Arc<InternalMetrics>,
        resource: Option<&Resource>,
    ) -> Result<
        (
            ClientEventEncoder<LogsEventEncoder>,
            OtlpTransport<LogsRequestEncoder>,
        ),
        Error,
    > {
        Ok((
            ClientEventEncoder::new(self.encoding, self.event_encoder),
            self.transport.build(
                metrics.clone(),
                resource
                    .as_ref()
                    .map(|resource| encode_resource(self.encoding, resource)),
                ClientRequestEncoder::new(self.encoding, self.request_encoder),
            )?,
        ))
    }
}
