use std::{sync::Arc, time::Duration};

use wasm_bindgen::prelude::*;

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
        // Spawn the processors as fire-and-forget promises
        if let Some((transport, receiver)) = process_otlp_logs {
            let transport = Arc::new(transport);

            emit_batcher::web::spawn(receiver, move |batch| {
                let transport = transport.clone();

                async move { transport.send(batch).await }
            })
            .map_err(|e| Error::new("failed to spawn logs transport", e))?;
        }

        if let Some((transport, receiver)) = process_otlp_traces {
            let transport = Arc::new(transport);

            emit_batcher::web::spawn(receiver, move |batch| {
                let transport = transport.clone();

                async move { transport.send(batch).await }
            })
            .map_err(|e| Error::new("failed to spawn traces transport", e))?;
        }

        if let Some((transport, receiver)) = process_otlp_metrics {
            let transport = Arc::new(transport);

            emit_batcher::web::spawn(receiver, move |batch| {
                let transport = transport.clone();

                async move { transport.send(batch).await }
            })
            .map_err(|e| Error::new("failed to spawn metrics transport", e))?;
        }

        Ok(OtlpInner {
            otlp_logs,
            otlp_traces,
            otlp_metrics,
            metrics,
            _handle: (),
        })
    }
}

pub(crate) async fn flush(sender: &emit_batcher::Sender<Channel>, timeout: Duration) -> bool {
    emit_batcher::web::flush(sender, timeout).await
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = performance, js_name = "now")]
    pub fn performance_now() -> f64;
}
