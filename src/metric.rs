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
    well_known::{
        KEY_DIST_COUNT, KEY_DIST_EXP_SCALE, KEY_DIST_MAX, KEY_DIST_MIN, KEY_DIST_SUM, KEY_EVT_KIND,
        KEY_METRIC_AGG, KEY_METRIC_DESCRIPTION, KEY_METRIC_NAME, KEY_METRIC_UNIT, KEY_METRIC_VALUE,
    },
};

#[cfg(feature = "alloc")]
use emit_core::well_known::KEY_DIST_EXP_BUCKETS;

use crate::kind::Kind;

pub use self::{sampler::Sampler, source::Source};

/**
A diagnostic event that represents a metric sample.

Metrics are an extension of [`Event`]s that explicitly take the well-known properties that signal an event as being a metric sample. See the [`mod@crate::metric`] module for details.

A `Metric` can be converted into an [`Event`] through its [`ToEvent`] implemenation, or passed directly to an [`Emitter`] to emit it.
*/
pub struct Metric<'a, P> {
    mdl: Path<'a>,
    extent: Option<Extent>,
    tpl: Option<Template<'a>>,
    props: P,
}

impl<'a, P> Metric<'a, P> {
    /**
    Create a new metric from its properties.

    Each metric consists of:

    - `mdl`: The module that owns the underlying data source.
    - `extent`: The [`Extent`] that the sample covers.
    - `props`: Additional [`Props`] to associate with the sample.
    */
    pub fn new(mdl: impl Into<Path<'a>>, extent: impl ToExtent, props: P) -> Self {
        Metric {
            mdl: mdl.into(),
            extent: extent.to_extent(),
            tpl: None,
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
    Get exclusive access to additional properties associated with the sample.
    */
    pub fn props_mut(&mut self) -> &mut P {
        &mut self.props
    }

    /**
    Set the additional properties associated with the sample to a new value.
    */
    pub fn with_props<U>(self, props: U) -> Metric<'a, U> {
        Metric {
            mdl: self.mdl,
            extent: self.extent,
            tpl: self.tpl,
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
            props: map(self.props),
        }
    }
}

impl<'a, P: Props> Metric<'a, P> {
    /**
    Get the name of the underlying data source.
    */
    pub fn name(&self) -> Option<Str<'_>> {
        self.props.pull(KEY_METRIC_NAME)
    }

