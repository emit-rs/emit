/*!
Configuration and network client for the OTLP emitter.

This module is a consumer of `data`, using it to encode incoming events. These are then sent to a remote collector by the `http` module.
*/

use crate::{
    Error, OtlpMetrics,
    data::{self, EncodedEvent, EncodedPayload, EncodedScopeItems, RawEncoder},
    internal_metrics::InternalMetrics,
};
use emit::Filter;
use emit_batcher::BatchError;
use std::{collections::HashMap, fmt, future::Future, sync::Arc, time::Duration};

use self::{
    http::{HttpConnection, HttpVersion},
    imp::Handle,
};

#[cfg(not(all(
    target_arch = "wasm32",
    target_vendor = "unknown",
    target_os = "unknown"
)))]
#[path = "client/tokio.rs"]
mod imp;

#[cfg(all(
    feature = "web",
    target_arch = "wasm32",
    target_vendor = "unknown",
    target_os = "unknown"
))]
#[path = "client/web.rs"]
mod imp;

#[cfg(all(
    not(feature = "web"),
    target_arch = "wasm32",
    target_vendor = "unknown",
    target_os = "unknown"
))]
#[path = "client/stub.rs"]
mod imp;

mod channel;
mod http;
mod logs;
mod metrics;
mod traces;

pub(crate) use self::channel::{Channel, ChannelItem, SignalWorker};
pub use self::{logs::*, metrics::*, traces::*};

const DEFAULT_MAX_REQUEST_SIZE_BYTES: usize = 1024 * 1024; // 1MiB
const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/**
An [`emit::Emitter`] that sends diagnostic events via the OpenTelemetry Protocol (OTLP).

Use [`crate::new`] to start an [`OtlpBuilder`] for configuring an [`Otlp`] instance.

See the crate root documentation for more details.
*/
pub struct Otlp {
    inner: Option<OtlpInner>,
    metrics: Arc<InternalMetrics>,
}

struct OtlpInner {
    signals: SignalSenders,
    metrics: Arc<InternalMetrics>,
    #[allow(dead_code)]
    handle: Option<Handle>,
}

/** Manages the senders for all configured OTLP signals. */
pub(crate) struct SignalSenders {
    logs: Option<emit_batcher::Sender<Channel>>,
    traces: Option<emit_batcher::Sender<Channel>>,
    metrics: Option<emit_batcher::Sender<Channel>>,
}

impl SignalSenders {
    fn new(
        logs: Option<emit_batcher::Sender<Channel>>,
        traces: Option<emit_batcher::Sender<Channel>>,
        metrics: Option<emit_batcher::Sender<Channel>>,
    ) -> Self {
        SignalSenders {
            logs,
            traces,
            metrics,
        }
    }

    fn configured_count(&self) -> u32 {
        self.logs.as_ref().map(|_| 1).unwrap_or_default()
            + self.traces.as_ref().map(|_| 1).unwrap_or_default()
            + self.metrics.as_ref().map(|_| 1).unwrap_or_default()
    }

    fn metric_sources(
        &self,
    ) -> (
        Option<emit_batcher::ChannelMetrics<Channel>>,
        Option<emit_batcher::ChannelMetrics<Channel>>,
        Option<emit_batcher::ChannelMetrics<Channel>>,
    ) {
        (
            self.logs.as_ref().map(|s| s.metric_source()),
            self.traces.as_ref().map(|s| s.metric_source()),
            self.metrics.as_ref().map(|s| s.metric_source()),
        )
    }
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
        let (logs, traces, metrics) = self
            .inner
            .as_ref()
            .map(|i| i.signals.metric_sources())
            .unwrap_or((None, None, None));
        OtlpMetrics {
            logs_channel_metrics: logs,
            traces_channel_metrics: traces,
            metrics_channel_metrics: metrics,
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

        let _ = attributes.for_each(|k, v| {
            resource.attributes.insert(k.to_owned(), v.to_owned());

            std::ops::ControlFlow::Continue(())
        });

        self.resource = Some(resource);

        self
    }

