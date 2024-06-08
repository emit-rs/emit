use emit_batcher::BatchError;
use std::{collections::HashMap, mem, sync::Arc, time::Duration};

use crate::{
    data::{
        self,
        logs::{LogsEventEncoder, LogsRequestEncoder},
        metrics::{MetricsEventEncoder, MetricsRequestEncoder},
        traces::{TracesEventEncoder, TracesRequestEncoder},
        EncodedEvent, EncodedPayload, EncodedScopeItems, RawEncoder, RequestEncoder,
    },
    internal_metrics::InternalMetrics,
    Error, OtlpMetrics,
};

use self::http::HttpConnection;

mod http;
mod logs;
mod metrics;
mod traces;

pub use self::{logs::*, metrics::*, traces::*};

/**
An [`emit::Emitter`] that sends diagnostic events via the OpenTelemetry Protocol (OTLP).

Use [`crate::new`] to start an [`OtlpBuilder`] for configuring an [`Otlp`] instance.

See the crate root documentation for more details.
*/
pub struct Otlp {
    otlp_logs: Option<ClientEventEncoder<LogsEventEncoder>>,
    otlp_traces: Option<ClientEventEncoder<TracesEventEncoder>>,
    otlp_metrics: Option<ClientEventEncoder<MetricsEventEncoder>>,
    metrics: Arc<InternalMetrics>,
    sender: emit_batcher::Sender<Channel>,
}

impl Otlp {
    /**
    Start a builder for configuring an [`Otlp`] instance.

    The [`OtlpBuilder`] can be completed by calling [`OtlpBuilder::spawn`].
    */
    pub fn builder() -> OtlpBuilder {
        OtlpBuilder::new()
    }

    /**
    Get an [`emit::metric::Source`] for instrumentation produced by an [`Otlp`] instance.

    These metrics can be used to monitor the running health of your diagnostic pipeline.
    */
    pub fn metric_source(&self) -> OtlpMetrics {
        OtlpMetrics {
            channel_metrics: self.sender.metric_source(),
            metrics: self.metrics.clone(),
        }
    }
}

/**
A builder for [`Otlp`].

Use [`crate::new`] to start a builder and [`OtlpBuilder::spawn`] to complete it, passing the resulting [`Otlp`] to [`emit::Setup::emit_to`].

Signals can be configured on the builder through [`OtlpBuilder::logs`], [`OtlpBuilder::traces`], and [`OtlpBuilder::metrics`].

See the crate root documentation for more details.
*/
#[must_use = "call `.spawn()` to complete the builder"]
pub struct OtlpBuilder {
    resource: Option<Resource>,
    otlp_logs: Option<OtlpLogsBuilder>,
    otlp_traces: Option<OtlpTracesBuilder>,
    otlp_metrics: Option<OtlpMetricsBuilder>,
}

impl OtlpBuilder {
    /**
    Start a builder for an [`Otlp`] emitter.

    Signals can be configured on the builder through [`OtlpBuilder::logs`], [`OtlpBuilder::traces`], and [`OtlpBuilder::metrics`].

    Once the builder is configured, call [`OtlpBuilder::spawn`] to complete it, passing the resulting [`Otlp`] to [`emit::Setup::emit_to`].

    See the crate root documentation for more details.
    */
    pub fn new() -> Self {
        OtlpBuilder {
            resource: None,
            otlp_logs: None,
            otlp_traces: None,
            otlp_metrics: None,
        }
    }

    /**
    Configure the logs signal.
    */
    pub fn logs(mut self, builder: OtlpLogsBuilder) -> Self {
        self.otlp_logs = Some(builder);
        self
    }

    /**
    Configure the traces signal.
    */
    pub fn traces(mut self, builder: OtlpTracesBuilder) -> Self {
        self.otlp_traces = Some(builder);
        self
    }

    /**
    Configure the metrics signal.
    */
    pub fn metrics(mut self, builder: OtlpMetricsBuilder) -> Self {
        self.otlp_metrics = Some(builder);
        self
    }

    /**
    Configure the resource.

    Some OTLP receivers accept data without a resource but the OpenTelemetry specification itself mandates it.
    */
    pub fn resource(mut self, attributes: impl emit::props::Props) -> Self {
        let mut resource = Resource {
            attributes: HashMap::new(),
        };

        attributes.for_each(|k, v| {
            resource.attributes.insert(k.to_owned(), v.to_owned());

            std::ops::ControlFlow::Continue(())
        });

        self.resource = Some(resource);

        self
    }

