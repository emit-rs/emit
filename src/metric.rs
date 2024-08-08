/*!
The [`Metric`] type.

Metrics are an effective approach to monitoring applications at scale. They can be cheap to collect, making them suitable for performance sensitive operations. They can also be compact to report, making them suitable for high-volume scenarios. `emit` doesn't provide much infrastructure for collecting or sampling metrics. What it does provide is a standard way to report metric samples as events.

A standard kind of metric is a monotonic counter, which can be represented as an atomic integer. In this example, our counter is for the number of bytes written to a file, which we'll call `bytes_written`. We can report a sample of this counter as an event by wrapping it in a [`Metric`]:

```
# #[cfg(not(feature = "std"))] fn main() {}
# #[cfg(feature = "std")] fn main() {
# fn sample_bytes_written() -> usize { 4643 }
use emit::{well_known::METRIC_AGG_COUNT, Clock};

let sample = sample_bytes_written();

emit::emit!(
    evt: emit::Metric::new(
        emit::module!(),
        emit::Empty,
        "bytes_written",
        METRIC_AGG_COUNT,
        sample,
        emit::Empty,
    )
);
# }
```

```text
Event {
    module: "my_app",
    tpl: "`metric_agg` of `metric_name` is `metric_value`",
    extent: Some(
        "2024-04-29T10:08:24.780230000Z",
    ),
    props: {
        "event_kind": metric,
        "metric_name": "bytes_written",
        "metric_agg": "count",
        "metric_value": 4643,
    },
}
```

Metrics may also be emitted manually:

```
# #[cfg(not(feature = "std"))] fn main() {}
# #[cfg(feature = "std")] fn main() {
# fn sample_bytes_written() -> usize { 4643 }
use emit::well_known::{EVENT_KIND_METRIC, METRIC_AGG_COUNT};

let sample = sample_bytes_written();

emit::emit!(
    "{metric_agg} of {metric_name} is {metric_value}",
    event_kind: EVENT_KIND_METRIC,
    metric_agg: METRIC_AGG_COUNT,
    metric_name: "bytes_written",
    metric_value: sample,
);
# }
```

# Data model

The data model of metrics is an extension of `emit`'s events. Metric events are points or buckets in a time-series. They don't model the underlying instruments collecting metrics like counters or gauges. They instead model the aggregation of readings from those instruments over their lifetime. Metric events include the following well-known properties:

- `event_kind`: with a value of `"metric"` to indicate that the event is a metric sample.
- `metric_agg`: the aggregation over the underlying data stream that produced the sample.
    - `"count"`: A monotonic sum of `1`'s for defined values, and `0`'s for undefined values.
    - `"sum"`: A potentially non-monotonic sum of defined values.
    - `"min"`: The lowest ordered value.
    - `"max"`: The largest ordered value.
    - `"last"`: The most recent value.
- `metric_name`: the name of the underlying data stream.
- `metric_value`: the sample itself. These values are expected to be numeric.

## Cumulative metrics

Metric events with a point extent are cumulative. Their `metric_value` is the result of applying their `metric_agg` over the entire underlying stream up to that point.

The following metric reports the current number of bytes written as 591:

```
# #[cfg(not(feature = "std"))] fn main() {}
# #[cfg(feature = "std")] fn main() {
use emit::{Clock, well_known::METRIC_AGG_COUNT};

emit::emit!(
    evt: emit::Metric::new(
        emit::module!(),
        emit::Empty,
        "bytes_written",
        METRIC_AGG_COUNT,
        591,
        emit::Empty,
    )
);
# }
```

```text
Event {
    module: "my_app",
    tpl: "`metric_agg` of `metric_name` is `metric_value`",
    extent: Some(
        "2024-04-30T06:53:41.069203000Z",
    ),
    props: {
        "event_kind": metric,
        "metric_name": "bytes_written",
        "metric_agg": "count",
        "metric_value": 591,
    },
}
```

## Delta metrics

Metric events with a span extent are deltas. Their `metric_value` is the result of applying their `metric_agg` over the underlying stream within the extent.

The following metric reports that the number of bytes written changed by 17 over the last 30 seconds:

```
# #[cfg(not(feature = "std"))] fn main() {}
# #[cfg(feature = "std")] fn main() {
use emit::{Clock, well_known::METRIC_AGG_COUNT};

let now = emit::clock().now();
let last_sample = now.map(|now| now - std::time::Duration::from_secs(30));

emit::emit!(
    evt: emit::Metric::new(
        emit::module!(),
        last_sample..now,
        "bytes_written",
        METRIC_AGG_COUNT,
        17,
        emit::Empty,
    )
);
# }
```

```text
Event {
    module: "my_app",
    tpl: "`metric_agg` of `metric_name` is `metric_value`",
    extent: Some(
        "2024-04-30T06:55:59.839770000Z".."2024-04-30T06:56:29.839770000Z",
    ),
    props: {
        "event_kind": metric,
        "metric_name": "bytes_written",
        "metric_agg": "count",
        "metric_value": 17,
    },
}
```

## Time-series metrics

Metric events with a span extent, where the `metric_value` is an array are a complete time-series. Each element in the array is a bucket in the time-series. The width of each bucket is the length of the extent divided by the number of buckets.

The following metric is for a time-series with 15 buckets, where each bucket covers 1 second:

```
# #[cfg(not(feature = "std"))] fn main() {}
# #[cfg(feature = "std")] fn main() {
use emit::{Clock, well_known::METRIC_AGG_COUNT};

let now = emit::clock().now();
let last_sample = now.map(|now| now - std::time::Duration::from_secs(15));

emit::emit!(
    evt: emit::Metric::new(
        emit::module!(),
        last_sample..now,
        "bytes_written",
        METRIC_AGG_COUNT,
        &[
            0,
            5,
            56,
            0,
            0,
            221,
            7,
            0,
            0,
            5,
            876,
            0,
            194,
            0,
            18,
        ],
        emit::Empty,
    )
);
# }
```

```text
Event {
    module: "my_app",
    tpl: "`metric_agg` of `metric_name` is `metric_value`",
    extent: Some(
        "2024-04-30T07:03:07.828185000Z".."2024-04-30T07:03:22.828185000Z",
    ),
    props: {
        "event_kind": metric,
        "metric_name": "bytes_written",
        "metric_agg": "count",
        "metric_value": [
            0,
            5,
            56,
            0,
            0,
            221,
            7,
            0,
            0,
            5,
            876,
            0,
            194,
            0,
            18,
        ],
    },
}
```

# Metric sources

The [`Source`] trait represents some underlying data source that can be sampled to provide [`Metric`]s. You can sample sources directly, or combine them into a [`Reporter`] to sample all the sources of metrics in your application together:

```
# #[cfg(not(feature = "std"))] fn main() {}
# #[cfg(feature = "std")] fn main() {
use emit::metric::{Source as _, Sampler as _};

// Create some metric sources
let source_1 = emit::metric::source::from_fn(|sampler| {
    sampler.metric(emit::Metric::new(
        emit::path!("source_1"),
        emit::Empty,
        "bytes_written",
        emit::well_known::METRIC_AGG_COUNT,
        1,
        emit::Empty,
    ));
});

let source_2 = emit::metric::source::from_fn(|sampler| {
    sampler.metric(emit::Metric::new(
        emit::path!("source_2"),
        emit::Empty,
        "bytes_written",
        emit::well_known::METRIC_AGG_COUNT,
        2,
        emit::Empty,
    ));
});

// Collect them into a reporter
let mut reporter = emit::metric::Reporter::new();

reporter.add_source(source_1);
reporter.add_source(source_2);

// You'll probably want to run this task in your async runtime
// so it observes cancellation etc, but works for this illustration.
std::thread::spawn(move || {
    loop {
        // You could also use `sample_metrics` here and tweak the extents of metrics
        // to ensure they're all aligned together
        reporter.emit_metrics(emit::runtime::shared());

        std::thread::sleep(std::time::Duration::from_secs(30));
    }
});
# }
```
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
    value::{ToValue, Value},
    well_known::{KEY_EVENT_KIND, KEY_METRIC_AGG, KEY_METRIC_NAME, KEY_METRIC_VALUE},
};

use crate::kind::Kind;

pub use self::{sampler::Sampler, source::Source};

/**
A diagnostic event that represents a metric sample.

Metrics are an extension of [`Event`]s that explicitly take the well-known properties that signal an event as being a metric sample. See the [`crate::metric`] module for details.

A `Metric` can be converted into an [`Event`] through its [`ToEvent`] implemenation, or passed directly to an [`Emitter`] to emit it.
*/
pub struct Metric<'a, P> {
    module: Path<'a>,
    extent: Option<Extent>,
    name: Str<'a>,
    agg: Str<'a>,
    value: Value<'a>,
    props: P,
}

