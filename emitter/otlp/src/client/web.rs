use std::{sync::Arc, time::Duration};

use wasm_bindgen::prelude::*;

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
        otlp_logs: Option<emit_batcher::Sender<Channel>>,
        worker_logs: Option<SignalWorker<HttpConnection, LogsEventEncoder, LogsRequestEncoder>>,
        otlp_traces: Option<emit_batcher::Sender<Channel>>,
        worker_traces: Option<
            SignalWorker<HttpConnection, TracesEventEncoder, TracesRequestEncoder>,
        >,
        otlp_metrics: Option<emit_batcher::Sender<Channel>>,
        worker_metrics: Option<
            SignalWorker<HttpConnection, MetricsEventEncoder, MetricsRequestEncoder>,
        >,
        metrics: Arc<InternalMetrics>,
    ) -> Result<OtlpInner, Error> {
        let _ = metrics;

        if let Some(worker) = worker_logs {
            emit_batcher::web::spawn(worker.receiver, move |batch| {
                let transport = worker.transport.clone();
                async move { transport.send(batch).await }
            })
            .map_err(|e| Error::new("failed to spawn logs transport", e))?;
        }

        if let Some(worker) = worker_traces {
            emit_batcher::web::spawn(worker.receiver, move |batch| {
                let transport = worker.transport.clone();
                async move { transport.send(batch).await }
            })
            .map_err(|e| Error::new("failed to spawn traces transport", e))?;
        }

        if let Some(worker) = worker_metrics {
            emit_batcher::web::spawn(worker.receiver, move |batch| {
                let transport = worker.transport.clone();
                async move { transport.send(batch).await }
            })
            .map_err(|e| Error::new("failed to spawn metrics transport", e))?;
        }

        Ok(OtlpInner {
            signals: SignalSenders::new(otlp_logs, otlp_traces, otlp_metrics),
            metrics,
            handle: None,
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