    /**
    Try spawn an [`Otlp`] instance which can be used to send diagnostic events via OTLP.

    This method will fail if any previously configured values are invalid, such as malformed URIs.

    See the crate root documentation for more details.
    */
    pub fn spawn(self) -> Result<Otlp, Error> {
        let metrics = Arc::new(InternalMetrics::default());

        let (sender, receiver) = emit_batcher::bounded(10_000);

        // Encoders are used by the caller to convert emit's events
        // into the right OTLP item
        let mut logs_event_encoder = None;
        let mut traces_event_encoder = None;
        let mut metrics_event_encoder = None;

        // Build the client
        // This type is used by the background worker to send requests
        // It owns the actual HTTP connections used by each configured signal
        let client = OtlpClient {
            logs: match self.otlp_logs {
                Some(otlp_logs) => {
                    let (encoder, transport) =
                        otlp_logs.build(metrics.clone(), self.resource.as_ref())?;

                    logs_event_encoder = Some(encoder);
                    Some(Arc::new(transport))
                }
                None => None,
            },
            traces: match self.otlp_traces {
                Some(otlp_traces) => {
                    let (encoder, transport) =
                        otlp_traces.build(metrics.clone(), self.resource.as_ref())?;

                    traces_event_encoder = Some(encoder);
                    Some(Arc::new(transport))
                }
                None => None,
            },
            metrics: match self.otlp_metrics {
                Some(otlp_metrics) => {
                    let (encoder, transport) =
                        otlp_metrics.build(metrics.clone(), self.resource.as_ref())?;

                    metrics_event_encoder = Some(encoder);
                    Some(Arc::new(transport))
                }
                None => None,
            },
        };

        // Spawn the background worker to receive and forward batches of OTLP items
        emit_batcher::tokio::spawn(receiver, move |batch: Channel| {
            async fn send_channel_batch<R: RequestEncoder>(
                r: &mut Result<(), BatchError<Channel>>,
                client: &OtlpTransport<R>,
                batch: EncodedScopeItems,
                set: impl Fn(&mut Channel, EncodedScopeItems),
            ) {
                // Attempt to send the batch, restoring it on the channel if it fails
                if let Err(e) = client.send(batch).await {
                    *r = if let Err(re) = mem::replace(r, Ok(())) {
                        Err(re.map_retryable(|channel| {
                            let mut channel = channel.unwrap_or_default();
                            set(&mut channel, e.into_retryable().unwrap_or_default());

                            Some(channel)
                        }))
                    } else {
                        Err(e.map_retryable(|batch| {
                            let mut channel = Channel::default();
                            set(&mut channel, batch.unwrap_or_default());

                            Some(channel)
                        }))
                    };
                }
            }

            let client = client.clone();

            /*
            NOTE: Possible degenerate behavior here where one signal blocks others;
            the logs endpoint is flaky and fails a lot, so it means traces also get
            backed up waiting for retries of logs to succeed.
            */
            async move {
                let Channel {
                    otlp_logs,
                    otlp_traces,
                    otlp_metrics,
                } = batch;

                let mut r = Ok::<(), BatchError<Channel>>(());

                // Send a batch of OTLP log records
                if otlp_logs.total_items() > 0 {
                    if let Some(client) = client.logs {
                        send_channel_batch(&mut r, &client, otlp_logs, |channel, otlp_logs| {
                            channel.otlp_logs = otlp_logs;
                        })
                        .await;
                    }
                }

                // Send a batch of OTLP spans
                if otlp_traces.total_items() > 0 {
                    if let Some(client) = client.traces {
                        send_channel_batch(&mut r, &client, otlp_traces, |channel, otlp_traces| {
                            channel.otlp_traces = otlp_traces;
                        })
                        .await;
                    }
                }

                // Send a batch of OTLP metrics
                if otlp_metrics.total_items() > 0 {
                    if let Some(client) = client.metrics {
                        send_channel_batch(
                            &mut r,
                            &client,
                            otlp_metrics,
                            |channel, otlp_metrics| {
                                channel.otlp_metrics = otlp_metrics;
                            },
                        )
                        .await;
                    }
                }

                r
            }
        });

        Ok(Otlp {
            otlp_logs: logs_event_encoder,
            otlp_traces: traces_event_encoder,
            otlp_metrics: metrics_event_encoder,
            metrics,
            sender,
        })
    }
}