impl<'a, P> Metric<'a, P> {
    /**
    Create a new metric from its properties.

    Each metric consists of:

    - `module`: The module that owns the underlying data source.
    - `extent`: The [`Extent`] that the sample covers.
    - `name`: The name of the underlying data source.
    - `agg`: The aggregation applied to the underlying data source to produce the sample. See the [`crate::metric`] module for details.
    - `value`: The value of the sample itself.
    - `props`: Additional [`Props`] to associate with the sample.
    */
    pub fn new(
        module: impl Into<Path<'a>>,
        extent: impl ToExtent,
        name: impl Into<Str<'a>>,
        agg: impl Into<Str<'a>>,
        value: impl Into<Value<'a>>,
        props: P,
    ) -> Self {
        Metric {
            module: module.into(),
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
    pub fn module(&self) -> &Path<'a> {
        &self.module
    }

    /**
    Set the module of the underlying data source to a new value.
    */
    pub fn with_module(mut self, module: impl Into<Path<'a>>) -> Self {
        self.module = module.into();
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
            module: self.module,
            extent: self.extent,
            name: self.name,
            agg: self.agg,
            value: self.value,
            props,
        }
    }
}

impl<'a, P: Props> ToEvent for Metric<'a, P> {
    type Props<'b> = &'b Self where Self: 'b;

