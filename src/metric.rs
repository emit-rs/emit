/*!
The [`Metric`] type.
*/

use core::ops::ControlFlow;

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
            name: self.name,
            agg: self.agg,
            value: self.value,
            props,
        }
    }
}

impl<'a, P: Props> ToEvent for Metric<'a, P> {
    type Props<'b>
        = &'b Self
    where
        Self: 'b;

    fn to_event<'b>(&'b self) -> Event<'b, Self::Props<'b>> {
        // "{metric_agg} of {metric_name} is {metric_value}"
        const TEMPLATE: &'static [template::Part<'static>] = &[
            template::Part::hole("metric_agg"),
            template::Part::text(" of "),
            template::Part::hole("metric_name"),
            template::Part::text(" is "),
            template::Part::hole("metric_value"),
        ];

        Event::new(
            self.mdl.by_ref(),
            Template::new(TEMPLATE),
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
        Produce a current sample for all metrics in the source, emitting them as diagnostic events to the given [`Emitter`].
        */
        fn emit_metrics<E: Emitter>(&self, emitter: E) {
            self.sample_metrics(from_emitter(emitter))
        }

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

        fn emit_metrics<E: Emitter>(&self, emitter: E) {
            (**self).emit_metrics(emitter)
        }
    }

    impl<T: Source> Source for Option<T> {
        fn sample_metrics<S: sampler::Sampler>(&self, sampler: S) {
            if let Some(source) = self {
                source.sample_metrics(sampler);
            }
        }

        fn emit_metrics<E: Emitter>(&self, emitter: E) {
            if let Some(source) = self {
                source.emit_metrics(emitter);
            }
        }
    }

    #[cfg(feature = "alloc")]
    impl<'a, T: Source + ?Sized + 'a> Source for alloc::boxed::Box<T> {
        fn sample_metrics<S: sampler::Sampler>(&self, sampler: S) {
            (**self).sample_metrics(sampler)
        }

        fn emit_metrics<E: Emitter>(&self, emitter: E) {
            (**self).emit_metrics(emitter)
        }
    }

    #[cfg(feature = "alloc")]
    impl<'a, T: Source + ?Sized + 'a> Source for alloc::sync::Arc<T> {
        fn sample_metrics<S: sampler::Sampler>(&self, sampler: S) {
            (**self).sample_metrics(sampler)
        }

        fn emit_metrics<E: Emitter>(&self, emitter: E) {
            (**self).emit_metrics(emitter)
        }
    }

    impl<T: Source, U: Source> Source for And<T, U> {
        fn sample_metrics<S: sampler::Sampler>(&self, sampler: S) {
            self.left().sample_metrics(&sampler);
            self.right().sample_metrics(&sampler);
        }

        fn emit_metrics<E: Emitter>(&self, emitter: E) {
            self.left().emit_metrics(&emitter);
            self.right().emit_metrics(&emitter);
        }
    }

    impl<T: Source, U: Source> Source for Or<T, U> {
        fn sample_metrics<S: sampler::Sampler>(&self, sampler: S) {
            self.left().sample_metrics(&sampler);
            self.right().sample_metrics(&sampler);
        }

        fn emit_metrics<E: Emitter>(&self, emitter: E) {
            self.left().emit_metrics(&emitter);
            self.right().emit_metrics(&emitter);
        }
    }

    pub(in crate::metric) struct FromEmitter<E>(E);

    impl<E: Emitter> sampler::Sampler for FromEmitter<E> {
        fn metric<P: Props>(&self, metric: Metric<P>) {
            self.0.emit(metric)
        }
    }

    pub(in crate::metric) const fn from_emitter<E: Emitter>(emitter: E) -> FromEmitter<E> {
        FromEmitter(emitter)
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
        use emit_core::emitter;

        use super::*;

        pub trait DispatchSource {
            fn dispatch_sample_metrics(&self, sampler: &dyn sampler::ErasedSampler);

            fn dispatch_emit_metrics(&self, emitter: &dyn emitter::ErasedEmitter);
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

        fn dispatch_emit_metrics(&self, emitter: &dyn emit_core::emitter::ErasedEmitter) {
            self.emit_metrics(emitter)
        }
    }

    impl<'a> Source for dyn ErasedSource + 'a {
        fn sample_metrics<S: sampler::Sampler>(&self, sampler: S) {
            self.erase_source().0.dispatch_sample_metrics(&sampler)
        }

        fn emit_metrics<E: Emitter>(&self, emitter: E) {
            self.erase_source().0.dispatch_emit_metrics(&emitter)
        }
    }

    impl<'a> Source for dyn ErasedSource + Send + Sync + 'a {
        fn sample_metrics<S: sampler::Sampler>(&self, sampler: S) {
            (self as &(dyn ErasedSource + 'a)).sample_metrics(sampler)
        }

        fn emit_metrics<E: Emitter>(&self, emitter: E) {
            (self as &(dyn ErasedSource + 'a)).emit_metrics(emitter)
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
                        Path::new_unchecked("test"),
                        "metric 1",
                        "count",
                        crate::Empty,
                        42,
                        crate::Empty,
                    ));

                    sampler.metric(Metric::new(
                        Path::new_unchecked("test"),
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

            let calls = Cell::new(0);

            MySource.emit_metrics(crate::emitter::from_fn(|_| {
                calls.set(calls.get() + 1);
            }));

            assert_eq!(2, calls.get());
        }

        #[test]
        fn and_sample() {
            let calls = Cell::new(0);

            from_fn(|sampler| {
                sampler.metric(Metric::new(
                    Path::new_unchecked("test"),
                    "metric 1",
                    "count",
                    crate::Empty,
                    42,
                    crate::Empty,
                ));
            })
            .and_sample(from_fn(|sampler| {
                sampler.metric(Metric::new(
                    Path::new_unchecked("test"),
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
                    Path::new_unchecked("test"),
                    "metric 1",
                    "count",
                    crate::Empty,
                    42,
                    crate::Empty,
                ));

                sampler.metric(Metric::new(
                    Path::new_unchecked("test"),
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
                    Path::new_unchecked("test"),
                    "metric 1",
                    "count",
                    crate::Empty,
                    42,
                    crate::Empty,
                ));

                sampler.metric(Metric::new(
                    Path::new_unchecked("test"),
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
    }
}

#[cfg(feature = "alloc")]
mod alloc_support {
    use super::*;

    use alloc::{boxed::Box, vec::Vec};
    use core::ops::Range;

    use crate::{
        clock::{Clock, ErasedClock},
        metric::source::{self, ErasedSource, Source},
    };

    /**
    A set of [`Source`]s that are all sampled together.

    The reporter can be sampled like any other source through its own [`Source`] implementation.

    # Normalization

    The reporter will attempt to normalize the extents of any metrics sampled from its sources. When the `std` Cargo feature is enabled this will be done automatically.
    In other cases, normalization won't happen unless it's configured by [`Reporter::normalize_with_clock`].
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
                clock: {
                    #[cfg(feature = "std")]
                    {
                        ReporterClock::System
                    }
                    #[cfg(not(feature = "std"))]
                    {
                        ReporterClock::Other(None)
                    }
                },
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
            self.sample_metrics(source::from_emitter(emitter))
        }
    }

    impl Source for Reporter {
        fn sample_metrics<S: sampler::Sampler>(&self, sampler: S) {
            self.sample_metrics(sampler)
        }

        fn emit_metrics<E: Emitter>(&self, emitter: E) {
            self.emit_metrics(emitter)
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
        #[cfg(feature = "std")]
        System,
        Other(Option<Box<dyn ErasedClock + Send + Sync>>),
    }

    impl Clock for ReporterClock {
        fn now(&self) -> Option<Timestamp> {
            match self {
                #[cfg(feature = "std")]
                ReporterClock::System => crate::platform::system_clock::SystemClock::new().now(),
                ReporterClock::Other(clock) => clock.now(),
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::cell::Cell;

        #[test]
        fn reporter_is_send_sync() {
            fn check<T: Send + Sync>() {}

            check::<Reporter>();
        }

        #[test]
        fn reporter_sample() {
            let mut reporter = Reporter::new();

            reporter
                .add_source(source::from_fn(|sampler| {
                    sampler.metric(Metric::new(
                        Path::new_unchecked("test"),
                        "metric 1",
                        "count",
                        crate::Empty,
                        42,
                        crate::Empty,
                    ));
                }))
                .add_source(source::from_fn(|sampler| {
                    sampler.metric(Metric::new(
                        Path::new_unchecked("test"),
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

        #[test]
        fn reporter_normalize_empty_extent() {
            todo!()
        }

        #[test]
        fn reporter_normalize_point_extent() {
            todo!()
        }

        #[test]
        fn reporter_normalize_range_extent() {
            todo!()
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
    A [`Sampler`] from a function.

    This type can be created directly, or via [`from_fn`].
    */
    pub struct FromFn<F = fn(&Metric<&dyn ErasedProps>)>(F);

    /**
    Create a [`Sampler`] from a function.
    */
    pub const fn from_fn<F: Fn(&Metric<&dyn ErasedProps>)>(f: F) -> FromFn<F> {
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

    impl<F: Fn(&Metric<&dyn ErasedProps>)> Sampler for FromFn<F> {
        fn metric<P: Props>(&self, metric: Metric<P>) {
            (self.0)(&metric.erase())
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
                Path::new_unchecked("test"),
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
                Path::new_unchecked("test"),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    use crate::Timestamp;

    #[test]
    fn metric_new() {
        let metric = Metric::new(
            Path::new_unchecked("test"),
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
            Path::new_unchecked("test"),
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
                Path::new_unchecked("test"),
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
