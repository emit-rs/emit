use std::{future::Future, pin::Pin, sync::Arc};

use futures_util::{stream::FuturesUnordered, StreamExt};

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

pub(super) type Handle = std::thread::JoinHandle<()>;

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
        let receive = async move {
            let processors =
                FuturesUnordered::<Pin<Box<dyn Future<Output = ()> + Send + 'static>>>::new();

            if let Some((transport, receiver)) = process_otlp_logs {
                let transport = Arc::new(transport);

                processors.push(Box::pin(emit_batcher::tokio::exec(
                    receiver,
                    move |batch| {
                        let transport = transport.clone();

                        async move { transport.send(batch).await }
                    },
                )));
            }

            if let Some((transport, receiver)) = process_otlp_traces {
                let transport = Arc::new(transport);

                processors.push(Box::pin(emit_batcher::tokio::exec(
                    receiver,
                    move |batch| {
                        let transport = transport.clone();

                        async move { transport.send(batch).await }
                    },
                )));
            }

            if let Some((transport, receiver)) = process_otlp_metrics {
                let transport = Arc::new(transport);

                processors.push(Box::pin(emit_batcher::tokio::exec(
                    receiver,
                    move |batch| {
                        let transport = transport.clone();

                        async move { transport.send(batch).await }
                    },
                )));
            }

            // Process batches from each signal independently
            // This ensures one signal becoming unavailable doesn't
            // block the others
            let _ = processors.into_future().await;
        };

        // Spawn a background thread to process batches
        // This is a safe way to ensure users of `Otlp` can never
        // deadlock waiting on the processing of batches
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
            otlp_logs,
            otlp_traces,
            otlp_metrics,
            metrics,
            _handle: handle,
        })
    }
}
