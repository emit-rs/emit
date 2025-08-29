/*!
The [`Metric`] type.
*/

use core::{fmt, ops::ControlFlow};

use emit_core::{
    and::And,
    emitter::Emitter,
    event::{Event, ToEvent},
    extent::{Extent, ToExtent},
    or::Or,
    path::Path,
    props::{ErasedProps, Props},
    str::{Str, ToStr},
    template::{self, Template},
    timestamp::Timestamp,
    value::{ToValue, Value},
    well_known::{KEY_EVT_KIND, KEY_METRIC_AGG, KEY_METRIC_NAME, KEY_METRIC_VALUE},
};

use crate::kind::Kind;

pub use self::{sampler::Sampler, source::Source};

/**
A diagnostic event that represents a metric sample.

Metrics are an extension of [`Event`]s that explicitly take the well-known properties that signal an event as being a metric sample. See the [`crate::metric`] module for details.

A `Metric` can be converted into an [`Event`] through its [`ToEvent`] implemenation, or passed directly to an [`Emitter`] to emit it.
*/
pub struct Metric<'a, P> {
    mdl: Path<'a>,
    name: Str<'a>,
    agg: Str<'a>,
    extent: Option<Extent>,
    tpl: Option<Template<'a>>,
    value: Value<'a>,
    props: P,
}

impl<'a, P> Metric<'a, P> {
    /**
    Create a new metric from its properties.

    Each metric consists of:

    - `mdl`: The module that owns the underlying data source.
    - `extent`: The [`Extent`] that the sample covers.
    - `name`: The name of the underlying data source.
    - `agg`: The aggregation applied to the underlying data source to produce the sample. See the [`crate::metric`] module for details.
    - `value`: The value of the sample itself.
    - `props`: Additional [`Props`] to associate with the sample.
    */
    pub fn new(
        mdl: impl Into<Path<'a>>,
        name: impl Into<Str<'a>>,
        agg: impl Into<Str<'a>>,
        extent: impl ToExtent,
        value: impl Into<Value<'a>>,
        props: P,
    ) -> Self {
        Metric {
            mdl: mdl.into(),
            extent: extent.to_extent(),
            tpl: None,
            name: name.into(),
            agg: agg.into(),
            value: value.into(),
            props,
        }
    }

    /**
    Get the module that owns the underlying data source.
    */
    pub fn mdl(&self) -> &Path<'a> {
        &self.mdl
    }

    /**
    Set the module of the underlying data source to a new value.
    */
    pub fn with_mdl(mut self, mdl: impl Into<Path<'a>>) -> Self {
        self.mdl = mdl.into();
        self
    }

    /**
    Get the name of the underlying data source.
    */
    pub fn name(&self) -> &Str<'a> {
        &self.name
    }

    /**
    Set the name of the underlying data source to a new value.
    */
    pub fn with_name(mut self, name: impl Into<Str<'a>>) -> Self {
        self.name = name.into();
        self
    }

    /**
    Get the aggregation applied to the underlying data source to produce the sample.

    The value of the aggregation should be one of the [`crate::well_known`] aggregation types.
    */
    pub fn agg(&self) -> &Str<'a> {
        &self.agg
    }

    /**
    Set the aggregation to a new value.

    The value of the aggregation should be one of the [`crate::well_known`] aggregation types.
    */
    pub fn with_agg(mut self, agg: impl Into<Str<'a>>) -> Self {
        self.agg = agg.into();
        self
    }

    /**
    Get the value of the sample itself.
    */
    pub fn value(&self) -> &Value<'a> {
        &self.value
    }

    /**
    Set the sample to a new value.
    */
    pub fn with_value(mut self, value: impl Into<Value<'a>>) -> Self {
        self.value = value.into();
        self
    }

    /**
    Get the extent for which the sample was generated.
    */
    pub fn extent(&self) -> Option<&Extent> {
        self.extent.as_ref()
    }

    /**
    Set the extent of the sample to a new value.
    */
    pub fn with_extent(mut self, extent: impl ToExtent) -> Self {
        self.extent = extent.to_extent();
        self
    }

    /**
    Get the extent of the metric as a point in time.

    If the metric has an extent then this method will return `Some`, with the result of [`Extent::as_point`]. If the metric doesn't have an extent then this method will return `None`.
    */
    pub fn ts(&self) -> Option<&Timestamp> {
        self.extent.as_ref().map(|extent| extent.as_point())
    }

    /**
    Get the start point of the extent of the metric.

    If the metric has an extent, and that extent covers a timespan then this method will return `Some`. Otherwise this method will return `None`.
    */
    pub fn ts_start(&self) -> Option<&Timestamp> {
        self.extent
            .as_ref()
            .and_then(|extent| extent.as_range())
            .map(|span| &span.start)
    }

    /**
    Get the template that will be used to render the metric.
    */
    pub fn tpl(&self) -> &Template<'a> {
        self.tpl.as_ref().unwrap_or(&TEMPLATE)
    }

    /**
    Set the template of the metric.
    */
    pub fn with_tpl(mut self, tpl: impl Into<Template<'a>>) -> Self {
        self.tpl = Some(tpl.into());
        self
    }

    /**
    Get the additional properties associated with the sample.
    */
    pub fn props(&self) -> &P {
        &self.props
    }

    /**
    Set the additional properties associated with the sample to a new value.
    */
    pub fn with_props<U>(self, props: U) -> Metric<'a, U> {
        Metric {
            mdl: self.mdl,
            extent: self.extent,
            tpl: self.tpl,
            name: self.name,
            agg: self.agg,
            value: self.value,
            props,
        }
    }

    /**
    Map the properties of the metric.
    */
    pub fn map_props<U>(self, map: impl FnOnce(P) -> U) -> Metric<'a, U> {
        Metric {
            mdl: self.mdl,
            extent: self.extent,
            tpl: self.tpl,
            name: self.name,
            agg: self.agg,
            value: self.value,
            props: map(self.props),
        }
    }
}

