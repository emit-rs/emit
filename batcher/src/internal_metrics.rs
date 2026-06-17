use std::sync::atomic::{AtomicUsize, Ordering};

macro_rules! metrics {
    ($container:ident {
        $($name:ident: $ty:ty,)*
    }) => {
        #[derive(Default)]
        pub(crate) struct $container { $(pub(crate) $name: $ty),* }

        impl emit::metric::Source for $container {
            fn sample_metrics<S: emit::metric::Sampler>(&self, sampler: S) {
                let $container { $($name),* } = self;

                $(
                    sampler.metric(
                        emit::metric::Metric::new(
                            emit::pkg!(),
                            emit::Empty,
                            emit::props! {
                                metric_name: stringify!($name),
                                metric_agg: <$ty>::AGG,
                                metric_value: $name.sample(),
                            },
                        ),
                    );
                )*
            }
        }
    };
}

#[derive(Default)]
pub(crate) struct Counter(AtomicUsize);

impl Counter {
    const AGG: &'static str = emit::well_known::METRIC_AGG_COUNT;

    pub fn increment(&self) {
        self.increment_by(1);
    }

    pub fn increment_by(&self, by: usize) {
        self.0.fetch_add(by, Ordering::Relaxed);
    }

    pub fn sample(&self) -> usize {
        self.0.load(Ordering::Relaxed)
    }
}

metrics!(InternalMetrics {
    queue_full_truncated: Counter,
    queue_full_blocked: Counter,
    queue_batch_processed: Counter,
    queue_batch_failed: Counter,
    queue_batch_panicked: Counter,
    queue_batch_retry: Counter,
});