    /**
    Set the name of the underlying data source.
    */
    pub fn with_name(
        self,
        name: impl Into<Str<'a>>,
    ) -> Metric<'a, And<(&'static str, Str<'a>), P>> {
        self.map_props(|props| (KEY_METRIC_NAME, name.into()).and_props(props))
    }

    /**
    Get a description of the underlying data source.
    */
    pub fn description(&self) -> Option<Str<'_>> {
        self.props.pull(KEY_METRIC_DESCRIPTION)
    }

    /**
    Set the description of the underlying data source.
    */
    pub fn with_description(
        self,
        description: impl Into<Str<'a>>,
    ) -> Metric<'a, And<(&'static str, Str<'a>), P>> {
        self.map_props(|props| (KEY_METRIC_DESCRIPTION, description.into()).and_props(props))
    }

    /**
    Get the aggregation applied to the underlying data source to produce the sample.

    The value of the aggregation should be one of the [`crate::well_known`] aggregation types.
    */
    pub fn agg(&self) -> Option<Str<'_>> {
        self.props.pull(KEY_METRIC_AGG)
    }

    /**
    Set the aggregation applied to the underyling data source to produce the sample.

    The value of the aggregation should be one of the [`crate::well_known`] aggregation types.
    */
    pub fn with_agg(self, agg: impl Into<Str<'a>>) -> Metric<'a, And<(&'static str, Str<'a>), P>> {
        self.map_props(|props| (KEY_METRIC_AGG, agg.into()).and_props(props))
    }

    /**
    Get the unit of the sample value.
    */
    pub fn unit(&self) -> Option<Str<'_>> {
        self.props.pull(KEY_METRIC_UNIT)
    }

    /**
    Set the unit of the sample value.
    */
    pub fn with_unit(
        self,
        unit: impl Into<Str<'a>>,
    ) -> Metric<'a, And<(&'static str, Str<'a>), P>> {
        self.map_props(|props| (KEY_METRIC_UNIT, unit.into()).and_props(props))
    }

    /**
    Get the value of the sample itself.
    */
    pub fn value(&self) -> Option<Value<'_>> {
        self.props.get(KEY_METRIC_VALUE)
    }

    /**
    Set the value of the sample itself.
    */
    pub fn with_value(
        self,
        value: impl Into<Value<'a>>,
    ) -> Metric<'a, And<(&'static str, Value<'a>), P>> {
        self.map_props(|props| (KEY_METRIC_VALUE, value.into()).and_props(props))
    }

    /**
    Get the minimum observed value.
    */
    pub fn dist_min(&self) -> Option<Value<'_>> {
        self.props.get(KEY_DIST_MIN)
    }

    /**
    Set the minimum observed value.
    */
    pub fn with_dist_min(
        self,
        dist_min: impl Into<Value<'a>>,
    ) -> Metric<'a, And<(&'static str, Value<'a>), P>> {
        self.map_props(|props| (KEY_DIST_MIN, dist_min.into()).and_props(props))
    }

    /**
    Get the maximum observed value.
    */
    pub fn dist_max(&self) -> Option<Value<'_>> {
        self.props.get(KEY_DIST_MAX)
    }

    /**
    Set the maximum observed value.
    */
    pub fn with_dist_max(
        self,
        dist_max: impl Into<Value<'a>>,
    ) -> Metric<'a, And<(&'static str, Value<'a>), P>> {
        self.map_props(|props| (KEY_DIST_MAX, dist_max.into()).and_props(props))
    }

    /**
    Get the count of observed values.
    */
    pub fn dist_count(&self) -> Option<Value<'_>> {
        self.props.get(KEY_DIST_COUNT)
    }

    /**
    Set the count of observed values.
    */
    pub fn with_dist_count(
        self,
        dist_count: impl Into<Value<'a>>,
    ) -> Metric<'a, And<(&'static str, Value<'a>), P>> {
        self.map_props(|props| (KEY_DIST_COUNT, dist_count.into()).and_props(props))
    }

    /**
    Get the sum of observed values.
    */
    pub fn dist_sum(&self) -> Option<Value<'_>> {
        self.props.get(KEY_DIST_SUM)
    }

    /**
    Set the sum of observed values.
    */
    pub fn with_dist_sum(
        self,
        dist_sum: impl Into<Value<'a>>,
    ) -> Metric<'a, And<(&'static str, Value<'a>), P>> {
        self.map_props(|props| (KEY_DIST_SUM, dist_sum.into()).and_props(props))
    }

    /**
    Get the scale of exponential histogram buckets.
    */
    pub fn dist_exp_scale(&self) -> Option<i32> {
        self.props.pull(KEY_DIST_EXP_SCALE)
    }

    /**
    Set the scale of exponential histogram buckets.
    */
    pub fn with_dist_exp_scale(
        self,
        dist_exp_scale: impl Into<i32>,
    ) -> Metric<'a, And<(&'static str, i32), P>> {
        self.map_props(|props| (KEY_DIST_EXP_SCALE, dist_exp_scale.into()).and_props(props))
    }

    /**
    Get the exponential histogram buckets.
    */
    #[cfg(feature = "alloc")]
    pub fn dist_exp_buckets(&self) -> Option<exp::BucketSet> {
        self.props.pull(KEY_DIST_EXP_BUCKETS)
    }

    /**
    Set the exponential histogram buckets.
    */
    #[cfg(feature = "alloc")]
    pub fn with_dist_exp_buckets(
        self,
        dist_exp_buckets: impl Into<exp::BucketSet>,
    ) -> Metric<'a, And<(&'static str, exp::BucketSet), P>> {
        self.map_props(|props| (KEY_DIST_EXP_BUCKETS, dist_exp_buckets.into()).and_props(props))
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
                        crate::Empty,
                        crate::Empty,
                    ));

                    sampler.metric(Metric::new(
                        Path::new_raw("test"),
                        crate::Empty,
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
                    crate::Empty,
                    crate::Empty,
                ));
            })
            .and_sample(from_fn(|sampler| {
                sampler.metric(Metric::new(
                    Path::new_raw("test"),
                    crate::Empty,
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
                    crate::Empty,
                    crate::Empty,
                ));

                sampler.metric(Metric::new(
                    Path::new_raw("test"),
                    crate::Empty,
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
                    crate::Empty,
                    crate::Empty,
                ));

                sampler.metric(Metric::new(
                    Path::new_raw("test"),
                    crate::Empty,
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
                assert_eq!("metric", metric.name().unwrap().to_string());
                assert_eq!("count", metric.agg().unwrap().to_string());
            });

            let metric = Metric::new(
                Path::new_raw("test"),
                crate::Empty,
                [("metric_name", "metric"), ("metric_agg", "count")],
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
            self.sample_metrics(sampler::from_emitter(emitter).with_sampled_at(self.clock.now()))
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

        fn sampled_at(&self) -> Option<Timestamp> {
            self.now
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
                        crate::Empty,
                        crate::Empty,
                    ));
                }))
                .add_source(source::from_fn(|sampler| {
                    sampler.metric(Metric::new(
                        Path::new_raw("test"),
                        crate::Empty,
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
                    crate::Empty,
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
                    crate::Empty,
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
                    crate::Empty,
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
                    Timestamp::from_unix(Duration::from_secs(100)).unwrap()
                        ..Timestamp::from_unix(Duration::from_secs(200)).unwrap(),
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

    use emit_core::{
        clock::Clock, ctxt::Ctxt, emitter::Emitter, empty::Empty, filter::Filter, rng::Rng,
        runtime::Runtime,
    };

    use super::*;

    /**
    A receiver of [`Metric`]s as produced by a [`Source`].
    */
    pub trait Sampler {
        /**
        Receive a metric sample.
        */
        fn metric<P: Props>(&self, metric: Metric<P>);

        /**
        A value for the point in time that the sample was requested.

        This value can be used to normalize timestamps for metrics that are logically sampled at the same time.
        */
        fn sampled_at(&self) -> Option<Timestamp> {
            None
        }

        /**
        Associate a [`Timestamp`] with the sampler.
        */
        fn with_sampled_at(self, now: Option<Timestamp>) -> WithSampledAt<Self>
        where
            Self: Sized,
        {
            WithSampledAt::new(self, now)
        }
    }

    impl<'a, T: Sampler + ?Sized> Sampler for &'a T {
        fn metric<P: Props>(&self, metric: Metric<P>) {
            (**self).metric(metric)
        }

        fn sampled_at(&self) -> Option<Timestamp> {
            (**self).sampled_at()
        }
    }

    impl Sampler for Empty {
        fn metric<P: Props>(&self, _: Metric<P>) {}
    }

    /**
    A [`Sampler`] with an explicit value for [`Sampler::now`].
    */
    pub struct WithSampledAt<S> {
        sampler: S,
        now: Option<Timestamp>,
    }

    impl<S> WithSampledAt<S> {
        /**
        Associate a [`Timestamp`] with a [`Sampler`].
        */
        pub const fn new(sampler: S, now: Option<Timestamp>) -> Self {
            WithSampledAt { sampler, now }
        }
    }

    impl<S: Sampler> Sampler for WithSampledAt<S> {
        fn metric<P: Props>(&self, metric: Metric<P>) {
            self.sampler.metric(metric)
        }

        fn sampled_at(&self) -> Option<Timestamp> {
            self.now
        }
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
    A [`Sampler`] from a [`Runtime`].

    On completion, a [`Metric`] will be emitted as an event using [`Metric::to_event`].

    This type can be created directly, or via [`from_runtime`].
    */
    pub struct FromRuntime<'a, E, F, C, T, R>(&'a Runtime<E, F, C, T, R>);

    impl<'a, E: Emitter, F: Filter, C: Ctxt, T: Clock, R: Rng> Sampler
        for FromRuntime<'a, E, F, C, T, R>
    {
        fn metric<P: Props>(&self, metric: Metric<P>) {
            self.0.emit(metric)
        }
    }

    impl<'a, E, F, C, T, R> FromRuntime<'a, E, F, C, T, R> {
        /**
        Wrap the given emitter.
        */
        pub const fn new(rt: &'a Runtime<E, F, C, T, R>) -> Self {
            FromRuntime(rt)
        }
    }

    /**
    A [`Sampler`] from a [`Runtime`].

    On completion, a [`Metric`] will be emitted as an event using [`Metric::to_event`].
    */
    pub const fn from_runtime<'a, E: Emitter, F: Filter, C: Ctxt, T: Clock, R: Rng>(
        rt: &'a Runtime<E, F, C, T, R>,
    ) -> FromRuntime<'a, E, F, C, T, R> {
        FromRuntime(rt)
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

            fn dispatch_sampled_at(&self) -> Option<Timestamp>;
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

        fn dispatch_sampled_at(&self) -> Option<Timestamp> {
            self.sampled_at()
        }
    }

    impl<'a> Sampler for dyn ErasedSampler + 'a {
        fn metric<P: Props>(&self, metric: Metric<P>) {
            self.erase_sampler().0.dispatch_metric(metric.erase())
        }

        fn sampled_at(&self) -> Option<Timestamp> {
            self.erase_sampler().0.dispatch_sampled_at()
        }
    }

    impl<'a> Sampler for dyn ErasedSampler + Send + Sync + 'a {
        fn metric<P: Props>(&self, metric: Metric<P>) {
            (self as &(dyn ErasedSampler + 'a)).metric(metric)
        }

        fn sampled_at(&self) -> Option<Timestamp> {
            (self as &(dyn ErasedSampler + 'a)).sampled_at()
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        use emit_core::{emitter, runtime::Runtime};

        use std::cell::Cell;

        #[test]
        fn from_fn_sampler() {
            let called = Cell::new(false);

            let sampler = from_fn(|metric| {
                assert_eq!("test", metric.name().unwrap());

                called.set(true);
            });

            sampler.metric(Metric::new(
                Path::new_raw("test"),
                Empty,
                ("metric_name", "test"),
            ));

            assert!(called.get());
        }

        #[test]
        fn erased_sampler() {
            let called = Cell::new(false);

            let sampler = from_fn(|metric| {
                assert_eq!("test", metric.name().unwrap());

                called.set(true);
            });

            let sampler = &sampler as &dyn ErasedSampler;

            sampler.metric(Metric::new(
                Path::new_raw("test"),
                crate::Empty,
                ("metric_name", "test"),
            ));

            assert!(called.get());
        }

        #[test]
        fn from_runtime_sampler() {
            let called = Cell::new(false);

            let rt = Runtime::default().with_emitter(emitter::from_fn(|_| {
                called.set(true);
            }));

            let sampler = from_runtime(&rt);

            sampler.metric(Metric::new(
                Path::new_raw("test"),
                crate::Empty,
                crate::Empty,
            ));

            assert!(called.get());
        }
    }
}

pub mod exp {
    /*!
    Functions for working with exponential histograms.
    */

    use crate::{
        platform::libm,
        value::{FromValue, ToValue, Value},
    };

    use core::{cmp, fmt, hash, str::FromStr};

    /**
    An error encountered attempting to parse a [`Point`].
    */
    #[derive(Debug)]
    pub struct ParsePointError {}

    impl fmt::Display for ParsePointError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "the input was not a valid point")
        }
    }

    #[cfg(feature = "std")]
    impl std::error::Error for ParsePointError {}

    /**
    A totally ordered value, representing a point within an exponential bucket.

    Values to construct points from can be computed by the [`midpoint`] function.

    This type is a plain wrapper over `f64`, but implements the necessary ordering traits needed to store them in `BTreeMap`s or `HashMap`s.
    */
    #[derive(Clone, Copy)]
    #[repr(transparent)]
    pub struct Point(f64);

    impl Point {
        /**
        Treat a midpoint `f64` value as a `Point`.
        */
        pub const fn new(value: f64) -> Self {
            Point(value)
        }

        /**
        Parse a `Point` from its textual representation.
        */
        pub fn try_from_str(s: &str) -> Result<Self, ParsePointError> {
            Ok(Point::new(s.parse().map_err(|_| ParsePointError {})?))
        }

        /**
        Get the value of this midpoint as an `f64`.
        */
        pub const fn get(&self) -> f64 {
            self.0
        }

        /**
        Whether the sign of the midpoint is positive.
        */
        pub const fn is_sign_positive(&self) -> bool {
            self.get().is_sign_positive()
        }

        /**
        Whether the sign of the midpoint is negative.
        */
        pub const fn is_sign_negative(&self) -> bool {
            self.get().is_sign_negative()
        }

        /**
        Whether the midpoint is for the zero bucket.

        If this method returns `true`, then [`self.is_positive_bucket`] and [`self.is_negative_bucket`] will both return `false`.
        */
        pub const fn is_zero_bucket(&self) -> bool {
            self.get() == 0.0
        }

        /**
        Whether the midpoint belongs to a positive bucket.

        If this method returns `true`, then [`self.is_zero_bucket`] and [`self.is_negative_bucket`] will both return `false`.
        */
        pub const fn is_positive_bucket(&self) -> bool {
            self.is_indexable() && self.is_sign_positive()
        }

        /**
        Whether the midpoint belongs to a negative bucket.

        If this method returns `true`, then [`self.is_zero_bucket`] and [`self.is_positive_bucket`] will both return `false`.
        */
        pub const fn is_negative_bucket(&self) -> bool {
            self.is_indexable() && self.is_sign_negative()
        }

        /**
        Whether the midpoint can be represented as an exponential bucket index.

        A midpoint is considered indexable if:

        1. It is not `0` or `-0`.
        2. It is finite (not infinity or NaN).
        */
        pub const fn is_indexable(&self) -> bool {
            let value = self.get();

            value != 0.0 && value.is_finite()
        }
    }

    impl From<f64> for Point {
        fn from(value: f64) -> Self {
            Point::new(value)
        }
    }

    impl From<Point> for f64 {
        fn from(value: Point) -> Self {
            value.get()
        }
    }

    impl PartialEq for Point {
        fn eq(&self, other: &Self) -> bool {
            self.cmp(other) == cmp::Ordering::Equal
        }
    }

    impl Eq for Point {}

    impl PartialOrd for Point {
        fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
            Some(self.cmp(other))
        }
    }

    impl Ord for Point {
        fn cmp(&self, other: &Self) -> cmp::Ordering {
            libm::cmp(self.get()).cmp(&libm::cmp(other.get()))
        }
    }

    impl hash::Hash for Point {
        fn hash<H: hash::Hasher>(&self, state: &mut H) {
            libm::cmp(self.get()).hash(state)
        }
    }

    impl fmt::Debug for Point {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            fmt::Debug::fmt(&self.get(), f)
        }
    }

    impl fmt::Display for Point {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            fmt::Display::fmt(&self.get(), f)
        }
    }

    impl FromStr for Point {
        type Err = ParsePointError;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            Self::try_from_str(s)
        }
    }

    #[cfg(feature = "sval")]
    impl sval::Value for Point {
        fn stream<'sval, S: sval::Stream<'sval> + ?Sized>(
            &'sval self,
            stream: &mut S,
        ) -> sval::Result {
            stream.f64(self.get())
        }
    }

    #[cfg(feature = "serde")]
    impl serde::Serialize for Point {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            serializer.serialize_f64(self.get())
        }
    }

    impl ToValue for Point {
        fn to_value(&self) -> Value<'_> {
            Value::capture_display(self)
        }
    }

    impl<'v> FromValue<'v> for Point {
        fn from_value(value: Value<'v>) -> Option<Self>
        where
            Self: Sized,
        {
            value
                .downcast_ref::<Point>()
                .copied()
                .or_else(|| f64::from_value(value).map(Point::new))
        }
    }

    /**
    Compute γ, the base of an exponential histogram.

    The value of γ is a number close to 1, computed by `2^2^(-scale)`.
    The exponential bucket of a value, `v`, can be computed from γ by `⌈logγ(v)⌉`.

    # Implementation

    This function uses a portable implementation of `powf` that is consistent across platforms.
    You may also consider using a native port of it for performance reasons.
    */
    pub const fn gamma(scale: i32) -> f64 {
        libm::pow(2.0, libm::pow(2.0, -(scale as f64)))
    }

    /**
    Compute the exponential bucket midpoint for the given input value at a given scale.

    This function accepts the following parameters:

    - `value`: The observed sample value to be bucketed.
    - `scale`: The size of exponential buckets. Larger scales produce larger numbers of smaller buckets.

    This function can be used to compress an input data stream by feeding it input values and tracking the counts of resulting buckets.
    The choice of `scale` is a trade-off between size and accuracy.
    Larger buckets (smaller scales) count more unique input values in fewer unique bucket values, and resulting in higher compression but lower accuracy.

    This function uses the same `scale` as OpenTelemetry's metrics data model, but returns the midpoint of the bucket a value belongs to instead of its index.

    # Implementation

    This function uses a portable implementation of `powf` and `log` that is consistent across platforms.
    You may also consider using a native port of it for performance reasons.
    */
    pub const fn midpoint(value: f64, scale: i32) -> Point {
        let sign = value.signum();
        let value = value.abs();

        if value == 0.0 {
            return Point::new(value);
        }

        let gamma = gamma(scale);

        let index = libm::ceil(libm::log(value, gamma));

        let lower = libm::pow(gamma, index - 1.0);
        let upper = lower * gamma;

        Point::new(sign * lower.midpoint(upper))
    }

    #[cfg(feature = "alloc")]
    mod alloc_support {
        use super::*;

        use emit_core::{
            props::Props,
            str::{Str, ToStr},
            well_known::{
                KEY_DIST_COUNT, KEY_DIST_EXP_BUCKETS, KEY_DIST_EXP_SCALE, KEY_DIST_MAX,
                KEY_DIST_MIN, KEY_DIST_SUM,
            },
        };

        use crate::core::{cmp, ops::ControlFlow};

        pub mod bucket_set {
            /*!
            The [`BucketSet`] type.
            */

            use emit_core::value::{FromValue, ToValue, Value};

            use crate::{
                alloc::collections::{btree_map, BTreeMap},
                buf::{find, trim, trim_start},
                core::{
                    fmt::{self, Write as _},
                    str::FromStr,
                },
                metric::exp::Point,
            };

            /**
            An error encountered attempting to parse a [`BucketSet`].
            */
            #[derive(Debug)]
            pub struct ParseBucketSetError {}

            impl fmt::Display for ParseBucketSetError {
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    write!(f, "the input was not a valid bucket set")
                }
            }

            #[cfg(feature = "std")]
            impl std::error::Error for ParseBucketSetError {}

            /**
            A collection for buckets in an exponential histogram.

            The set stores sparse, sorted buckets as a tuple of their midpoint ([`Point`]), and count of occurrences.
            */
            #[derive(Clone, PartialEq, Eq)]
            pub struct BucketSet {
                total: u64,
                buckets: BTreeMap<Point, u64>,
            }

            impl fmt::Debug for BucketSet {
                fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    fmt::Display::fmt(self, f)
                }
            }

            impl fmt::Display for BucketSet {
                fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    f.write_char('[')?;

                    let mut first = true;
                    for (k, v) in &self.buckets {
                        if !first {
                            f.write_char(',')?;
                        }
                        first = false;

                        f.write_char('[')?;
                        fmt::Display::fmt(k, f)?;
                        f.write_char(',')?;
                        fmt::Display::fmt(v, f)?;
                        f.write_char(']')?;
                    }

                    f.write_char(']')
                }
            }

            impl BucketSet {
                /**
                Create a new empty `BucketSet`.

                This method does not allocate.
                */
                pub fn new() -> Self {
                    BucketSet {
                        buckets: BTreeMap::new(),
                        total: 0,
                    }
                }

                /**
                Parse a `BucketSet` from its raw textual representation.
                */
                pub fn try_from_str(s: &str) -> Result<Self, ParseBucketSetError> {
                    Self::try_from_slice(s.as_bytes())
                }

                fn try_from_slice(mut s: &[u8]) -> Result<Self, ParseBucketSetError> {
                    let mut set = BucketSet::new();

                    if s.len() < 2 {
                        // Truncated
                        return Err(ParseBucketSetError {});
                    }

                    // Must be enclosed by `[]`, `()`, or `{}`
                    let container_end = match (s.first(), s.last()) {
                        (Some(&b'['), Some(&b']')) => b']',
                        (Some(&b'('), Some(&b')')) => b')',
                        (Some(&b'{'), Some(&b'}')) => b'}',
                        _ => return Err(ParseBucketSetError {}),
                    };
                    s = &s[1..];
                    s = trim_start(s);

                    let mut first = true;
                    while s.len() > 1 {
                        // Parse each bucket
                        if !first {
                            if s.first() != Some(&b',') {
                                // Invalid bucket: expected `,`
                                return Err(ParseBucketSetError {});
                            }
                            s = &s[1..];
                            s = trim_start(s);
                        }
                        first = false;

                        // Determine the kind of bucket we're parsing
                        let (key_start_skip, key_end, value_end): (
                            usize,
                            &[(u8, u8)],
                            &[(u8, u8)],
                        ) = match s.first() {
                            // `[k, v]`, `[k: v]`, or `[k = v]`
                            Some(&b'[') => {
                                (1, &[(b',', 1u8), (b':', 1u8), (b'=', 1u8)], &[(b']', 1u8)])
                            }
                            // `(k, v)`, `(k: v)`, or `(k = v)`
                            Some(&b'(') => {
                                (1, &[(b',', 1u8), (b':', 1u8), (b'=', 1u8)], &[(b')', 1u8)])
                            }
                            // `{k, v}`, `{k: v}`, or `{k = v}`
                            Some(&b'{') => {
                                (1, &[(b',', 1u8), (b':', 1u8), (b'=', 1u8)], &[(b'}', 1u8)])
                            }
                            // `k: v`, or `k = v`
                            _ => (
                                0,
                                &[(b':', 1u8), (b'=', 1u8)],
                                &[(b',', 0u8), (container_end, 0u8)],
                            ),
                        };
                        s = &s[key_start_skip..];

                        // Find the bounds of the key
                        let Some((key_end, key_end_skip)) = find(s, key_end) else {
                            // Unexpected EOF parsing key: expected `$key_end`
                            return Err(ParseBucketSetError {});
                        };

                        let key = str::from_utf8(trim(&s[..key_end]))
                            .map_err(|_| ParseBucketSetError {})?;
                        s = &s[key_end + key_end_skip..];

                        // Find the bounds of the value
                        let Some((value_end, value_end_skip)) = find(s, value_end) else {
                            // Unexpected EOF parsing value: expected `$value_end`
                            return Err(ParseBucketSetError {});
                        };

                        let value = str::from_utf8(trim(&s[..value_end]))
                            .map_err(|_| ParseBucketSetError {})?;
                        s = &s[value_end + value_end_skip..];

                        // Parse the key and value
                        let key = key.parse().map_err(|_| ParseBucketSetError {})?;
                        let value = value.parse().map_err(|_| ParseBucketSetError {})?;

                        set.total = set
                            .total
                            .checked_add(value)
                            .ok_or_else(|| ParseBucketSetError {})?;
                        if set.buckets.insert(key, value).is_some() {
                            // Duplicate key
                            return Err(ParseBucketSetError {});
                        }

                        s = trim_start(s);
                    }

                    if s.len() != 1 {
                        // Unexpected EOF
                        return Err(ParseBucketSetError {});
                    }

                    Ok(set)
                }

                /**
                Observe a [`Point`] computed from a raw value.

                The count for this point will be incremented by `1`.

                All points should be computed from the same scale.

                # Panics

                This method will panic if adding `count` to an existing entry in `value` would overflow.
                */
                pub fn observe(&mut self, value: Point) {
                    self.observe_all(value, 1)
                }

                /**
                Observe `count` instances of a [`Point`] computed from a raw value.

                The count for this point will be incremented by `count`.

                All points should be computed from the same scale.

                # Panics

                This method will panic if adding `count` to an existing entry in `value` would overflow.
                */
                pub fn observe_all(&mut self, value: Point, count: u64) {
                    let entry = self.buckets.entry(value).or_default();

                    *entry = entry.checked_add(count).unwrap_or_else(|| {
                        panic!("adding {count} observations would overflow bucket")
                    });
                    self.total = self.total.checked_add(count).unwrap_or_else(|| {
                        panic!("adding {count} observations would overflow total")
                    });
                }

                /**
                Remap and merge buckets.

                This method can be used to rescale the buckets to a coarser granularity in combination with the [`crate::metric::exp::midpoint`] function. It accepts a closure that maps stored bucket [`Point`]s to new values. When multiple buckets map to the same new value, their counts will be summed.

                # Panics

                This method will panic if adding `count` to an existing entry in `value` would overflow. This can happen when merging buckets with large counts.
                */
                pub fn remap(&mut self, mut map: impl FnMut(Point) -> Point) {
                    let mut remapped = BTreeMap::<Point, u64>::new();

                    for (value, count) in &self.buckets {
                        let entry = remapped.entry(map(*value)).or_default();

                        *entry = entry.checked_add(*count).unwrap_or_else(|| {
                            panic!("adding {count} observations would overflow bucket")
                        });
                    }

                    self.buckets = remapped;
                }

                /**
                Get the number of buckets currently in the set.

                This is **not** the total count of observed values in all buckets, just the count of buckets themselves. See [`BucketSet::total`] for the total count.
                */
                pub fn len(&self) -> usize {
                    self.buckets.len()
                }

                /**
                Get the total count of observed values.
                */
                pub fn total(&self) -> u64 {
                    self.total
                }

                /**
                Clear all buckets, allowing the allocation to be re-used.
                */
                pub fn clear(&mut self) {
                    self.buckets.clear();
                    self.total = 0;
                }

                /**
                Get the count for a particular bucket.
                */
                pub fn get(&self, value: Point) -> Option<u64> {
                    self.buckets.get(&value).copied()
                }

                /**
                Get the first (lowest numbered) bucket.
                */
                pub fn first(&self) -> Option<(Point, u64)> {
                    self.buckets.first_key_value().map(|(k, v)| (*k, *v))
                }

                /**
                Get the last (highest numbered) bucket.
                */
                pub fn last(&self) -> Option<(Point, u64)> {
                    self.buckets.last_key_value().map(|(k, v)| (*k, *v))
                }

                /**
                Iterate over buckets in order.
                */
                pub fn iter(&self) -> Iter<'_> {
                    Iter(self.buckets.iter())
                }
            }

            impl<'a> IntoIterator for &'a BucketSet {
                type IntoIter = Iter<'a>;
                type Item = (Point, u64);

                fn into_iter(self) -> Self::IntoIter {
                    self.iter()
                }
            }

            impl<'a> FromIterator<(Point, u64)> for BucketSet {
                fn from_iter<I: IntoIterator<Item = (Point, u64)>>(iter: I) -> Self {
                    let mut set = BucketSet::new();
                    set.extend(iter);

                    set
                }
            }

            impl<'a> Extend<(Point, u64)> for BucketSet {
                fn extend<I: IntoIterator<Item = (Point, u64)>>(&mut self, iter: I) {
                    for (value, count) in iter {
                        self.observe_all(value, count);
                    }
                }
            }

            /**
            An iterator over sorted buckets from a [`BucketSet`].

            This method is the result of calling [`BucketSet::iter`].
            */
            pub struct Iter<'a>(btree_map::Iter<'a, Point, u64>);

            impl<'a> Iterator for Iter<'a> {
                type Item = (Point, u64);

                fn next(&mut self) -> Option<Self::Item> {
                    self.0.next().map(|(k, v)| (*k, *v))
                }
            }

            #[cfg(feature = "sval")]
            impl sval::Value for BucketSet {
                fn stream<'sval, S: sval::Stream<'sval> + ?Sized>(
                    &'sval self,
                    stream: &mut S,
                ) -> sval::Result {
                    stream.seq_begin(Some(self.buckets.len()))?;

                    for bucket in &self.buckets {
                        stream.value_computed(&bucket)?;
                    }

                    stream.seq_end()
                }
            }

            #[cfg(feature = "serde")]
            impl serde::Serialize for BucketSet {
                fn serialize<S: serde::Serializer>(
                    &self,
                    serializer: S,
                ) -> Result<S::Ok, S::Error> {
                    use serde::ser::SerializeSeq as _;

                    let mut seq = serializer.serialize_seq(Some(self.buckets.len()))?;

                    for bucket in &self.buckets {
                        seq.serialize_element(&bucket)?;
                    }

                    seq.end()
                }
            }

            impl FromStr for BucketSet {
                type Err = ParseBucketSetError;

                fn from_str(s: &str) -> Result<Self, Self::Err> {
                    Self::try_from_str(s)
                }
            }

            impl ToValue for BucketSet {
                fn to_value(&self) -> Value<'_> {
                    #[cfg(feature = "sval")]
                    {
                        Value::capture_sval(self)
                    }
                    #[cfg(all(feature = "serde", not(feature = "sval")))]
                    {
                        Value::capture_serde(self)
                    }
                    #[cfg(all(not(feature = "serde"), not(feature = "sval")))]
                    {
                        Value::capture_display(self)
                    }
                }
            }

            impl<'a> FromValue<'a> for BucketSet {
                fn from_value(v: Value<'a>) -> Option<Self> {
                    if let Some(buckets) = v.downcast_ref::<Self>() {
                        return Some(buckets.clone());
                    }

                    #[cfg(feature = "sval")]
                    {
                        if let Some(buckets) = from_sval(v.by_ref()) {
                            return Some(buckets);
                        }
                    }

                    #[cfg(all(not(feature = "sval"), feature = "serde"))]
                    {
                        if let Some(buckets) = from_serde(v.by_ref()) {
                            return Some(buckets);
                        }
                    }

                    v.parse()
                }
            }

            #[cfg(any(feature = "sval", feature = "serde"))]
            #[derive(Default)]
            struct Extract {
                depth: usize,
                buckets: BTreeMap<Point, u64>,
                count: u64,
                next_midpoint: Option<f64>,
                next_count: Option<u64>,
            }

            #[derive(Debug)]
            #[cfg(any(feature = "sval", feature = "serde"))]
            struct Incompatible;

            #[cfg(any(feature = "sval", feature = "serde"))]
            impl Extract {
                fn push(
                    &mut self,
                    midpoint: impl FnOnce() -> Option<f64>,
                    count: impl FnOnce() -> Option<u64>,
                ) -> Result<(), Incompatible> {
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

                    Err(Incompatible)
                }

                fn apply(&mut self) -> Result<(), Incompatible> {
                    if self.depth == 2 {
                        let midpoint = self.next_midpoint.take().ok_or(Incompatible)?;
                        let count = self.next_count.take().ok_or(Incompatible)?;

                        let entry = self.buckets.entry(Point::new(midpoint)).or_default();
                        *entry = entry.checked_add(count).ok_or_else(|| Incompatible)?;

                        self.count = self.count.checked_add(count).ok_or_else(|| Incompatible)?;

                        Ok(())
                    } else {
                        Ok(())
                    }
                }

                fn down(&mut self) -> Result<(), Incompatible> {
                    self.depth += 1;

                    if self.depth > 2 {
                        Err(Incompatible)
                    } else {
                        Ok(())
                    }
                }

                fn up(&mut self) -> Result<(), Incompatible> {
                    self.apply()?;
                    self.depth -= 1;

                    Ok(())
                }

                fn end(self) -> BucketSet {
                    BucketSet {
                        buckets: self.buckets,
                        total: self.count,
                    }
                }
            }

            #[cfg(feature = "sval")]
            fn from_sval(value: Value) -> Option<BucketSet> {
                #[allow(non_local_definitions)]
                impl From<Incompatible> for sval::Error {
                    fn from(_: Incompatible) -> sval::Error {
                        sval::Error::new()
                    }
                }

                #[allow(non_local_definitions)]
                impl<'sval> sval::Stream<'sval> for Extract {
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
                        Ok(self.push(|| Some(value as f64), || value.try_into().ok())?)
                    }

                    fn u64(&mut self, value: u64) -> sval::Result {
                        Ok(self.push(|| Some(value as f64), || Some(value))?)
                    }

                    fn i128(&mut self, value: i128) -> sval::Result {
                        Ok(self.push(|| Some(value as f64), || value.try_into().ok())?)
                    }

                    fn u128(&mut self, value: u128) -> sval::Result {
                        Ok(self.push(|| Some(value as f64), || value.try_into().ok())?)
                    }

                    fn f64(&mut self, value: f64) -> sval::Result {
                        Ok(self.push(|| Some(value), || Some(value as u64))?)
                    }

                    fn seq_begin(&mut self, _: Option<usize>) -> sval::Result {
                        Ok(self.down()?)
                    }

                    fn seq_value_begin(&mut self) -> sval::Result {
                        Ok(())
                    }

                    fn seq_value_end(&mut self) -> sval::Result {
                        Ok(())
                    }

                    fn seq_end(&mut self) -> sval::Result {
                        Ok(self.up()?)
                    }
                }

                let mut extract = Extract::default();
                sval::stream(&mut extract, &value).ok()?;

                Some(extract.end())
            }

            #[cfg(all(not(feature = "sval"), feature = "serde"))]
            fn from_serde(value: Value) -> Option<BucketSet> {
                use serde::Serialize as _;

                #[allow(non_local_definitions)]
                impl fmt::Display for Incompatible {
                    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                        f.write_str("incompatible")
                    }
                }

                #[allow(non_local_definitions)]
                impl serde::ser::StdError for Incompatible {}

                #[allow(non_local_definitions)]
                impl serde::ser::Error for Incompatible {
                    fn custom<T>(_: T) -> Self
                    where
                        T: fmt::Display,
                    {
                        Incompatible
                    }
                }

                #[allow(non_local_definitions)]
                impl<'a> serde::Serializer for &'a mut Extract {
                    type Ok = ();
                    type Error = Incompatible;
                    type SerializeSeq = Self;
                    type SerializeTuple = Self;
                    type SerializeTupleStruct = Self;
                    type SerializeTupleVariant = Self;
                    type SerializeMap = Self;
                    type SerializeStruct = Self;
                    type SerializeStructVariant = Self;

                    fn serialize_bool(self, _: bool) -> Result<Self::Ok, Self::Error> {
                        Err(Incompatible)
                    }

                    fn serialize_i8(self, value: i8) -> Result<Self::Ok, Self::Error> {
                        self.push(|| Some(value as f64), || value.try_into().ok())
                    }

                    fn serialize_i16(self, value: i16) -> Result<Self::Ok, Self::Error> {
                        self.push(|| Some(value as f64), || value.try_into().ok())
                    }

                    fn serialize_i32(self, value: i32) -> Result<Self::Ok, Self::Error> {
                        self.push(|| Some(value as f64), || value.try_into().ok())
                    }

                    fn serialize_i64(self, value: i64) -> Result<Self::Ok, Self::Error> {
                        self.push(|| Some(value as f64), || value.try_into().ok())
                    }

                    fn serialize_u8(self, value: u8) -> Result<Self::Ok, Self::Error> {
                        self.push(|| Some(value as f64), || value.try_into().ok())
                    }

                    fn serialize_u16(self, value: u16) -> Result<Self::Ok, Self::Error> {
                        self.push(|| Some(value as f64), || value.try_into().ok())
                    }

                    fn serialize_u32(self, value: u32) -> Result<Self::Ok, Self::Error> {
                        self.push(|| Some(value as f64), || value.try_into().ok())
                    }

                    fn serialize_u64(self, value: u64) -> Result<Self::Ok, Self::Error> {
                        self.push(|| Some(value as f64), || value.try_into().ok())
                    }

                    fn serialize_u128(self, value: u128) -> Result<Self::Ok, Self::Error> {
                        self.push(|| Some(value as f64), || value.try_into().ok())
                    }

                    fn serialize_i128(self, value: i128) -> Result<Self::Ok, Self::Error> {
                        self.push(|| Some(value as f64), || value.try_into().ok())
                    }

                    fn serialize_f32(self, value: f32) -> Result<Self::Ok, Self::Error> {
                        self.push(|| Some(value as f64), || Some(value as u64))
                    }

                    fn serialize_f64(self, value: f64) -> Result<Self::Ok, Self::Error> {
                        self.push(|| Some(value as f64), || Some(value as u64))
                    }

                    fn serialize_char(self, _: char) -> Result<Self::Ok, Self::Error> {
                        Err(Incompatible)
                    }

                    fn serialize_str(self, _: &str) -> Result<Self::Ok, Self::Error> {
                        Err(Incompatible)
                    }

                    fn serialize_bytes(self, _: &[u8]) -> Result<Self::Ok, Self::Error> {
                        Err(Incompatible)
                    }

                    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
                        Err(Incompatible)
                    }

                    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
                    where
                        T: ?Sized + serde::Serialize,
                    {
                        value.serialize(self)
                    }

                    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
                        Err(Incompatible)
                    }

                    fn serialize_unit_struct(
                        self,
                        name: &'static str,
                    ) -> Result<Self::Ok, Self::Error> {
                        name.serialize(self)
                    }

                    fn serialize_unit_variant(
                        self,
                        _: &'static str,
                        _: u32,
                        variant: &'static str,
                    ) -> Result<Self::Ok, Self::Error> {
                        variant.serialize(self)
                    }

                    fn serialize_newtype_struct<T>(
                        self,
                        _: &'static str,
                        value: &T,
                    ) -> Result<Self::Ok, Self::Error>
                    where
                        T: ?Sized + serde::Serialize,
                    {
                        value.serialize(self)
                    }

                    fn serialize_newtype_variant<T>(
                        self,
                        _: &'static str,
                        _: u32,
                        _: &'static str,
                        value: &T,
                    ) -> Result<Self::Ok, Self::Error>
                    where
                        T: ?Sized + serde::Serialize,
                    {
                        value.serialize(self)
                    }

                    fn serialize_seq(
                        self,
                        _: Option<usize>,
                    ) -> Result<Self::SerializeSeq, Self::Error> {
                        self.down()?;

                        Ok(self)
                    }

                    fn serialize_tuple(
                        self,
                        _: usize,
                    ) -> Result<Self::SerializeTuple, Self::Error> {
                        self.down()?;

                        Ok(self)
                    }

                    fn serialize_tuple_struct(
                        self,
                        _: &'static str,
                        _: usize,
                    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
                        self.down()?;

                        Ok(self)
                    }

                    fn serialize_tuple_variant(
                        self,
                        _: &'static str,
                        _: u32,
                        _: &'static str,
                        _: usize,
                    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
                        self.down()?;

                        Ok(self)
                    }

                    fn serialize_map(
                        self,
                        _: Option<usize>,
                    ) -> Result<Self::SerializeMap, Self::Error> {
                        self.down()?;

                        Ok(self)
                    }

                    fn serialize_struct(
                        self,
                        _: &'static str,
                        _: usize,
                    ) -> Result<Self::SerializeStruct, Self::Error> {
                        self.down()?;

                        Ok(self)
                    }

                    fn serialize_struct_variant(
                        self,
                        _: &'static str,
                        _: u32,
                        _: &'static str,
                        _: usize,
                    ) -> Result<Self::SerializeStructVariant, Self::Error> {
                        self.down()?;

                        Ok(self)
                    }
                }

                #[allow(non_local_definitions)]
                impl<'a> serde::ser::SerializeSeq for &'a mut Extract {
                    type Ok = ();
                    type Error = Incompatible;

                    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
                    where
                        T: ?Sized + serde::Serialize,
                    {
                        value.serialize(&mut **self)
                    }

                    fn end(self) -> Result<Self::Ok, Self::Error> {
                        self.up()
                    }
                }

                #[allow(non_local_definitions)]
                impl<'a> serde::ser::SerializeTuple for &'a mut Extract {
                    type Ok = ();
                    type Error = Incompatible;

                    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
                    where
                        T: ?Sized + serde::Serialize,
                    {
                        value.serialize(&mut **self)
                    }

                    fn end(self) -> Result<Self::Ok, Self::Error> {
                        self.up()
                    }
                }

                #[allow(non_local_definitions)]
                impl<'a> serde::ser::SerializeTupleStruct for &'a mut Extract {
                    type Ok = ();
                    type Error = Incompatible;

                    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
                    where
                        T: ?Sized + serde::Serialize,
                    {
                        value.serialize(&mut **self)
                    }

                    fn end(self) -> Result<Self::Ok, Self::Error> {
                        self.up()
                    }
                }

                #[allow(non_local_definitions)]
                impl<'a> serde::ser::SerializeTupleVariant for &'a mut Extract {
                    type Ok = ();
                    type Error = Incompatible;

                    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
                    where
                        T: ?Sized + serde::Serialize,
                    {
                        value.serialize(&mut **self)
                    }

                    fn end(self) -> Result<Self::Ok, Self::Error> {
                        self.up()
                    }
                }

                #[allow(non_local_definitions)]
                impl<'a> serde::ser::SerializeMap for &'a mut Extract {
                    type Ok = ();
                    type Error = Incompatible;

                    fn serialize_key<T>(&mut self, key: &T) -> Result<(), Self::Error>
                    where
                        T: ?Sized + serde::Serialize,
                    {
                        self.down()?;
                        key.serialize(&mut **self)
                    }

                    fn serialize_value<T>(&mut self, value: &T) -> Result<(), Self::Error>
                    where
                        T: ?Sized + serde::Serialize,
                    {
                        value.serialize(&mut **self)?;
                        self.up()
                    }

                    fn end(self) -> Result<Self::Ok, Self::Error> {
                        self.up()
                    }
                }

                #[allow(non_local_definitions)]
                impl<'a> serde::ser::SerializeStruct for &'a mut Extract {
                    type Ok = ();
                    type Error = Incompatible;

                    fn serialize_field<T>(
                        &mut self,
                        key: &'static str,
                        value: &T,
                    ) -> Result<(), Self::Error>
                    where
                        T: ?Sized + serde::Serialize,
                    {
                        self.down()?;
                        key.serialize(&mut **self)?;
                        value.serialize(&mut **self)?;
                        self.up()
                    }

                    fn end(self) -> Result<Self::Ok, Self::Error> {
                        self.up()
                    }
                }

                #[allow(non_local_definitions)]
                impl<'a> serde::ser::SerializeStructVariant for &'a mut Extract {
                    type Ok = ();
                    type Error = Incompatible;

                    fn serialize_field<T>(
                        &mut self,
                        key: &'static str,
                        value: &T,
                    ) -> Result<(), Self::Error>
                    where
                        T: ?Sized + serde::Serialize,
                    {
                        self.down()?;
                        key.serialize(&mut **self)?;
                        value.serialize(&mut **self)?;
                        self.up()
                    }

                    fn end(self) -> Result<Self::Ok, Self::Error> {
                        self.up()
                    }
                }

                let mut extract = Extract::default();
                value.serialize(&mut extract).ok()?;

                Some(extract.end())
            }

            #[cfg(test)]
            mod tests {
                use super::*;

                use std::collections::{BTreeMap, BTreeSet};

                #[test]
                fn bucket_set_observe() {
                    let mut set = BucketSet::new();

                    assert_eq!(0, set.len());

                    set.observe(Point::new(0.0));
                    set.observe_all(Point::new(0.0), 2);
                    set.observe_all(Point::new(1.0), 2);

                    assert_eq!(2, set.len());
                    assert_eq!(3, set.get(Point::new(0.0)).unwrap());
                    assert_eq!(2, set.get(Point::new(1.0)).unwrap());

                    assert_eq!((Point::new(0.0), 3), set.first().unwrap());
                    assert_eq!((Point::new(1.0), 2), set.last().unwrap());
                }

                #[test]
                fn bucket_set_remap() {
                    let mut set = BucketSet::new();

                    set.observe_all(Point::new(0.0), 3);
                    set.observe_all(Point::new(1.0), 2);

                    assert_eq!(2, set.len());

                    set.remap(|_| Point::new(2.0));

                    assert_eq!(1, set.len());
                    assert_eq!(5, set.get(Point::new(2.0)).unwrap());
                }

                #[test]
                fn bucket_set_roundtrip() {
                    for case in [
                        BucketSet::new(),
                        {
                            let mut set = BucketSet::new();
                            set.observe(Point::new(0.0));
                            set
                        },
                        {
                            let mut set = BucketSet::new();
                            set.observe(Point::new(0.0));
                            set.observe(Point::new(1.0));
                            set
                        },
                    ] {
                        let fmt = case.to_string();
                        assert_eq!(Some(case), BucketSet::try_from_str(&fmt).ok(), "{fmt}");
                    }
                }

                #[test]
                fn bucket_set_from_iter() {
                    let mut set = BucketSet::from_iter([
                        (Point::new(0.0), 3),
                        (Point::new(0.0), 2),
                        (Point::new(1.0), 2),
                    ]);

                    assert_eq!(5, set.get(Point::new(0.0)).unwrap());
                    assert_eq!(2, set.get(Point::new(1.0)).unwrap());

                    set.extend([(Point::new(1.0), 3), (Point::new(2.0), 2)]);

                    assert_eq!(5, set.get(Point::new(1.0)).unwrap());
                    assert_eq!(2, set.get(Point::new(2.0)).unwrap());
                }

                #[test]
                fn bucket_set_parse() {
                    for (case, expected) in [
                        (format!("{:?}", ([[1, 1], [2, 2]])), {
                            let mut set = BucketSet::new();
                            set.observe_all(Point::new(1.0), 1);
                            set.observe_all(Point::new(2.0), 2);
                            set
                        }),
                        (format!("{:?}", ([(1.0, 1), (2.0, 2)])), {
                            let mut set = BucketSet::new();
                            set.observe_all(Point::new(1.0), 1);
                            set.observe_all(Point::new(2.0), 2);
                            set
                        }),
                        (
                            format!("{:?}", {
                                let mut set = BTreeSet::new();
                                set.insert((1, 1));
                                set.insert((2, 2));
                                set
                            }),
                            {
                                let mut set = BucketSet::new();
                                set.observe_all(Point::new(1.0), 1);
                                set.observe_all(Point::new(2.0), 2);
                                set
                            },
                        ),
                        (
                            format!("{:?}", {
                                let mut set = BTreeMap::new();
                                set.insert(1, 1);
                                set.insert(2, 2);
                                set
                            }),
                            {
                                let mut set = BucketSet::new();
                                set.observe_all(Point::new(1.0), 1);
                                set.observe_all(Point::new(2.0), 2);
                                set
                            },
                        ),
                        ("[ [ 1 , 1 ] , [ 2 , 2 ] ]".to_string(), {
                            let mut set = BucketSet::new();
                            set.observe_all(Point::new(1.0), 1);
                            set.observe_all(Point::new(2.0), 2);
                            set
                        }),
                        ("[ 1 : 1 , 2 : 2 ]".to_string(), {
                            let mut set = BucketSet::new();
                            set.observe_all(Point::new(1.0), 1);
                            set.observe_all(Point::new(2.0), 2);
                            set
                        }),
                    ] {
                        assert_eq!(
                            Some(expected),
                            BucketSet::try_from_str(&case).ok(),
                            "{case}"
                        );
                    }
                }

                #[test]
                fn bucket_set_parse_exotic() {
                    for case in ["[[inf,1]]", "[[nan,1]]", "[(1, 1), [2, 1], {3: 1}]"] {
                        assert!(BucketSet::try_from_str(case).is_ok());
                    }
                }

                #[test]
                fn bucket_set_to_from_value() {
                    for case in [{
                        let mut set = BucketSet::new();
                        set.observe_all(Point::new(1.0), 1);
                        set.observe_all(Point::new(2.0), 2);
                        set
                    }] {
                        assert_eq!(case, BucketSet::from_value(case.to_value()).unwrap());
                    }
                }

                #[test]
                fn bucket_set_from_value_string() {
                    for (case, expected) in [("[[1.0,1],[2.0,2]]", {
                        let mut set = BucketSet::new();
                        set.observe_all(Point::new(1.0), 1);
                        set.observe_all(Point::new(2.0), 2);
                        set
                    })] {
                        assert_eq!(expected, Value::from(case).cast().unwrap());
                    }
                }

                #[test]
                fn bucket_set_from_value_structured() {
                    #[cfg(feature = "sval")]
                    trait CaseSval: sval::Value {}
                    #[cfg(feature = "sval")]
                    impl<T: sval::Value> CaseSval for T {}
                    #[cfg(not(feature = "sval"))]
                    trait CaseSval {}
                    #[cfg(not(feature = "sval"))]
                    impl<T> CaseSval for T {}

                    #[cfg(feature = "serde")]
                    trait CaseSerde: serde::Serialize {}
                    #[cfg(feature = "serde")]
                    impl<T: serde::Serialize> CaseSerde for T {}
                    #[cfg(not(feature = "serde"))]
                    trait CaseSerde {}
                    #[cfg(not(feature = "serde"))]
                    impl<T> CaseSerde for T {}

                    trait Case: CaseSval + CaseSerde + fmt::Debug {}
                    impl<T: fmt::Debug + CaseSval + CaseSerde> Case for T {}

                    fn case(case: &impl Case, expected: &BucketSet) {
                        assert_eq!(expected, &Value::from_debug(case).cast().unwrap());

                        #[cfg(feature = "sval")]
                        {
                            assert_eq!(expected, &Value::from_sval(case).cast().unwrap());
                        }

                        #[cfg(feature = "serde")]
                        {
                            assert_eq!(expected, &Value::from_serde(case).cast().unwrap());
                        }
                    }

                    let mut set = BucketSet::new();
                    set.observe_all(Point::new(1.0), 1);
                    set.observe_all(Point::new(2.0), 2);

                    case(&[[1, 1], [2, 2]], &set);
                    case(&[(1.0, 1), (2.0, 2)], &set);
                    case(
                        &{
                            let mut set = BTreeSet::new();
                            set.insert((1, 1));
                            set.insert((2, 2));
                            set
                        },
                        &set,
                    );
                    case(
                        &{
                            let mut set = BTreeMap::new();
                            set.insert(1, 1);
                            set.insert(2, 2);
                            set
                        },
                        &set,
                    );
                }

                #[test]
                fn err_bucket_set_invalid() {
                    for case in [
                        "",
                        "<>",
                        "[1, 1]",
                        "1, 1",
                        "[[1, 1]], [[2, 1]]",
                        "[[]]",
                        "[}",
                        "[[1,1}]",
                        "[[1 1]]",
                        "[[1, 1] [2, 1]]",
                        "[,]",
                        "[[,]]",
                        "[[1,]]",
                        "[[,1]]",
                        "[[:]]",
                        "[[1:]]",
                        "[[:1]]",
                        "[[1, -1]]",
                        "[[1, 1.0]]",
                        "[[1, 0xff]]",
                        "[[1, ff]]",
                        "{1.2789: 11111111111111111111, 2789: 11111111111111111111, 2 \0:  \0: 2}",
                    ] {
                        assert!(BucketSet::try_from_str(case).is_err(), "{case}");
                    }
                }
            }
        }

        pub use self::bucket_set::BucketSet;

        /**
        A container for approximating the distribution of a streaming data source.

        `Distribution`s aggregate statistics from raw samples that pass through them. They include:

        - `total`: The total number of observed values.
        - `sum`: The sum of all observed values.
        - `min`: The smallest observed value.
        - `max`: The largest observed value.
        - `buckets`: An exponential histogram backed by a [`BucketSet`].

        Call the [`Distribution::observe`] method on each raw value.

        Use the [`Props`] implementation on `Distribution` to include it on a metric sample.
        */
        pub struct Distribution {
            max_buckets: usize,
            max_scale: i32,
            scale: i32,
            sum: Option<f64>,
            min: Option<f64>,
            max: Option<f64>,
            buckets: BucketSet,
        }

        impl Default for Distribution {
            fn default() -> Self {
                Self::new(Self::DEFAULT_MAX_SCALE, Self::DEFAULT_MAX_BUCKETS)
            }
        }

        impl Distribution {
            /**
            The default initial scale used when converting observed values into bucket midpoints.
            */
            pub const DEFAULT_MAX_SCALE: i32 = 20;

            /**
            The default maximum number of buckets before rescaling will apply.
            */
            pub const DEFAULT_MAX_BUCKETS: usize = 160;

            /**
            Create a new `Distribution` that can store up to `max_buckets` sparse buckets, at up to `max_scale` precision.

            The distribution uses a large scale initially. Whenever the number of buckets would overflow `max_buckets`, the scale is decremented and the buckets are rescaled. This reduces the number of buckets by half while also decreasing precision.
            */
            pub fn new(max_scale: i32, max_buckets: usize) -> Self {
                Distribution {
                    max_buckets,
                    max_scale,
                    scale: max_scale,
                    min: None,
                    max: None,
                    sum: None,
                    buckets: BucketSet::new(),
                }
            }

            /**
            Observe a raw value.

            The value will be converted into a bucket midpoint at the current internal scale. The count for the resulting bucket will be incremented by `1`.

            # Panics

            This method will panic if adding `count` to an existing entry in `value` would overflow.
            */
            pub fn observe(&mut self, raw_value: f64) {
                self.observe_all(raw_value, 1)
            }

            /**
            Observe a raw value.

            The value will be converted into a bucket midpoint at the current internal scale. The count for the resulting bucket will be incremented by `1`.

            # Panics

            This method will panic if adding `count` to an existing entry in `value` would overflow.
            */
            pub fn observe_all(&mut self, raw_value: f64, count: u64) {
                self.buckets
                    .observe_all(midpoint(raw_value, self.scale), count);

                // Track the extrema
                self.min = self
                    .min
                    .map(|min| cmp::min_by(min, raw_value, |a, b| a.total_cmp(b)))
                    .or(Some(raw_value));
                self.max = self
                    .max
                    .map(|max| cmp::max_by(max, raw_value, |a, b| a.total_cmp(b)))
                    .or(Some(raw_value));
                self.sum = self.sum.map(|sum| sum + raw_value).or(Some(raw_value));

                // If we've overflowed then reduce our scale and resample
                // Each time `scale` is decremented, our number of buckets will be halved
                if self.buckets.len() > self.max_buckets {
                    self.scale -= 1;
                    self.buckets
                        .remap(|value| midpoint(value.get(), self.scale));
                }
            }

            /**
            Clear the distribution of any data so it can be re-used.

            This method will also reset the internal scale back to its initial value.
            */
            pub fn reset(&mut self) {
                let Distribution {
                    max_scale,
                    max_buckets: _,
                    scale,
                    min,
                    max,
                    sum,
                    buckets,
                } = self;

                buckets.clear();
                *min = None;
                *max = None;
                *sum = None;
                *scale = *max_scale;
            }

            /**
            Get the total count of observed values across all buckets.

            This method returns `0` if no values have been seen.
            */
            pub fn count(&self) -> u64 {
                self.buckets.total()
            }

            /**
            Get the minimum observed value.

            This method returns `None` if no values have been seen.
            */
            pub fn min(&self) -> Option<f64> {
                self.min
            }

            /**
            Get the maximum observed value.

            This method returns `None` if no values have been seen.
            */
            pub fn max(&self) -> Option<f64> {
                self.max
            }

            /**
            Get the sum of all observed values.

            This method returns `None` if no values have been seen.
            */
            pub fn sum(&self) -> Option<f64> {
                self.sum
            }

            /**
            Get the current scale used to bucket values.
            */
            pub fn scale(&self) -> i32 {
                self.scale
            }

            /**
            Get the bucket values.
            */
            pub fn buckets(&self) -> &BucketSet {
                &self.buckets
            }

            /**
            Get the maximum number of buckets the distribution can hold before rescaling.
            */
            pub fn max_buckets(&self) -> usize {
                self.max_buckets
            }

            /**
            Get the maximum scale the distribution can use.
            */
            pub fn max_scale(&self) -> i32 {
                self.max_scale
            }
        }

        impl Props for Distribution {
            fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
                &'kv self,
                mut for_each: F,
            ) -> ControlFlow<()> {
                for_each(KEY_DIST_EXP_SCALE.to_str(), self.scale().into())?;
                for_each(KEY_DIST_EXP_BUCKETS.to_str(), self.buckets().to_value())?;

                for_each(KEY_DIST_COUNT.to_str(), self.count().into())?;

                if let Some(sum) = self.sum() {
                    for_each(KEY_DIST_SUM.to_str(), sum.into())?;
                }
                if let Some(min) = self.min() {
                    for_each(KEY_DIST_MIN.to_str(), min.into())?;
                }
                if let Some(max) = self.max() {
                    for_each(KEY_DIST_MAX.to_str(), max.into())?;
                }

                ControlFlow::Continue(())
            }
        }

        #[cfg(test)]
        mod tests {
            use super::*;

            #[test]
            fn distribution_observe() {
                let mut distribution = Distribution::new(10, 10);

                assert_eq!(distribution.max_scale(), distribution.scale());
                assert_eq!(0, distribution.buckets().len());
                assert_eq!(None, distribution.min());
                assert_eq!(None, distribution.max());
                assert_eq!(None, distribution.sum());
                assert_eq!(0, distribution.count());

                distribution.observe(1.0);
                distribution.observe(1.0);

                assert_eq!(
                    2,
                    distribution
                        .buckets()
                        .get(midpoint(1.0, distribution.max_scale()))
                        .unwrap()
                );
                assert_eq!(1, distribution.buckets().len());
                assert_eq!(Some(1.0), distribution.min());
                assert_eq!(Some(1.0), distribution.max());
                assert_eq!(Some(2.0), distribution.sum());
                assert_eq!(2, distribution.count());

                distribution.reset();

                assert_eq!(distribution.max_scale(), distribution.scale());
                assert_eq!(0, distribution.buckets().len());
                assert_eq!(None, distribution.min());
                assert_eq!(None, distribution.max());
                assert_eq!(None, distribution.sum());
                assert_eq!(0, distribution.count());
            }

            #[test]
            fn distribution_rescale() {
                let mut distribution = Distribution::new(10, 10);

                for i in 0..100 {
                    distribution.observe(i as f64);
                }

                assert!(distribution.buckets().len() <= distribution.max_buckets());
                assert!(distribution.scale() < distribution.max_scale());

                distribution.reset();

                assert_eq!(distribution.max_scale(), distribution.scale());
            }
        }
    }

    #[cfg(feature = "alloc")]
    pub use self::alloc_support::*;

    #[cfg(test)]
    mod tests {
        use core::f64::consts::PI;

        use super::*;

        #[test]
        fn point_cmp() {
            let mut values = vec![
                Point::new(1.0),
                Point::new(f64::NAN),
                Point::new(0.0),
                Point::new(f64::NEG_INFINITY),
                Point::new(-1.0),
                Point::new(-0.0),
                Point::new(f64::INFINITY),
            ];

            values.sort();

            assert_eq!(
                vec![
                    Point::new(f64::NEG_INFINITY),
                    Point::new(-1.0),
                    Point::new(-0.0),
                    Point::new(0.0),
                    Point::new(1.0),
                    Point::new(f64::INFINITY),
                    Point::new(f64::NAN)
                ],
                &*values
            );
        }

        #[test]
        fn point_roundtrip() {
            let point = Point::new(1.0);

            assert_eq!(point, Point::try_from_str(&point.to_string()).unwrap());
        }

        #[test]
        fn point_is_indexable() {
            for (case, indexable) in [
                (Point::new(0.0), false),
                (Point::new(-0.0), false),
                (Point::new(f64::INFINITY), false),
                (Point::new(f64::NEG_INFINITY), false),
                (Point::new(f64::NAN), false),
                (Point::new(f64::EPSILON), true),
                (Point::new(-f64::EPSILON), true),
                (Point::new(f64::MIN), true),
                (Point::new(f64::MAX), true),
            ] {
                assert_eq!(indexable, case.is_indexable());
            }
        }

        #[test]
        fn point_is_bucket() {
            for (case, zero, neg, pos) in [
                (Point::new(0.0), true, false, false),
                (Point::new(-0.0), true, false, false),
                (Point::new(f64::INFINITY), false, false, false),
                (Point::new(f64::NEG_INFINITY), false, false, false),
                (Point::new(f64::NAN), false, false, false),
                (Point::new(f64::EPSILON), false, false, true),
                (Point::new(-f64::EPSILON), false, true, false),
                (Point::new(f64::MIN), false, true, false),
                (Point::new(f64::MAX), false, false, true),
            ] {
                assert_eq!(zero, case.is_zero_bucket());
                assert_eq!(neg, case.is_negative_bucket());
                assert_eq!(pos, case.is_positive_bucket());
            }
        }

        #[cfg(feature = "sval")]
        #[test]
        fn point_stream() {
            sval_test::assert_tokens(&Point::new(3.1), &[sval_test::Token::F64(3.1)]);
        }

        #[cfg(feature = "serde")]
        #[test]
        fn point_serialize() {
            serde_test::assert_ser_tokens(&Point::new(3.1), &[serde_test::Token::F64(3.1)]);
        }

        #[test]
        fn point_to_from_value() {
            let point = Point::new(3.1);

            assert_eq!(point, Point::from_value(point.to_value()).unwrap());
        }

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
                    let roundtrip = midpoint(actual.get(), scale);

                    if expected.is_nan() && actual.get().is_nan() && roundtrip.get().is_nan() {
                        continue;
                    }

                    assert_eq!(
                        expected.to_bits(),
                        actual.get().to_bits(),
                        "expected midpoint({case}, {scale}) to be {expected}, but got {actual}"
                    );

                    assert_eq!(
                        actual.get().to_bits(),
                        roundtrip.get().to_bits(),
                        "expected midpoint(midpoint({case}, {scale}), {scale}) to roundtrip to {actual}, but got {roundtrip}"
                    );
                }
            }
        }
    }
}