impl<'a, P: Props> fmt::Debug for Metric<'a, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.to_event(), f)
    }
}

impl<'a, P: Props> ToEvent for Metric<'a, P> {
    type Props<'b>
        = &'b Self
    where
        Self: 'b;

    fn to_event<'b>(&'b self) -> Event<'b, Self::Props<'b>> {
        Event::new(
            self.mdl.by_ref(),
            self.tpl().by_ref(),
            self.extent.clone(),
            self,
        )
    }
}

impl<'a, P: Props> Metric<'a, P> {
    /**
    Get a new metric sample, borrowing data from this one.
    */
    pub fn by_ref<'b>(&'b self) -> Metric<'b, &'b P> {
        Metric {
            mdl: self.mdl.by_ref(),
            extent: self.extent.clone(),
            tpl: self.tpl.as_ref().map(|tpl| tpl.by_ref()),
            name: self.name.by_ref(),
            agg: self.agg.by_ref(),
            value: self.value.by_ref(),
            props: &self.props,
        }
    }

    /**
    Get a type-erased metric sample, borrowing data from this one.
    */
    pub fn erase<'b>(&'b self) -> Metric<'b, &'b dyn ErasedProps> {
        Metric {
            mdl: self.mdl.by_ref(),
            extent: self.extent.clone(),
            tpl: self.tpl.as_ref().map(|tpl| tpl.by_ref()),
            name: self.name.by_ref(),
            agg: self.agg.by_ref(),
            value: self.value.by_ref(),
            props: &self.props,
        }
    }
}

impl<'a, P> ToExtent for Metric<'a, P> {
    fn to_extent(&self) -> Option<Extent> {
        self.extent.clone()
    }
}

impl<'a, P: Props> Props for Metric<'a, P> {
    fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        mut for_each: F,
    ) -> ControlFlow<()> {
        for_each(KEY_EVT_KIND.to_str(), Kind::Metric.to_value())?;
        for_each(KEY_METRIC_NAME.to_str(), self.name.to_value())?;
        for_each(KEY_METRIC_AGG.to_str(), self.agg.to_value())?;
        for_each(KEY_METRIC_VALUE.to_str(), self.value.by_ref())?;

        self.props.for_each(for_each)
    }
}

// "{metric_agg} of {metric_name} is {metric_value}"
const TEMPLATE_PARTS: &'static [template::Part<'static>] = &[
    template::Part::hole("metric_agg"),
    template::Part::text(" of "),
    template::Part::hole("metric_name"),
    template::Part::text(" is "),
    template::Part::hole("metric_value"),
];

static TEMPLATE: Template<'static> = Template::new(TEMPLATE_PARTS);

pub mod source {
    /*!
    The [`Source`] type.

    [`Source`]s produce [`Metric`]s on-demand. They can be sampled directly, or combined with a [`crate::metric::Reporter`] and sampled together.
    */

    use self::sampler::ErasedSampler;

    use super::*;

    /**
    A source of [`Metric`]s.
    */
    pub trait Source {
        /**
        Produce a current sample for all metrics in the source.
        */
        fn sample_metrics<S: sampler::Sampler>(&self, sampler: S);

        /**
        Chain this source to `other`, sampling metrics from both.
        */
        fn and_sample<U>(self, other: U) -> And<Self, U>
        where
            Self: Sized,
        {
            And::new(self, other)
        }
    }

    impl<'a, T: Source + ?Sized> Source for &'a T {
        fn sample_metrics<S: sampler::Sampler>(&self, sampler: S) {
            (**self).sample_metrics(sampler)
        }
    }

    impl<T: Source> Source for Option<T> {
        fn sample_metrics<S: sampler::Sampler>(&self, sampler: S) {
            if let Some(source) = self {
                source.sample_metrics(sampler);
            }
        }
    }

    #[cfg(feature = "alloc")]
    impl<'a, T: Source + ?Sized + 'a> Source for alloc::boxed::Box<T> {
        fn sample_metrics<S: sampler::Sampler>(&self, sampler: S) {
            (**self).sample_metrics(sampler)
        }
    }

    #[cfg(feature = "alloc")]
    impl<'a, T: Source + ?Sized + 'a> Source for alloc::sync::Arc<T> {
        fn sample_metrics<S: sampler::Sampler>(&self, sampler: S) {
            (**self).sample_metrics(sampler)
        }
    }

    impl<T: Source, U: Source> Source for And<T, U> {
        fn sample_metrics<S: sampler::Sampler>(&self, sampler: S) {
            self.left().sample_metrics(&sampler);
            self.right().sample_metrics(&sampler);
        }
    }

    impl<T: Source, U: Source> Source for Or<T, U> {
        fn sample_metrics<S: sampler::Sampler>(&self, sampler: S) {
            self.left().sample_metrics(&sampler);
            self.right().sample_metrics(&sampler);
        }
    }

    impl<'a, P: Props> Source for Metric<'a, P> {
        fn sample_metrics<S: Sampler>(&self, sampler: S) {
            sampler.metric(self.by_ref());
        }
    }

    /**
    A [`Source`] from a function.

    This type can be created directly, or via [`from_fn`].
    */
    pub struct FromFn<F = fn(&mut dyn ErasedSampler)>(F);

    /**
    Create a [`Source`] from a function.
    */
    pub const fn from_fn<F: Fn(&mut dyn ErasedSampler)>(source: F) -> FromFn<F> {
        FromFn::new(source)
    }