    /**
    Try spawn an [`Otlp`] instance which can be used to send diagnostic events via OTLP.

    If any configured values are invalid, such as malformed URIs, this method won't fail or panic. It will discard any events emitted to it. In these cases it will log to [`emit::runtime::internal`] and increment the `configuration_failed` metric on [`Otlp::metric_source`]. See the _Troubleshooting_ section of the crate root docs for more details.
    */
    pub fn spawn(self) -> Otlp {
        let metrics = Arc::new(InternalMetrics::default());

        let inner = match self.try_spawn_inner(metrics.clone()) {
            Ok(inner) => Some(inner),
            Err(err) => {
                emit::error!(
                    rt: emit::runtime::internal(),
                    "OTLP configuration is invalid; no events will be written: {err}"
                );

                metrics.configuration_failed.increment();

                None
            }
        };

        Otlp { metrics, inner }
    }

    fn try_spawn_inner(self, metrics: Arc<InternalMetrics>) -> Result<OtlpInner, Error> {
        let (otlp_logs, worker_logs) = match self.otlp_logs {
            Some(builder) => {
                let transport = builder.build(metrics.clone(), self.resource.as_ref())?;
                let (sender, receiver) = emit_batcher::bounded(10_000);
                let worker = SignalWorker::new(transport, receiver);
                (Some(sender), Some(worker))
            }
            None => (None, None),
        };

        let (otlp_traces, worker_traces) = match self.otlp_traces {
            Some(builder) => {
                let transport = builder.build(metrics.clone(), self.resource.as_ref())?;
                let (sender, receiver) = emit_batcher::bounded(10_000);
                let worker = SignalWorker::new(transport, receiver);
                (Some(sender), Some(worker))
            }
            None => (None, None),
        };

        let (otlp_metrics, worker_metrics) = match self.otlp_metrics {
            Some(builder) => {
                let transport = builder.build(metrics.clone(), self.resource.as_ref())?;
                let (sender, receiver) = emit_batcher::bounded(10_000);
                let worker = SignalWorker::new(transport, receiver);
                (Some(sender), Some(worker))
            }
            None => (None, None),
        };

        Self::try_spawn_inner_imp(
            otlp_logs,
            worker_logs,
            otlp_traces,
            worker_traces,
            otlp_metrics,
            worker_metrics,
            metrics,
        )
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

    fn build<E, R>(
        self,
        metrics: Arc<InternalMetrics>,
        event_encoder: ClientEventEncoder<E>,
        resource: Option<EncodedPayload>,
        request_encoder: ClientRequestEncoder<R>,
    ) -> Result<OtlpTransport<HttpConnection, E, R>, Error> {
        let mut url = self.url_base;

        if let Some(path) = self.url_path {
            crate::push_path(&mut url, path);
        }

        let request_sender = match self.protocol {
            // Configure the transport to use regular HTTP requests
            Protocol::Http => HttpConnection::new(
                HttpVersion::Http1,
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

                            Ok(())
                        } else {
                            metrics.http_batch_failed.increment();

                            Err(Error::msg(format_args!(
                                "OTLP HTTP server responded {status}"
                            )))
                        }
                    }
                },
            )?,
            // Configure the transport to use gRPC requests
            // These are mostly the same as regular HTTP requests, but use
            // a simple message framing protocol and carry status codes in a trailer
            // instead of the response status
            Protocol::Grpc => HttpConnection::new(
                HttpVersion::Http2,
                metrics.clone(),
                url,
                self.allow_compression,
                self.headers,
                |req| {
                    let content_type_header = match req.content_type_header() {
                        "application/x-protobuf" => "application/grpc+proto",
                        content_type => {
                            return Err(Error::msg(format_args!(
                                "unsupported content type '{content_type}'"
                            )));
                        }
                    };

                    // Wrap the content in the gRPC frame protocol
                    // This is a simple length-prefixed format that uses
                    // 5 bytes to indicate the length and compression of the message
                    let len = (u32::try_from(req.content_payload_len()).unwrap()).to_be_bytes();

                    Ok(
                        // If the content is compressed then set the gRPC compression header byte for it
                        if let Some(compression) = req.content_encoding_header() {
                            req.with_content_encoding_header(None)
                                .with_content_type_header(content_type_header)
                                .with_headers(match compression {
                                    "gzip" => &[("grpc-encoding", "gzip")],
                                    compression => {
                                        return Err(Error::msg(format_args!(
                                            "unsupported compression '{compression}'"
                                        )));
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

                        res.stream_trailers(|k, v| match k {
                            "grpc-status" => {
                                status = v.parse().unwrap_or(0);
                            }
                            "grpc-message" => {
                                msg = v.into();
                            }
                            _ => {}
                        })
                        .await?;

                        // A request is considered successful if the grpc-status trailer is 0
                        if status == 0 {
                            metrics.grpc_batch_sent.increment();

                            Ok(())
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
        };

        Ok(OtlpTransport {
            event_encoder,
            request_sender,
            resource,
            request_encoder,
        })
    }
}

pub(crate) struct OtlpTransport<S, E, R> {
    event_encoder: ClientEventEncoder<E>,
    request_sender: S,
    resource: Option<EncodedPayload>,
    request_encoder: ClientRequestEncoder<R>,
}

impl<S: ClientRequestSender, E: data::EventEncoder, R: data::RequestEncoder>
    OtlpTransport<S, E, R>
{
    pub(crate) async fn send(&self, channel: Channel) -> Result<(), BatchError<Channel>> {
        let event_encoder = &self.event_encoder;
        channel::execute(
            channel,
            DEFAULT_MAX_REQUEST_SIZE_BYTES,
            |event| event_encoder.encode_event(event),
            |batch| {
                let batch = batch.clone();
                async move {
                    Self::send_batch(
                        &self.request_sender,
                        &self.resource,
                        &self.request_encoder,
                        &batch,
                    )
                    .await
                }
            },
        )
        .await
    }

    #[emit::span(rt: emit::runtime::internal(), guard: span, "send OTLP batch of {batch_size} events to {uri}", batch_size: batch.total_items(), uri: request_sender.uri())]
    pub(crate) async fn send_batch(
        request_sender: &S,
        resource: &Option<EncodedPayload>,
        request_encoder: &ClientRequestEncoder<R>,
        batch: &EncodedScopeItems,
    ) -> Result<(), BatchError<()>> {
        let uri = request_sender.uri();
        let batch_size = batch.total_items();

        match request_sender
            .send(
                request_encoder.encode_request(resource.as_ref(), &batch)?,
                DEFAULT_REQUEST_TIMEOUT,
            )
            .await
        {
            Ok(res) => {
                span.complete_with(emit::span::completion::from_fn(|evt| {
                    emit::debug!(
                        rt: emit::runtime::internal(),
                        evt,
                        "OTLP batch of {batch_size} events to {uri}",
                        batch_size,
                    )
                }));

                res
            }
            Err(err) => {
                span.complete_with(emit::span::completion::from_fn(|evt| {
                    emit::warn!(
                        rt: emit::runtime::internal(),
                        evt,
                        "OTLP batch of {batch_size} events to {uri} failed: {err}",
                        batch_size,
                        err,
                    )
                }));

                return Err(BatchError::retry(err, ()));
            }
        };

        Ok(())
    }
}

impl Otlp {
    /**
    Wait for up to `timeout` for any pending events to be sent to the backend OTLP service.

    This method is an alternative to [`emit::Emitter::blocking_flush`].
    */
    pub async fn flush(&self, timeout: Duration) -> bool {
        if let Some(ref inner) = self.inner {
            inner.flush(timeout).await
        } else {
            true
        }
    }
}

impl emit::Emitter for Otlp {
    fn emit<E: emit::event::ToEvent>(&self, evt: E) {
        self.inner.emit(evt)
    }

    fn blocking_flush(&self, timeout: Duration) -> bool {
        self.inner.blocking_flush(timeout)
    }
}

impl OtlpInner {
    fn configured_signals(&self) -> u32 {
        self.signals.configured_count()
    }

    async fn flush(&self, timeout: Duration) -> bool {
        let timeout = timeout / self.configured_signals();
        let mut fully_flushed = true;

        if let Some(ref sender) = self.signals.logs {
            if !imp::flush(sender, timeout).await {
                fully_flushed = false;
            }
        }

        if let Some(ref sender) = self.signals.traces {
            if !imp::flush(sender, timeout).await {
                fully_flushed = false;
            }
        }

        if let Some(ref sender) = self.signals.metrics {
            if !imp::flush(sender, timeout).await {
                fully_flushed = false;
            }
        }

        fully_flushed
    }

    /** Take the background handle, consuming the inner.
    Used by tests to verify the worker thread shuts down cleanly. */
    #[allow(dead_code)]
    fn take_handle(&mut self) -> Handle {
        self.handle.take().expect("handle already taken")
    }
}

impl emit::Emitter for OtlpInner {
    fn emit<E: emit::event::ToEvent>(&self, evt: E) {
        let owned = evt.to_event().to_owned();

        // Check metrics first (highest priority)
        if let Some(ref sender) = self.signals.metrics {
            if emit::kind::is_metric_filter().matches(&owned) {
                sender.send(ChannelItem { event: owned });
                return;
            }
        }

        // Check traces second
        if let Some(ref sender) = self.signals.traces {
            if emit::kind::is_span_filter().matches(&owned) {
                sender.send(ChannelItem { event: owned });
                return;
            }
        }

        // Logs is the fallback — accepts all events
        if let Some(ref sender) = self.signals.logs {
            sender.send(ChannelItem { event: owned });
            return;
        }

        self.metrics.event_discarded.increment();
    }

    /**
    Wait for up to `timeout` for any pending events to be sent to the backend OTLP service.
    */
    fn blocking_flush(&self, timeout: Duration) -> bool {
        let timeout = timeout / self.configured_signals();
        let mut fully_flushed = true;

        if let Some(ref sender) = self.signals.logs {
            if !emit_batcher::blocking_flush(sender, timeout) {
                fully_flushed = false;
            }
        }

        if let Some(ref sender) = self.signals.traces {
            if !emit_batcher::blocking_flush(sender, timeout) {
                fully_flushed = false;
            }
        }

        if let Some(ref sender) = self.signals.metrics {
            if !emit_batcher::blocking_flush(sender, timeout) {
                fully_flushed = false;
            }
        }

        fully_flushed
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

pub(crate) struct ClientEventEncoder<E> {
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

pub(crate) trait ClientRequestSender {
    fn uri(&self) -> &(impl fmt::Display + 'static);

    fn send(
        &self,
        body: EncodedPayload,
        timeout: Duration,
    ) -> impl Future<Output = Result<(), Error>> + Send;
}

pub(crate) struct ClientRequestEncoder<R> {
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
    ) -> Result<EncodedPayload, BatchError<()>> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(not(all(
        target_arch = "wasm32",
        target_vendor = "unknown",
        target_os = "unknown"
    )))]
    fn otlp_empty_closes_bg_thread_on_drop() {
        let mut otlp = Otlp::builder().spawn();

        let handle = {
            let mut inner = otlp.inner.take().unwrap();
            inner.take_handle()
        };

        drop(otlp);

        // Ensure the background thread is torn down
        handle.join().unwrap();
    }

    #[test]
    #[cfg(not(all(
        target_arch = "wasm32",
        target_vendor = "unknown",
        target_os = "unknown"
    )))]
    fn otlp_non_empty_closes_bg_thread_on_drop() {
        let mut otlp = Otlp::builder()
            .logs(OtlpLogsBuilder::proto(OtlpTransportBuilder::http(
                "http://localhost:4319",
            )))
            .spawn();

        let handle = {
            let mut inner = otlp.inner.take().unwrap();
            inner.take_handle()
        };

        drop(otlp);

        // Ensure the background thread is torn down
        handle.join().unwrap();
    }
}