/**
A builder for an OTLP transport channel, either HTTP or gRPC.

Use [`crate::http`] or [`crate::grpc`] to start a new builder.
*/
pub struct OtlpTransportBuilder {
    protocol: Protocol,
    url_base: String,
    allow_compression: bool,
    url_path: Option<&'static str>,
    headers: Vec<(String, String)>,
}

impl OtlpTransportBuilder {
    /**
    Create a transport builder for OTLP via HTTP.

    The `dst` argument should include the complete path to the OTLP endpoint for the given signal, like:

    - `http://localhost:4318/v1/logs` for the logs signal.
    - `http://localhost:4318/v1/traces` for the traces signal.
    - `http://localhost:4318/v1/metrics` for the metrics signal.
    */
    pub fn http(dst: impl Into<String>) -> Self {
        OtlpTransportBuilder {
            protocol: Protocol::Http,
            allow_compression: true,
            url_base: dst.into(),
            url_path: None,
            headers: Vec::new(),
        }
    }

    /**
    Create a transport builder for OTLP via gRPC.

    The `dst` argument should include just the root of the target gRPC service, like `http://localhost:4319`.
    */
    pub fn grpc(dst: impl Into<String>) -> Self {
        OtlpTransportBuilder {
            protocol: Protocol::Grpc,
            allow_compression: true,
            url_base: dst.into(),
            url_path: None,
            headers: Vec::new(),
        }
    }

    /**
    Set custom headers to be included in each request to the target service.

    Duplicate header keys are allowed.
    */
    pub fn headers<K: Into<String>, V: Into<String>>(
        mut self,
        headers: impl IntoIterator<Item = (K, V)>,
    ) -> Self {
        self.headers = headers
            .into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect();

        self
    }

    /**
    Whether to compress request payloads.

    Passing `false` to this method will disable compression on all requests. If the URI scheme is HTTPS then no compression will be applied either way.
    */
    #[cfg(feature = "gzip")]
    pub fn allow_compression(mut self, allow: bool) -> Self {
        self.allow_compression = allow;

        self
    }

    fn build<R>(
        self,
        metrics: Arc<InternalMetrics>,
        resource: Option<EncodedPayload>,
        request_encoder: ClientRequestEncoder<R>,
    ) -> Result<OtlpTransport<R>, Error> {
        let mut url = self.url_base;

        if let Some(path) = self.url_path {
            if !url.ends_with("/") && !path.starts_with("/") {
                url.push('/');
            }

            url.push_str(&path);
        }

        Ok(match self.protocol {
            // Configure the transport to use regular HTTP requests
            Protocol::Http => OtlpTransport::Http {
                http: HttpConnection::http1(
                    metrics.clone(),
                    url,
                    self.allow_compression,
                    self.headers,
                    |req| Ok(req),
                    move |res| {
                        let metrics = metrics.clone();

                        async move {
                            let status = res.http_status();

                            // A request is considered successful if it returns 2xx status code
                            if status >= 200 && status < 300 {
                                metrics.http_batch_sent.increment();

                                Ok(vec![])
                            } else {
                                metrics.http_batch_failed.increment();

                                Err(Error::msg(format_args!(
                                    "OTLP HTTP server responded {status}"
                                )))
                            }
                        }
                    },
                )?,
                resource,
                request_encoder,
            },
            // Configure the transport to use gRPC requests
            // These are mostly the same as regular HTTP requests, but use
            // a simple message framing protocol and carry status codes in a trailer
            // instead of the response status
            Protocol::Grpc => OtlpTransport::Http {
                http: HttpConnection::http2(
                    metrics.clone(),
                    url,
                    self.allow_compression,
                    self.headers,
                    |mut req| {
                        let content_type_header = match req.content_type_header() {
                            "application/x-protobuf" => "application/grpc+proto",
                            content_type => {
                                return Err(Error::msg(format_args!(
                                    "unsupported content type '{content_type}'"
                                )))
                            }
                        };

                        // Wrap the content in the gRPC frame protocol
                        // This is a simple length-prefixed format that uses
                        // 5 bytes to indicate the length and compression of the message
                        let len = (u32::try_from(req.content_payload_len()).unwrap()).to_be_bytes();

                        Ok(
                            // If the content is compressed then set the gRPC compression header byte for it
                            if let Some(compression) = req.take_content_encoding_header() {
                                req.with_content_type_header(content_type_header)
                                    .with_headers(match compression {
                                        "gzip" => &[("grpc-encoding", "gzip")],
                                        compression => {
                                            return Err(Error::msg(format_args!(
                                                "unsupported compression '{compression}'"
                                            )))
                                        }
                                    })
                                    .with_content_frame([1, len[0], len[1], len[2], len[3]])
                            }
                            // If the content is not compressed then leave the gRPC compression header byte unset
                            else {
                                req.with_content_type_header(content_type_header)
                                    .with_content_frame([0, len[0], len[1], len[2], len[3]])
                            },
                        )
                    },
                    move |res| {
                        let metrics = metrics.clone();

                        async move {
                            let mut status = 0;
                            let mut msg = String::new();

                            res.stream_payload(
                                |_| {},
                                |k, v| match k {
                                    "grpc-status" => {
                                        status = v.parse().unwrap_or(0);
                                    }
                                    "grpc-message" => {
                                        msg = v.into();
                                    }
                                    _ => {}
                                },
                            )
                            .await?;

                            // A request is considered successful if the grpc-status trailer is 0
                            if status == 0 {
                                metrics.grpc_batch_sent.increment();

                                Ok(vec![])
                            }
                            // In any other case the request failed and may carry some diagnostic message
                            else {
                                metrics.grpc_batch_failed.increment();

                                if msg.len() > 0 {
                                    Err(Error::msg(format_args!(
                                        "OTLP gRPC server responded {status} {msg}"
                                    )))
                                } else {
                                    Err(Error::msg(format_args!(
                                        "OTLP gRPC server responded {status}"
                                    )))
                                }
                            }
                        }
                    },
                )?,
                resource,
                request_encoder,
            },
        })
    }
}