    impl<F> FromFn<F> {
        /**
        Wrap the given source function.
        */
        pub const fn new(source: F) -> Self {
            FromFn(source)
        }
    }

    impl<F: Fn(&mut dyn ErasedSampler)> Source for FromFn<F> {
        fn sample_metrics<S: sampler::Sampler>(&self, mut sampler: S) {
            (self.0)(&mut sampler)
        }
    }

    mod internal {
        use super::*;

        pub trait DispatchSource {
            fn dispatch_sample_metrics(&self, sampler: &dyn sampler::ErasedSampler);
        }

        pub trait SealedSource {
            fn erase_source(&self) -> crate::internal::Erased<&dyn DispatchSource>;
        }
    }

    /**
    An object-safe [`Source`].

    A `dyn ErasedSource` can be treated as `impl Source`.
    */
    pub trait ErasedSource: internal::SealedSource {}

    impl<T: Source> ErasedSource for T {}

    impl<T: Source> internal::SealedSource for T {
        fn erase_source(&self) -> crate::internal::Erased<&dyn internal::DispatchSource> {
            crate::internal::Erased(self)
        }
    }

    impl<T: Source> internal::DispatchSource for T {
        fn dispatch_sample_metrics(&self, sampler: &dyn sampler::ErasedSampler) {
            self.sample_metrics(sampler)
        }
    }

    impl<'a> Source for dyn ErasedSource + 'a {
        fn sample_metrics<S: sampler::Sampler>(&self, sampler: S) {
            self.erase_source().0.dispatch_sample_metrics(&sampler)
        }
    }