mod delta {
    use super::*;

    use core::mem;

    use crate::Timestamp;

    /**
    A container for tracking delta-encoded metrics.

    `emit` represents delta metrics as [`Event`]s where the [`Extent`] is a range.

    `Delta` tracks the time its value was last sampled along with the current value itself.
    The value can be accumulated into with [`Delta::current_value_mut`].

    At the end of a user-defined time period, the value can be sampled with [`Delta::advance`].
    When sampled, an [`Extent`] between the last sample and the current is returned along with an exclusive reference to the current value.
    Callers are expected to reset this value for the new time period before their borrow expires.

    `Delta` is not a [`Source`] directly, but can be used as the underlying storage in implementations of them.
    */
    pub struct Delta<T> {
        start: Option<Timestamp>,
        value: T,
    }

    impl<T> Delta<T> {
        /**
        Create a new delta container with an initial timestamp and value.
        */
        pub fn new(start: Option<Timestamp>, initial: T) -> Self {
            Delta {
                start,
                value: initial,
            }
        }

        /**
        Create a new delta container with an initial timestamp and default value.
        */
        pub fn new_default(start: Option<Timestamp>) -> Self
        where
            T: Default,
        {
            Self::new(start, Default::default())
        }

        /**
        Get a reference to the start of the current time period.
        */
        pub fn current_start(&self) -> Option<&Timestamp> {
            self.start.as_ref()
        }