#[derive(Clone)]
struct OtlpClient {
    logs: Option<Arc<OtlpTransport<LogsRequestEncoder>>>,
    traces: Option<Arc<OtlpTransport<TracesRequestEncoder>>>,
    metrics: Option<Arc<OtlpTransport<MetricsRequestEncoder>>>,
}

enum OtlpTransport<R> {
    Http {
        http: HttpConnection,
        resource: Option<EncodedPayload>,
        request_encoder: ClientRequestEncoder<R>,
    },
}

impl<R: data::RequestEncoder> OtlpTransport<R> {
    #[emit::span(rt: emit::runtime::internal(), arg: span, "send OTLP batch of {batch_size} events", batch_size: batch.total_items())]
    pub(crate) async fn send(
        &self,
        batch: EncodedScopeItems,
    ) -> Result<(), BatchError<EncodedScopeItems>> {
        match self {
            OtlpTransport::Http {
                ref http,
                ref resource,
                ref request_encoder,
            } => {
                let uri = http.uri();
                let batch_size = batch.total_items();

                match http
                    .send(request_encoder.encode_request(resource.as_ref(), &batch)?)
                    .await
                {
                    Ok(res) => {
                        span.complete_with(|span| {
                            emit::debug!(
                                rt: emit::runtime::internal(),
                                event: span,
                                "OTLP batch of {batch_size} events to {uri}",
                                batch_size,
                            )
                        });

                        res
                    }
                    Err(err) => {
                        span.complete_with(|span| {
                            emit::warn!(
                                rt: emit::runtime::internal(),
                                event: span,
                                "OTLP batch of {batch_size} events to {uri} failed: {err}",
                                batch_size,
                                err,
                            )
                        });

                        return Err(BatchError::retry(err, batch));
                    }
                };
            }
        }

        Ok(())
    }
}

impl emit::emitter::Emitter for Otlp {
    fn emit<E: emit::event::ToEvent>(&self, evt: E) {
        let evt = evt.to_event();

        if let Some(ref encoder) = self.otlp_metrics {
            if let Some(encoded) = encoder.encode_event(&evt) {
                return self.sender.send(ChannelItem::Metric(encoded));
            }
        }

        if let Some(ref encoder) = self.otlp_traces {
            if let Some(encoded) = encoder.encode_event(&evt) {
                return self.sender.send(ChannelItem::Span(encoded));
            }
        }

        if let Some(ref encoder) = self.otlp_logs {
            if let Some(encoded) = encoder.encode_event(&evt) {
                return self.sender.send(ChannelItem::LogRecord(encoded));
            }
        }

        self.metrics.event_discarded.increment();
    }