    impl<'a> Source for dyn ErasedSource + Send + Sync + 'a {
        fn sample_metrics<S: sampler::Sampler>(&self, sampler: S) {
            (self as &(dyn ErasedSource + 'a)).sample_metrics(sampler)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::cell::Cell;

        #[test]
        fn source_sample_emit() {
            struct MySource;

            impl Source for MySource {
                fn sample_metrics<S: Sampler>(&self, sampler: S) {
                    sampler.metric(Metric::new(
                        Path::new_raw("test"),
                        "metric 1",
                        "count",
                        crate::Empty,
                        42,
                        crate::Empty,
                    ));

                    sampler.metric(Metric::new(
                        Path::new_raw("test"),
                        "metric 2",
                        "count",
                        crate::Empty,
                        42,
                        crate::Empty,
                    ));
                }
            }

            let calls = Cell::new(0);

            MySource.sample_metrics(sampler::from_fn(|_| {
                calls.set(calls.get() + 1);
            }));

            assert_eq!(2, calls.get());
        }

        #[test]
        fn and_sample() {
            let calls = Cell::new(0);

            from_fn(|sampler| {
                sampler.metric(Metric::new(
                    Path::new_raw("test"),
                    "metric 1",
                    "count",
                    crate::Empty,
                    42,
                    crate::Empty,
                ));
            })
            .and_sample(from_fn(|sampler| {
                sampler.metric(Metric::new(
                    Path::new_raw("test"),
                    "metric 2",
                    "count",
                    crate::Empty,
                    42,
                    crate::Empty,
                ));
            }))
            .sample_metrics(sampler::from_fn(|_| {
                calls.set(calls.get() + 1);
            }));

            assert_eq!(2, calls.get());
        }

        #[test]
        fn from_fn_source() {
            let calls = Cell::new(0);

            from_fn(|sampler| {
                sampler.metric(Metric::new(
                    Path::new_raw("test"),
                    "metric 1",
                    "count",
                    crate::Empty,
                    42,
                    crate::Empty,
                ));

                sampler.metric(Metric::new(
                    Path::new_raw("test"),
                    "metric 2",
                    "count",
                    crate::Empty,
                    42,
                    crate::Empty,
                ));
            })
            .sample_metrics(sampler::from_fn(|_| {
                calls.set(calls.get() + 1);
            }));

            assert_eq!(2, calls.get());
        }

        #[test]
        fn erased_source() {
            let source = from_fn(|sampler| {
                sampler.metric(Metric::new(
                    Path::new_raw("test"),
                    "metric 1",
                    "count",
                    crate::Empty,
                    42,
                    crate::Empty,
                ));

                sampler.metric(Metric::new(
                    Path::new_raw("test"),
                    "metric 2",
                    "count",
                    crate::Empty,
                    42,
                    crate::Empty,
                ));
            });

            let source = &source as &dyn ErasedSource;

            let calls = Cell::new(0);

            source.sample_metrics(sampler::from_fn(|_| {
                calls.set(calls.get() + 1);
            }));

            assert_eq!(2, calls.get());
        }

        #[test]
        fn metric_as_source() {
            let sampler = sampler::from_fn(|metric| {
                assert_eq!("metric", metric.name().to_string());
                assert_eq!("count", metric.agg().to_string());
            });

            let metric = Metric::new(
                Path::new_raw("test"),
                "metric",
                "count",
                crate::Empty,
                42,
                crate::Empty,
            );

            metric.sample_metrics(sampler);
        }
    }
}

#[cfg(feature = "alloc")]
mod alloc_support {
    use super::*;

    use alloc::{boxed::Box, vec::Vec};
    use core::ops::Range;

    use crate::{
        clock::{Clock, ErasedClock},
        metric::source::{ErasedSource, Source},
    };

    /**
    A set of [`Source`]s that are all sampled together.

    The reporter can be sampled like any other source through its own [`Source`] implementation.

    # Normalization

    The reporter will attempt to normalize the extents of any metrics sampled from its sources. Normalization will:

    1. Take the current timestamp, `now`, when sampling metrics.
    2. If the metric sample has no extent, or has a point extent, it will be replaced with `now`.
    3. If the metric sample has a range extent, the end will be set to `now` and the start will be `now` minus the original length. If this would produce an invalid range then the original is kept.

    When the `std` Cargo feature is enabled this will be done automatically. In other cases, normalization won't happen unless it's configured by [`Reporter::normalize_with_clock`].

    Normalization can be disabled by calling [`Reporter::without_normalization`].
    */
    pub struct Reporter {
        sources: Vec<Box<dyn ErasedSource + Send + Sync>>,
        clock: ReporterClock,
    }

    impl Reporter {
        /**
        Create a new empty reporter.

        When the `std` Cargo feature is enabled, the reporter will normalize timestamps on reported samples using the system clock.
        When the `std` Cargo feature is not enabled, the reporter will not attempt to normalize timestamps.
        */
        pub const fn new() -> Self {
            Reporter {
                sources: Vec::new(),
                clock: ReporterClock::Default,
            }
        }

        /**
        Set the clock the reporter will use to unify timestamps on sampled metrics.
        */
        pub fn normalize_with_clock(
            &mut self,
            clock: impl Clock + Send + Sync + 'static,
        ) -> &mut Self {
            self.clock = ReporterClock::Other(Some(Box::new(clock)));

            self
        }

        /**
        Disable the clock, preventing the reporter from normalizing timestamps on sampled metrics.
        */
        pub fn without_normalization(&mut self) -> &mut Self {
            self.clock = ReporterClock::Other(None);

            self
        }

        /**
        Add a [`Source`] to the reporter.
        */
        pub fn add_source(&mut self, source: impl Source + Send + Sync + 'static) -> &mut Self {
            self.sources.push(Box::new(source));

            self
        }

        /**
        Produce a current sample for all metrics.
        */
        pub fn sample_metrics<S: sampler::Sampler>(&self, sampler: S) {
            let sampler = TimeNormalizer::new(self.clock.now(), sampler);

            for source in &self.sources {
                source.sample_metrics(&sampler);
            }
        }

        /**
        Produce a current sample for all metrics, emitting them as diagnostic events to the given [`Emitter`].
        */
        pub fn emit_metrics<E: Emitter>(&self, emitter: E) {
            self.sample_metrics(sampler::from_emitter(emitter))
        }
    }

    impl Source for Reporter {
        fn sample_metrics<S: sampler::Sampler>(&self, sampler: S) {
            self.sample_metrics(sampler)
        }
    }

    struct TimeNormalizer<S> {
        now: Option<Timestamp>,
        inner: S,
    }

    impl<S> TimeNormalizer<S> {
        fn new(now: Option<Timestamp>, sampler: S) -> TimeNormalizer<S> {
            TimeNormalizer {
                now,
                inner: sampler,
            }
        }
    }

    impl<S: Sampler> Sampler for TimeNormalizer<S> {
        fn metric<P: Props>(&self, metric: Metric<P>) {
            if let Some(now) = self.now {
                let extent = metric.extent();

                let extent = if let Some(range) = extent.and_then(|extent| extent.as_range()) {
                    // If the extent is a range then attempt to normalize it
                    normalize_range(now, range)
                        .map(Extent::range)
                        // If normalizing the range fails then use the original range
                        .unwrap_or_else(|| Extent::range(range.clone()))
                } else {
                    // If the extent is missing or a point then use the value of now
                    Extent::point(now)
                };

                self.inner.metric(metric.with_extent(extent))
            } else {
                self.inner.metric(metric)
            }
        }
    }

    fn normalize_range(now: Timestamp, range: &Range<Timestamp>) -> Option<Range<Timestamp>> {
        // Normalize a range by assigning its end bound to now
        // and its start bound to now - length
        let len = range.end.duration_since(range.start)?;
        let start = now.checked_sub(len)?;

        Some(start..now)
    }

    enum ReporterClock {
        Default,
        Other(Option<Box<dyn ErasedClock + Send + Sync>>),
    }

    impl Clock for ReporterClock {
        fn now(&self) -> Option<Timestamp> {
            match self {
                ReporterClock::Default => crate::platform::DefaultClock::new().now(),
                ReporterClock::Other(clock) => clock.now(),
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::time::Duration;

        #[cfg(all(
            target_arch = "wasm32",
            target_vendor = "unknown",
            target_os = "unknown"
        ))]
        use wasm_bindgen_test::*;

        #[test]
        fn reporter_is_send_sync() {
            fn check<T: Send + Sync>() {}

            check::<Reporter>();
        }

        #[test]
        #[cfg(not(miri))]
        #[cfg_attr(
            all(
                target_arch = "wasm32",
                target_vendor = "unknown",
                target_os = "unknown"
            ),
            wasm_bindgen_test
        )]
        fn reporter_sample() {
            use std::cell::Cell;

            let mut reporter = Reporter::new();

            reporter
                .add_source(source::from_fn(|sampler| {
                    sampler.metric(Metric::new(
                        Path::new_raw("test"),
                        "metric 1",
                        "count",
                        crate::Empty,
                        42,
                        crate::Empty,
                    ));
                }))
                .add_source(source::from_fn(|sampler| {
                    sampler.metric(Metric::new(
                        Path::new_raw("test"),
                        "metric 2",
                        "count",
                        crate::Empty,
                        42,
                        crate::Empty,
                    ));
                }));

            let calls = Cell::new(0);

            reporter.sample_metrics(sampler::from_fn(|_| {
                calls.set(calls.get() + 1);
            }));

            assert_eq!(2, calls.get());
        }

        struct TestClock(Option<Timestamp>);

        impl Clock for TestClock {
            fn now(&self) -> Option<Timestamp> {
                self.0
            }
        }