        /**
        Get exclusive access to the value of the current time period.
        */
        pub fn current_value_mut(&mut self) -> &mut T {
            &mut self.value
        }

        /**
        Get shared access to the value of the current time period.
        */
        pub fn current_value(&self) -> &T {
            &self.value
        }

        /**
        Advance the delta to a new time period.

        This method will return a range [`Extent`] from [`Delta::current_start`] to `end` along with the current accumulated value.
        The next time period will start from `end`.

        Callers are responsible for resetting the current value for the new time period.
        */
        pub fn advance(&mut self, end: Option<Timestamp>) -> (Option<Extent>, &mut T) {
            let start = mem::replace(&mut self.start, end);

            let extent = (start..end).to_extent();

            (extent, &mut self.value)
        }

        /**
        Advance the delta to a new time period.

        This method is an alternative to [`Delta::advance`] that sets the value for the new time period with its default for you, returning the previously accumulated one.

        This method will return a range [`Extent`] from [`Delta::current_start`] to `end` along with the current accumulated value.
        The next time period will start from `end`.
        */
        pub fn advance_default(&mut self, end: Option<Timestamp>) -> (Option<Extent>, T)
        where
            T: Default,
        {
            let (extent, value) = self.advance(end);

            (extent, mem::take(value))
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        use core::time::Duration;

        #[test]
        fn delta_advance() {
            let mut delta = Delta::new(Some(Timestamp::MIN), 0);

            *delta.current_value_mut() += 1;

            let (extent, value) = delta.advance(Some(Timestamp::MIN + Duration::from_secs(1)));
            let extent = extent.unwrap();
            let range = extent.as_range().unwrap();

            assert_eq!(
                Timestamp::MIN..Timestamp::MIN + Duration::from_secs(1),
                *range
            );
            assert_eq!(1, *value);
        }
    }
}

pub use self::delta::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    use crate::{Timestamp, Value};