    fn to_event<'b>(&'b self) -> Event<Self::Props<'b>> {
        // "{metric_agg} of {metric_name} is {metric_value}"
        const TEMPLATE: &'static [template::Part<'static>] = &[
            template::Part::hole("metric_agg"),
            template::Part::text(" of "),
            template::Part::hole("metric_name"),
            template::Part::text(" is "),
            template::Part::hole("metric_value"),
        ];

        Event::new(
            self.module.by_ref(),
            self.extent.clone(),
            Template::new(TEMPLATE),
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
            module: self.module.by_ref(),
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
            module: self.module.by_ref(),
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
        for_each(KEY_EVENT_KIND.to_str(), Kind::Metric.to_value())?;
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
            struct FromEmitter<E>(E);

            impl<E: Emitter> sampler::Sampler for FromEmitter<E> {
                fn metric<P: Props>(&self, metric: Metric<P>) {
                    self.0.emit(metric)
                }
            }

            self.sample_metrics(FromEmitter(emitter))
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

    /**
    A [`Source`] from a function.

    This type can be created directly, or via [`from_fn`].
    */
    pub struct FromFn<F = fn(&mut dyn ErasedSampler)>(F);

    /**
    Create a [`Source`] from a function.
    */
    pub fn from_fn<F: Fn(&mut dyn ErasedSampler)>(source: F) -> FromFn<F> {
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
                        crate::Empty,
                        "metric 1",
                        "count",
                        42,
                        crate::Empty,
                    ));

                    sampler.metric(Metric::new(
                        Path::new_unchecked("test"),
                        crate::Empty,
                        "metric 2",
                        "count",
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
                    crate::Empty,
                    "metric 1",
                    "count",
                    42,
                    crate::Empty,
                ));
            })
            .and_sample(from_fn(|sampler| {
                sampler.metric(Metric::new(
                    Path::new_unchecked("test"),
                    crate::Empty,
                    "metric 2",
                    "count",
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
                    crate::Empty,
                    "metric 1",
                    "count",
                    42,
                    crate::Empty,
                ));

                sampler.metric(Metric::new(
                    Path::new_unchecked("test"),
                    crate::Empty,
                    "metric 2",
                    "count",
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
                    crate::Empty,
                    "metric 1",
                    "count",
                    42,
                    crate::Empty,
                ));

                sampler.metric(Metric::new(
                    Path::new_unchecked("test"),
                    crate::Empty,
                    "metric 2",
                    "count",
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

    use self::source::{ErasedSource, Source};

    /**
    A set of [`Source`]s that are all sampled together.

    The reporter can be sampled like any other source through its own [`Source`] implementation.
    */
    pub struct Reporter(Vec<Box<dyn ErasedSource + Send + Sync>>);

    impl Reporter {
        /**
        Create a new empty reporter.
        */
        pub const fn new() -> Self {
            Reporter(Vec::new())
        }

        /**
        Add a [`Source`] to the reporter.
        */
        pub fn add_source(&mut self, source: impl Source + Send + Sync + 'static) -> &mut Self {
            self.0.push(Box::new(source));

            self
        }
    }

    impl Source for Reporter {
        fn sample_metrics<S: sampler::Sampler>(&self, sampler: S) {
            for source in &self.0 {
                source.sample_metrics(&sampler);
            }
        }

        fn emit_metrics<E: Emitter>(&self, emitter: E) {
            for source in &self.0 {
                source.emit_metrics(&emitter);
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
                        crate::Empty,
                        "metric 1",
                        "count",
                        42,
                        crate::Empty,
                    ));
                }))
                .add_source(source::from_fn(|sampler| {
                    sampler.metric(Metric::new(
                        Path::new_unchecked("test"),
                        crate::Empty,
                        "metric 2",
                        "count",
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
    pub fn from_fn<F: Fn(&Metric<&dyn ErasedProps>)>(f: F) -> FromFn<F> {
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
                Empty,
                "test",
                "count",
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
                Empty,
                "test",
                "count",
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
            Timestamp::from_unix(Duration::from_secs(1)),
            "my metric",
            "count",
            42,
            ("metric_prop", true),
        );

        assert_eq!("test", metric.module());
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
            Timestamp::from_unix(Duration::from_secs(1)),
            "my metric",
            "count",
            42,
            ("metric_prop", true),
        );

        let evt = metric.to_event();

        assert_eq!("test", evt.module());
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
            evt.props().pull::<Kind, _>(KEY_EVENT_KIND).unwrap()
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
                case,
                "my metric",
                "count",
                42,
                ("metric_prop", true),
            );

            let extent = metric.to_extent();

            assert_eq!(
                expected.map(|extent| extent.as_range().clone()),
                extent.map(|extent| extent.as_range().clone())
            );
        }
    }
}