        #[test]
        #[cfg(all(feature = "std", not(miri)))]
        #[cfg_attr(
            all(
                target_arch = "wasm32",
                target_vendor = "unknown",
                target_os = "unknown"
            ),
            wasm_bindgen_test
        )]
        fn reporter_normalize_std() {
            let mut reporter = Reporter::new();

            reporter.add_source(source::from_fn(|sampler| {
                sampler.metric(Metric::new(
                    Path::new_raw("test"),
                    "metric 1",
                    "count",
                    crate::Empty,
                    42,
                    crate::Empty,
                ));
            }));

            reporter.sample_metrics(sampler::from_fn(|metric| {
                assert!(metric.extent().is_some());
            }));
        }

        #[wasm_bindgen_test]
        #[cfg(all(feature = "web", not(miri)))]
        #[cfg(all(
            target_arch = "wasm32",
            target_vendor = "unknown",
            target_os = "unknown"
        ))]
        fn reporter_normalize_web() {
            let mut reporter = Reporter::new();

            reporter.add_source(source::from_fn(|sampler| {
                sampler.metric(Metric::new(
                    Path::new_raw("test"),
                    "metric 1",
                    "count",
                    crate::Empty,
                    42,
                    crate::Empty,
                ));
            }));

            reporter.sample_metrics(sampler::from_fn(|metric| {
                assert!(metric.extent().is_some());
            }));
        }

        #[test]
        #[cfg_attr(
            all(
                target_arch = "wasm32",
                target_vendor = "unknown",
                target_os = "unknown"
            ),
            wasm_bindgen_test
        )]
        fn reporter_normalize_empty_extent() {
            let mut reporter = Reporter::new();

            reporter.normalize_with_clock(TestClock(Some(Timestamp::MIN)));

            reporter.add_source(source::from_fn(|sampler| {
                sampler.metric(Metric::new(
                    Path::new_raw("test"),
                    "metric 1",
                    "count",
                    crate::Empty,
                    42,
                    crate::Empty,
                ));
            }));

            reporter.sample_metrics(sampler::from_fn(|metric| {
                assert_eq!(Timestamp::MIN, metric.extent().unwrap().as_point());
            }));
        }

        #[test]
        #[cfg_attr(
            all(
                target_arch = "wasm32",
                target_vendor = "unknown",
                target_os = "unknown"
            ),
            wasm_bindgen_test
        )]
        fn reporter_normalize_point_extent() {
            let mut reporter = Reporter::new();

            reporter.normalize_with_clock(TestClock(Some(
                Timestamp::from_unix(Duration::from_secs(37)).unwrap(),
            )));

            reporter.add_source(source::from_fn(|sampler| {
                sampler.metric(Metric::new(
                    Path::new_raw("test"),
                    "metric 1",
                    "count",
                    Timestamp::from_unix(Duration::from_secs(100)).unwrap(),
                    42,
                    crate::Empty,
                ));
            }));

            reporter.sample_metrics(sampler::from_fn(|metric| {
                assert_eq!(
                    Timestamp::from_unix(Duration::from_secs(37)).unwrap(),
                    metric.extent().unwrap().as_point()
                );
            }));
        }

        #[test]
        #[cfg_attr(
            all(
                target_arch = "wasm32",
                target_vendor = "unknown",
                target_os = "unknown"
            ),
            wasm_bindgen_test
        )]
        fn reporter_normalize_range_extent() {
            let mut reporter = Reporter::new();

            reporter.normalize_with_clock(TestClock(Some(
                Timestamp::from_unix(Duration::from_secs(350)).unwrap(),
            )));

            reporter.add_source(source::from_fn(|sampler| {
                sampler.metric(Metric::new(
                    Path::new_raw("test"),
                    "metric 1",
                    "count",
                    Timestamp::from_unix(Duration::from_secs(100)).unwrap()
                        ..Timestamp::from_unix(Duration::from_secs(200)).unwrap(),
                    42,
                    crate::Empty,
                ));
            }));

            reporter.sample_metrics(sampler::from_fn(|metric| {
                assert_eq!(
                    Timestamp::from_unix(Duration::from_secs(250)).unwrap()
                        ..Timestamp::from_unix(Duration::from_secs(350)).unwrap(),
                    metric.extent().unwrap().as_range().unwrap().clone()
                );
            }));
        }
    }
}

#[cfg(feature = "alloc")]
pub use self::alloc_support::*;

pub mod sampler {
    /*!
    The [`Sampler`] type.

    A [`Sampler`] is a visitor for a [`Source`] that receives [`Metric`]s when the source is sampled.
    */

    use emit_core::empty::Empty;

    use super::*;

    /**
    A receiver of [`Metric`]s as produced by a [`Source`].
    */
    pub trait Sampler {
        /**
        Receive a metric sample.
        */
        fn metric<P: Props>(&self, metric: Metric<P>);
    }

    impl<'a, T: Sampler + ?Sized> Sampler for &'a T {
        fn metric<P: Props>(&self, metric: Metric<P>) {
            (**self).metric(metric)
        }
    }

    impl Sampler for Empty {
        fn metric<P: Props>(&self, _: Metric<P>) {}
    }

    /**
    A [`Sampler`] from an [`Emitter`].

    On completion, a [`Metric`] will be emitted as an event using [`Metric::to_event`].

    This type can be created directly, or via [`from_emitter`].
    */
    pub struct FromEmitter<E>(E);

    impl<E: Emitter> Sampler for FromEmitter<E> {
        fn metric<P: Props>(&self, metric: Metric<P>) {
            self.0.emit(metric)
        }
    }

    impl<E> FromEmitter<E> {
        /**
        Wrap the given emitter.
        */
        pub const fn new(emitter: E) -> Self {
            FromEmitter(emitter)
        }
    }

