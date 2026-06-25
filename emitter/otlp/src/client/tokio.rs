use std::{future::Future, pin::Pin, sync::Arc, time::Duration};

use futures_util::{StreamExt, stream::FuturesUnordered};

use crate::{
    Error,
    client::{Channel, OtlpBuilder, OtlpInner, SignalSenders, SignalWorker},
    data::{
        logs::{LogsEventEncoder, LogsRequestEncoder},
        metrics::{MetricsEventEncoder, MetricsRequestEncoder},
        traces::{TracesEventEncoder, TracesRequestEncoder},
    },
    internal_metrics::InternalMetrics,
};

pub(super) type Handle = std::thread::JoinHandle<()>;

impl OtlpBuilder {
    pub(super) fn try_spawn_inner_imp(
        otlp_logs: Option<emit_batcher::Sender<Channel>>,
        worker_logs: Option<SignalWorker<LogsEventEncoder, LogsRequestEncoder>>,
        otlp_traces: Option<emit_batcher::Sender<Channel>>,
        worker_traces: Option<SignalWorker<TracesEventEncoder, TracesRequestEncoder>>,
        otlp_metrics: Option<emit_batcher::Sender<Channel>>,
        worker_metrics: Option<SignalWorker<MetricsEventEncoder, MetricsRequestEncoder>>,
        metrics: Arc<InternalMetrics>,
    ) -> Result<OtlpInner, Error> {
        let receive = async move {
            let processors =
                FuturesUnordered::<Pin<Box<dyn Future<Output = ()> + Send + 'static>>>::new();

            if let Some(worker) = worker_logs {
                let (inner, receiver) = worker.into_receiver();
                processors.push(Box::pin(emit_batcher::tokio::exec(
                    receiver,
                    move |batch| {
                        let inner = inner.clone();
                        async move {
                            inner
                                .transport
                                .send::<LogsEventEncoder>(&inner.event_encoder, batch)
                                .await
                        }
                    },
                )));
            }

            if let Some(worker) = worker_traces {
                let (inner, receiver) = worker.into_receiver();
                processors.push(Box::pin(emit_batcher::tokio::exec(
                    receiver,
                    move |batch| {
                        let inner = inner.clone();
                        async move {
                            inner
                                .transport
                                .send::<TracesEventEncoder>(&inner.event_encoder, batch)
                                .await
                        }
                    },
                )));
            }

            if let Some(worker) = worker_metrics {
                let (inner, receiver) = worker.into_receiver();
                processors.push(Box::pin(emit_batcher::tokio::exec(
                    receiver,
                    move |batch| {
                        let inner = inner.clone();
                        async move {
                            inner
                                .transport
                                .send::<MetricsEventEncoder>(&inner.event_encoder, batch)
                                .await
                        }
                    },
                )));
            }

            let _ = processors.into_future().await;
        };

        let handle = std::thread::Builder::new()
            .name("emit_otlp_worker".into())
            .spawn(move || {
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap()
                    .block_on(receive);
            })
            .map_err(|e| Error::new("failed to spawn background worker", e))?;

        Ok(OtlpInner {
            signals: SignalSenders::new(otlp_logs, otlp_traces, otlp_metrics),
            metrics,
            handle: Some(handle),
        })
    }
}

pub(crate) async fn flush(sender: &emit_batcher::Sender<Channel>, timeout: Duration) -> bool {
    emit_batcher::tokio::flush(sender, timeout).await
}