    #[test]
    fn metric_new() {
        let metric = Metric::new(
            Path::new_raw("test"),
            Timestamp::from_unix(Duration::from_secs(1)),
            [
                ("metric_prop", Value::from(true)),
                ("metric_name", Value::from("my metric")),
                ("metric_value", Value::from(42)),
                ("metric_description", Value::from("my description")),
                ("metric_agg", Value::from("count")),
            ],
        );

        assert_eq!("test", metric.mdl());
        assert_eq!(
            Timestamp::from_unix(Duration::from_secs(1)).unwrap(),
            metric.extent().unwrap().as_point()
        );
        assert_eq!("my metric", metric.name().unwrap());
        assert_eq!("my description", metric.description().unwrap());
        assert_eq!("count", metric.agg().unwrap());
        assert_eq!(42, metric.value().to_value().cast::<i32>().unwrap());
        assert_eq!(true, metric.props().pull::<bool, _>("metric_prop").unwrap());

        let metric = metric
            .with_name("my metric 2")
            .with_description("my description 2")
            .with_agg("last")
            .with_value(17)
            .with_unit("ms")
            .with_dist_min(0)
            .with_dist_max(100)
            .with_dist_sum(1000)
            .with_dist_count(42);

        assert_eq!("my metric 2", metric.name().unwrap());
        assert_eq!("my description 2", metric.description().unwrap());
        assert_eq!("last", metric.agg().unwrap());
        assert_eq!(17, metric.value().to_value().cast::<i32>().unwrap());
        assert_eq!("ms", metric.unit().unwrap());

        assert_eq!(0, metric.dist_min().to_value().cast::<i32>().unwrap());
        assert_eq!(100, metric.dist_max().to_value().cast::<i32>().unwrap());
        assert_eq!(1000, metric.dist_sum().to_value().cast::<i32>().unwrap());
        assert_eq!(42, metric.dist_count().to_value().cast::<i32>().unwrap());

        #[cfg(feature = "alloc")]
        {
            let set = exp::BucketSet::from_iter([
                (exp::Point::new(0.0), 3),
                (exp::Point::new(0.0), 2),
                (exp::Point::new(1.0), 2),
            ]);

            let metric = metric
                .with_dist_exp_scale(-1)
                .with_dist_exp_buckets(set.clone());

            assert_eq!(-1, metric.dist_exp_scale().unwrap());
            assert_eq!(set, metric.dist_exp_buckets().unwrap());
        }
    }

    #[test]
    fn metric_to_event() {
        let metric = Metric::new(
            Path::new_raw("test"),
            Timestamp::from_unix(Duration::from_secs(1)),
            [
                ("metric_prop", Value::from(true)),
                ("metric_name", Value::from("my metric")),
                ("metric_agg", Value::from("count")),
                ("metric_value", Value::from(42)),
            ],
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
                Timestamp::from_unix(Duration::from_secs(1)),
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
            let metric = Metric::new(Path::new_raw("test"), case, ("metric_prop", true));

            let extent = metric.to_extent();

            assert_eq!(
                expected.map(|extent| extent.as_range().cloned()),
                extent.map(|extent| extent.as_range().cloned())
            );
        }
    }
}