    /**
    A [`Sampler`] from an [`Emitter`].

    On completion, a [`Metric`] will be emitted as an event using [`Metric::to_event`].
    */
    pub const fn from_emitter<E: Emitter>(emitter: E) -> FromEmitter<E> {
        FromEmitter(emitter)
    }

    /**
    A [`Sampler`] from a function.

    This type can be created directly, or via [`from_fn`].
    */
    pub struct FromFn<F = fn(Metric<&dyn ErasedProps>)>(F);

    /**
    Create a [`Sampler`] from a function.
    */
    pub const fn from_fn<F: Fn(Metric<&dyn ErasedProps>)>(f: F) -> FromFn<F> {
        FromFn(f)
    }

    impl<F> FromFn<F> {
        /**
        Wrap the given sampler function.
        */
        pub const fn new(sampler: F) -> FromFn<F> {
            FromFn(sampler)
        }
    }

    impl<F: Fn(Metric<&dyn ErasedProps>)> Sampler for FromFn<F> {
        fn metric<P: Props>(&self, metric: Metric<P>) {
            (self.0)(metric.erase())
        }
    }

    mod internal {
        use super::*;

        pub trait DispatchSampler {
            fn dispatch_metric(&self, metric: Metric<&dyn ErasedProps>);
        }

        pub trait SealedSampler {
            fn erase_sampler(&self) -> crate::internal::Erased<&dyn DispatchSampler>;
        }
    }

    /**
    An object-safe [`Sampler`].

    A `dyn ErasedSampler` can be treated as `impl Sampler`.
    */
    pub trait ErasedSampler: internal::SealedSampler {}

    impl<T: Sampler> ErasedSampler for T {}

    impl<T: Sampler> internal::SealedSampler for T {
        fn erase_sampler(&self) -> crate::internal::Erased<&dyn internal::DispatchSampler> {
            crate::internal::Erased(self)
        }
    }

    impl<T: Sampler> internal::DispatchSampler for T {
        fn dispatch_metric(&self, metric: Metric<&dyn ErasedProps>) {
            self.metric(metric)
        }
    }

    impl<'a> Sampler for dyn ErasedSampler + 'a {
        fn metric<P: Props>(&self, metric: Metric<P>) {
            self.erase_sampler().0.dispatch_metric(metric.erase())
        }
    }