    fn blocking_flush(&self, timeout: Duration) -> bool {
        emit_batcher::tokio::blocking_flush(&self.sender, timeout)
    }
}

#[derive(Default)]
pub(crate) struct Channel {
    otlp_logs: EncodedScopeItems,
    otlp_traces: EncodedScopeItems,
    otlp_metrics: EncodedScopeItems,
}

pub(crate) enum ChannelItem {
    LogRecord(EncodedEvent),
    Span(EncodedEvent),
    Metric(EncodedEvent),
}

impl emit_batcher::Channel for Channel {
    type Item = ChannelItem;

    fn new() -> Self {
        Channel {
            otlp_logs: EncodedScopeItems::new(),
            otlp_traces: EncodedScopeItems::new(),
            otlp_metrics: EncodedScopeItems::new(),
        }
    }

    fn push(&mut self, item: Self::Item) {
        match item {
            ChannelItem::LogRecord(item) => self.otlp_logs.push(item),
            ChannelItem::Span(item) => self.otlp_traces.push(item),
            ChannelItem::Metric(item) => self.otlp_metrics.push(item),
        }
    }

    fn len(&self) -> usize {
        let Channel {
            otlp_logs: logs,
            otlp_traces: traces,
            otlp_metrics: metrics,
        } = self;

        logs.total_items() + traces.total_items() + metrics.total_items()
    }

    fn clear(&mut self) {
        let Channel {
            otlp_logs: logs,
            otlp_traces: traces,
            otlp_metrics: metrics,
        } = self;

        logs.clear();
        traces.clear();
        metrics.clear();
    }
}

struct Resource {
    attributes: HashMap<emit::Str<'static>, emit::value::OwnedValue>,
}

enum Protocol {
    Http,
    Grpc,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum Encoding {
    Proto,
    Json,
}

impl Encoding {
    pub fn of(buf: &EncodedPayload) -> Self {
        match buf {
            EncodedPayload::Proto(_) => Encoding::Proto,
            EncodedPayload::Json(_) => Encoding::Json,
        }
    }
}

struct ClientEventEncoder<E> {
    encoding: Encoding,
    encoder: E,
}

impl<E> ClientEventEncoder<E> {
    pub fn new(encoding: Encoding, encoder: E) -> Self {
        ClientEventEncoder { encoding, encoder }
    }
}

impl<E: data::EventEncoder> ClientEventEncoder<E> {
    pub fn encode_event(
        &self,
        evt: &emit::event::Event<impl emit::props::Props>,
    ) -> Option<EncodedEvent> {
        match self.encoding {
            Encoding::Proto => self.encoder.encode_event::<data::Proto>(evt),
            Encoding::Json => self.encoder.encode_event::<data::Json>(evt),
        }
    }
}

struct ClientRequestEncoder<R> {
    encoding: Encoding,
    encoder: R,
}

impl<R> ClientRequestEncoder<R> {
    pub fn new(encoding: Encoding, encoder: R) -> Self {
        ClientRequestEncoder { encoding, encoder }
    }
}

impl<R: data::RequestEncoder> ClientRequestEncoder<R> {
    pub fn encode_request(
        &self,
        resource: Option<&EncodedPayload>,
        items: &EncodedScopeItems,
    ) -> Result<EncodedPayload, BatchError<EncodedScopeItems>> {
        match self.encoding {
            Encoding::Proto => self
                .encoder
                .encode_request::<data::Proto>(resource, items)
                .map_err(BatchError::no_retry),
            Encoding::Json => self
                .encoder
                .encode_request::<data::Json>(resource, items)
                .map_err(BatchError::no_retry),
        }
    }
}

fn encode_resource(encoding: Encoding, resource: &Resource) -> EncodedPayload {
    let attributes = data::PropsResourceAttributes(&resource.attributes);

    let resource = data::Resource {
        attributes: &attributes,
    };

    match encoding {
        Encoding::Proto => data::Proto::encode(&resource),
        Encoding::Json => data::Json::encode(&resource),
    }
}
