use std::sync::Arc;

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
        otlp_logs: Option<(
            ClientEventEncoder<LogsEventEncoder>,
            emit_batcher::Sender<Channel>,
        )>,
        process_otlp_logs: Option<(
            OtlpTransport<LogsRequestEncoder>,
            emit_batcher::Receiver<Channel>,
        )>,
        otlp_traces: Option<(
            ClientEventEncoder<TracesEventEncoder>,
            emit_batcher::Sender<Channel>,
        )>,
        process_otlp_traces: Option<(
            OtlpTransport<TracesRequestEncoder>,
            emit_batcher::Receiver<Channel>,
        )>,
        otlp_metrics: Option<(
            ClientEventEncoder<MetricsEventEncoder>,
            emit_batcher::Sender<Channel>,
        )>,
        process_otlp_metrics: Option<(
            OtlpTransport<MetricsRequestEncoder>,
            emit_batcher::Receiver<Channel>,
        )>,
        metrics: Arc<InternalMetrics>,
    ) -> Result<OtlpInner, Error> {
        todo!()
    }
}