    impl<'a> Sampler for dyn ErasedSampler + Send + Sync + 'a {
        fn metric<P: Props>(&self, metric: Metric<P>) {
            (self as &(dyn ErasedSampler + 'a)).metric(metric)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::cell::Cell;

        #[test]
        fn from_fn_sampler() {
            let called = Cell::new(false);

            let sampler = from_fn(|metric| {
                assert_eq!("test", metric.name());

                called.set(true);
            });

            sampler.metric(Metric::new(
                Path::new_raw("test"),
                "test",
                "count",
                Empty,
                1,
                Empty,
            ));

            assert!(called.get());
        }

        #[test]
        fn erased_sampler() {
            let called = Cell::new(false);

            let sampler = from_fn(|metric| {
                assert_eq!("test", metric.name());

                called.set(true);
            });

            let sampler = &sampler as &dyn ErasedSampler;

            sampler.metric(Metric::new(
                Path::new_raw("test"),
                "test",
                "count",
                Empty,
                1,
                Empty,
            ));

            assert!(called.get());
        }
    }
}

pub mod dist {
    /*!
    Functions for working with metric distributions.
    */

    use crate::platform::libm;

    /**
    Compute the exponential bucket midpoint for the given input value at a given scale.

    This function accepts the following parameters:

    - `value`: The observed sample value to be bucketed.
    - `scale`: The size of exponential buckets. Larger scales produce larger numbers of smaller buckets.

    This function can be used to compress an input data stream by feeding it input values and tracking the counts of resulting buckets.
    The choice of `scale` is a trade-off between size and accuracy.
    Larger buckets (smaller scales) count more unique input values in fewer unique bucket values, and resulting in higher compression but lower accuracy.

    This function uses the same `scale` as OpenTelemetry's metrics data model, but returns the midpoint of the bucket a value belongs to instead of its index.

    ## Algorithm

    This function implements the following procedure:

    ```text
    let sign = if value == 0.0 || value.is_sign_positive() { 1.0 } else { -1.0 };
    let value = value.abs();

    let gamma = 2.0f64.powf(2.0f64.powf(-scale));
    let index = value.log(gamma).ceil();

    let lower = gamma.log(index - 1.0);
    let upper = lower * gamma;

    sign * lower.midpoint(upper)
    ```

    The implementation here uses a portable implementation of `powf` and `log` that is consistent across platforms.
    You may also consider using a native port of it for performance reasons.
    */
    pub const fn midpoint(value: f64, scale: i32) -> f64 {
        let sign = if value == 0.0 || value.is_sign_positive() {
            1.0
        } else {
            -1.0
        };
        let value = value.abs();

        let gamma = libm::pow(2.0, libm::pow(2.0, -(scale as f64)));
        let index = libm::ceil(libm::log(value, gamma));

        let lower = libm::pow(gamma, index - 1.0);
        let upper = lower * gamma;

        sign * lower.midpoint(upper)
    }

    #[cfg(feature = "sval")]
    mod visit {
        use core::{fmt, ops::ControlFlow};

        use crate::Value;

        /**
        An error encountered attempting to visit bucket midpoint/count pairs.
        */
        #[derive(Debug)]
        pub struct VisitBucketPointsError {}

        impl fmt::Display for VisitBucketPointsError {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(
                    f,
                    "the input was not a valid sequence of bucket midpoint/count pairs"
                )
            }
        }

        #[cfg(feature = "std")]
        impl std::error::Error for VisitBucketPointsError {}

        /**
        Visit the bucket midpoint/count pairs in a value.
        */
        pub fn visit_bucket_points(
            buckets: Value,
            for_each: impl FnMut(f64, u64) -> ControlFlow<()>,
        ) -> Result<(), VisitBucketPointsError> {
            struct Stream<F> {
                depth: usize,
                next_midpoint: Option<f64>,
                next_count: Option<u64>,
                next: F,
                done: bool,
            }

            impl<F> Stream<F> {
                fn push(
                    &mut self,
                    midpoint: impl FnOnce() -> Option<f64>,
                    count: impl FnOnce() -> Option<u64>,
                ) -> sval::Result {
                    if self.depth == 2 {
                        if self.next_midpoint.is_none() {
                            self.next_midpoint = midpoint();

                            return Ok(());
                        }

                        if self.next_count.is_none() {
                            self.next_count = count();

                            return Ok(());
                        }
                    }

                    sval::error()
                }

                fn take(&mut self) -> Option<sval::Result<(f64, u64)>> {
                    if self.depth == 2 {
                        let (next_midpoint, next_count) =
                            (self.next_midpoint.take(), self.next_count.take());

                        Some((|| {
                            sval::Result::Ok((
                                next_midpoint.ok_or_else(|| sval::Error::new())?,
                                next_count.ok_or_else(|| sval::Error::new())?,
                            ))
                        })())
                    } else {
                        None
                    }
                }
            }

            impl<'sval, F: FnMut(f64, u64) -> ControlFlow<()>> sval::Stream<'sval> for Stream<F> {
                fn null(&mut self) -> sval::Result {
                    sval::error()
                }

                fn bool(&mut self, _: bool) -> sval::Result {
                    sval::error()
                }

                fn text_begin(&mut self, _: Option<usize>) -> sval::Result {
                    sval::error()
                }

                fn text_fragment_computed(&mut self, _: &str) -> sval::Result {
                    sval::error()
                }

                fn text_end(&mut self) -> sval::Result {
                    sval::error()
                }

                fn i64(&mut self, value: i64) -> sval::Result {
                    self.push(|| Some(value as f64), || value.try_into().ok())
                }

                fn u64(&mut self, value: u64) -> sval::Result {
                    self.push(|| Some(value as f64), || value.try_into().ok())
                }

                fn f64(&mut self, value: f64) -> sval::Result {
                    self.push(|| Some(value), || Some(value as u64))
                }

                fn seq_begin(&mut self, _: Option<usize>) -> sval::Result {
                    self.depth += 1;

                    if self.depth > 2 {
                        sval::error()
                    } else {
                        Ok(())
                    }
                }

                fn seq_value_begin(&mut self) -> sval::Result {
                    Ok(())
                }

                fn seq_value_end(&mut self) -> sval::Result {
                    Ok(())
                }

                fn seq_end(&mut self) -> sval::Result {
                    if let Some(next) = self.take() {
                        let (midpoint, count) = next?;

                        if let ControlFlow::Break(()) = (self.next)(midpoint, count) {
                            self.done = true;
                            return sval::error();
                        };
                    }

                    self.depth -= 1;

                    Ok(())
                }

                fn map_begin(&mut self, _: Option<usize>) -> sval::Result {
                    sval::error()
                }
            }

            let mut stream = Stream {
                depth: 0,
                next_midpoint: None,
                next_count: None,
                next: for_each,
                done: false,
            };

            let r = sval::stream(&mut stream, &buckets);

            if r.is_ok() || stream.done {
                Ok(())
            } else {
                Err(VisitBucketPointsError {})
            }
        }

        #[cfg(test)]
        mod tests {
            use super::*;

            #[test]
            fn visit_bucket_points_valid() {
                let value = [(1.0, 1), (2.0, 2), (3.0, 3)];

                let mut actual = Vec::new();

                let r = visit_bucket_points(Value::from_sval(&value), |midpoint, count| {
                    actual.push((midpoint, count));

                    ControlFlow::Continue(())
                });

                assert!(r.is_ok());

                assert_eq!(&value, &*actual);
            }

            #[test]
            fn visit_bucket_points_empty() {
                assert!(
                    visit_bucket_points(Value::from_any(&[] as &[f64; 0]), |_, _| {
                        ControlFlow::Continue(())
                    })
                    .is_ok()
                );
            }

            #[test]
            fn visit_bucket_points_invalid() {
                assert!(
                    visit_bucket_points(Value::from(0.0), |_, _| ControlFlow::Continue(()))
                        .is_err()
                );
                assert!(
                    visit_bucket_points(Value::from_any(&[1.0, 2.0, 3.0]), |_, _| {
                        ControlFlow::Continue(())
                    })
                    .is_err()
                );
                assert!(
                    visit_bucket_points(Value::from_sval(&[[(1.0, 42)]]), |_, _| {
                        ControlFlow::Continue(())
                    })
                    .is_err()
                );
                assert!(
                    visit_bucket_points(Value::from_sval(&[(1.0, 42, 1.0)]), |_, _| {
                        ControlFlow::Continue(())
                    })
                    .is_err()
                );
                assert!(
                    visit_bucket_points(Value::from_sval(&[(true, 42)]), |_, _| {
                        ControlFlow::Continue(())
                    })
                    .is_err()
                );
                assert!(
                    visit_bucket_points(Value::from_sval(&[("text", 42)]), |_, _| {
                        ControlFlow::Continue(())
                    })
                    .is_err()
                );
            }
        }
    }

    #[cfg(feature = "sval")]
    pub use self::visit::*;

    #[cfg(test)]
    mod tests {
        use core::f64::consts::PI;

        use super::*;

        #[test]
        fn compute_midpoints() {
            let cases = [
                0.0f64,
                PI,
                PI * 100.0f64,
                PI * 1000.0f64,
                -0.0f64,
                -PI,
                -(PI * 100.0f64),
                -(PI * 1000.0f64),
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NAN,
            ];
            for (scale, expected) in [
                (
                    0i32,
                    [
                        0.0f64,
                        3.0f64,
                        384.0f64,
                        3072.0f64,
                        0.0f64,
                        -3.0f64,
                        -384.0f64,
                        -3072.0f64,
                        f64::INFINITY,
                        f64::NEG_INFINITY,
                        f64::NAN,
                    ],
                ),
                (
                    2i32,
                    [
                        0.0f64,
                        3.0960063928805233f64,
                        333.2378467041041f64,
                        3170.3105463096517f64,
                        0.0f64,
                        -3.0960063928805233f64,
                        -333.2378467041041f64,
                        -3170.3105463096517f64,
                        f64::INFINITY,
                        f64::NEG_INFINITY,
                        f64::NAN,
                    ],
                ),
                (
                    4i32,
                    [
                        0.0f64,
                        3.152701157357188f64,
                        311.17631066575086f64,
                        3091.493858659732f64,
                        0.0f64,
                        -3.152701157357188f64,
                        -311.17631066575086f64,
                        -3091.493858659732f64,
                        f64::INFINITY,
                        f64::NEG_INFINITY,
                        f64::NAN,
                    ],
                ),
                (
                    8i32,
                    [
                        0.0f64,
                        3.1391891212579424f64,
                        314.0658342072582f64,
                        3145.6489181930947f64,
                        0.0f64,
                        -3.1391891212579424f64,
                        -314.0658342072582f64,
                        -3145.6489181930947f64,
                        f64::INFINITY,
                        f64::NEG_INFINITY,
                        f64::NAN,
                    ],
                ),
                (
                    16i32,
                    [
                        0.0f64,
                        3.141594303685526f64,
                        314.1602303152259f64,
                        3141.606302893263f64,
                        0.0f64,
                        -3.141594303685526f64,
                        -314.1602303152259f64,
                        -3141.606302893263f64,
                        f64::INFINITY,
                        f64::NEG_INFINITY,
                        f64::NAN,
                    ],
                ),
            ] {
                for (case, expected) in cases.iter().copied().zip(expected.iter().copied()) {
                    let actual = midpoint(case, scale);

                    if expected.is_nan() && actual.is_nan() {
                        continue;
                    }

                    assert_eq!(
                        expected.to_bits(),
                        actual.to_bits(),
                        "expected midpoint({case}, {scale}) to be {expected}, but got {actual}"
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    use crate::Timestamp;

    #[test]
    fn metric_new() {
        let metric = Metric::new(
            Path::new_raw("test"),
            "my metric",
            "count",
            Timestamp::from_unix(Duration::from_secs(1)),
            42,
            ("metric_prop", true),
        );

        assert_eq!("test", metric.mdl());
        assert_eq!(
            Timestamp::from_unix(Duration::from_secs(1)).unwrap(),
            metric.extent().unwrap().as_point()
        );
        assert_eq!("my metric", metric.name());
        assert_eq!("count", metric.agg());
        assert_eq!(42, metric.value().by_ref().cast::<i32>().unwrap());
        assert_eq!(true, metric.props().pull::<bool, _>("metric_prop").unwrap());
    }

    #[test]
    fn metric_to_event() {
        let metric = Metric::new(
            Path::new_raw("test"),
            "my metric",
            "count",
            Timestamp::from_unix(Duration::from_secs(1)),
            42,
            ("metric_prop", true),
        );

        let evt = metric.to_event();

        assert_eq!("test", evt.mdl());
        assert_eq!(
            Timestamp::from_unix(Duration::from_secs(1)).unwrap(),
            evt.extent().unwrap().as_point()
        );
        assert_eq!("count of my metric is 42", evt.msg().to_string());
        assert_eq!("count", evt.props().pull::<Str, _>(KEY_METRIC_AGG).unwrap());
        assert_eq!(42, evt.props().pull::<i32, _>(KEY_METRIC_VALUE).unwrap());
        assert_eq!(
            "my metric",
            evt.props().pull::<Str, _>(KEY_METRIC_NAME).unwrap()
        );
        assert_eq!(true, evt.props().pull::<bool, _>("metric_prop").unwrap());
        assert_eq!(
            Kind::Metric,
            evt.props().pull::<Kind, _>(KEY_EVT_KIND).unwrap()
        );
    }

    #[test]
    fn metric_to_event_uses_tpl() {
        assert_eq!(
            "test",
            Metric::new(
                Path::new_raw("test"),
                "my metric",
                "count",
                Timestamp::from_unix(Duration::from_secs(1)),
                42,
                ("metric_prop", true),
            )
            .with_tpl(Template::literal("test"))
            .to_event()
            .msg()
            .to_string(),
        );
    }

    #[test]
    fn metric_to_extent() {
        for (case, expected) in [
            (
                Some(Timestamp::from_unix(Duration::from_secs(1)).unwrap()),
                Some(Extent::point(
                    Timestamp::from_unix(Duration::from_secs(1)).unwrap(),
                )),
            ),
            (None, None),
        ] {
            let metric = Metric::new(
                Path::new_raw("test"),
                "my metric",
                "count",
                case,
                42,
                ("metric_prop", true),
            );

            let extent = metric.to_extent();

            assert_eq!(
                expected.map(|extent| extent.as_range().cloned()),
                extent.map(|extent| extent.as_range().cloned())
            );
        }
    }
}
