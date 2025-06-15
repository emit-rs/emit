compile_error!("unsupported target");

use std::{sync::Arc, time::Duration};

use crate::{
    client::{Channel, ClientEventEncoder, OtlpBuilder, OtlpInner, OtlpTransport},
    data::{
        logs::{LogsEventEncoder, LogsRequestEncoder},
        metrics::{MetricsEventEncoder, MetricsRequestEncoder},
        traces::{TracesEventEncoder, TracesRequestEncoder},
    },
    internal_metrics::InternalMetrics,
    Error,
};

pub(super) type Handle = ();

impl OtlpBuilder {
    pub(super) fn try_spawn_inner_imp(
        _otlp_logs: Option<(
            ClientEventEncoder<LogsEventEncoder>,
            emit_batcher::Sender<Channel>,
        )>,
        _process_otlp_logs: Option<(
            OtlpTransport<LogsRequestEncoder>,
            emit_batcher::Receiver<Channel>,
        )>,
        _otlp_traces: Option<(
            ClientEventEncoder<TracesEventEncoder>,
            emit_batcher::Sender<Channel>,
        )>,
        _process_otlp_traces: Option<(
            OtlpTransport<TracesRequestEncoder>,
            emit_batcher::Receiver<Channel>,
        )>,
        _otlp_metrics: Option<(
            ClientEventEncoder<MetricsEventEncoder>,
            emit_batcher::Sender<Channel>,
        )>,
        _process_otlp_metrics: Option<(
            OtlpTransport<MetricsRequestEncoder>,
            emit_batcher::Receiver<Channel>,
        )>,
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
