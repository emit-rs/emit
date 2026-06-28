compile_error!("unsupported target");

use std::{sync::Arc, time::Duration};

use crate::{
    Error,
    client::http::HttpConnection,
    client::{Channel, OtlpBuilder, OtlpInner, SignalSenders, SignalWorker},
    data::{
        logs::{LogsEventEncoder, LogsRequestEncoder},
        metrics::{MetricsEventEncoder, MetricsRequestEncoder},
        traces::{TracesEventEncoder, TracesRequestEncoder},
    },
    internal_metrics::InternalMetrics,
};

pub(super) type Handle = ();

impl OtlpBuilder {
    pub(super) fn try_spawn_inner_imp(
        _otlp_logs: Option<emit_batcher::Sender<Channel>>,
        _worker_logs: Option<SignalWorker<HttpConnection, LogsEventEncoder, LogsRequestEncoder>>,
        _otlp_traces: Option<emit_batcher::Sender<Channel>>,
        _worker_traces: Option<
            SignalWorker<HttpConnection, TracesEventEncoder, TracesRequestEncoder>,
        >,
        _otlp_metrics: Option<emit_batcher::Sender<Channel>>,
        _worker_metrics: Option<
            SignalWorker<HttpConnection, MetricsEventEncoder, MetricsRequestEncoder>,
        >,
        _metrics: Arc<InternalMetrics>,
    ) -> Result<OtlpInner, Error> {
        unreachable!()
    }
}

pub(crate) struct Instant;

impl Instant {
    pub fn now() -> Self {
        unreachable!()
    }

    pub fn elapsed(&self) -> Duration {
        unreachable!()
    }
}

pub(crate) async fn flush(_sender: &emit_batcher::Sender<Channel>, _timeout: Duration) -> bool {
    false
}
