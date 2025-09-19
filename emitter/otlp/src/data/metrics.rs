mod export_metrics_service;
mod metric;

use std::{cmp, collections::BTreeMap, ops::ControlFlow};

use crate::Error;

pub use self::{export_metrics_service::*, metric::*};

use emit::{Filter as _, Props as _};

use sval::{Stream, Value};

use super::{
    any_value, stream_encoded_scope_items, EncodedEvent, EncodedPayload, EncodedScopeItems,
    EventEncoder, InstrumentationScope, KeyValue, MessageFormatter, MessageRenderer, RawEncoder,
    RequestEncoder,
};

pub(crate) struct MetricsEventEncoder {
    pub name: Box<MessageFormatter>,
}

impl Default for MetricsEventEncoder {
    fn default() -> Self {
        Self {
            name: default_name_formatter(),
        }
    }
}

fn default_name_formatter() -> Box<MessageFormatter> {
    Box::new(|evt, f| {
        if let Some(name) = evt.props().get(emit::well_known::KEY_METRIC_NAME) {
            write!(f, "{}", name)
        } else {
            write!(f, "{}", evt.msg())
        }
    })
}

impl EventEncoder for MetricsEventEncoder {
    fn encode_event<E: RawEncoder>(
        &self,
        evt: &emit::event::Event<impl emit::props::Props>,
    ) -> Option<EncodedEvent> {
        if !emit::kind::is_metric_filter().matches(evt) {
            return None;
        }

        if let (Some(metric_value), metric_agg) = (
            evt.props().get(emit::well_known::KEY_METRIC_VALUE),
            evt.props().get(emit::well_known::KEY_METRIC_AGG),
        ) {
            let (start_time_unix_nano, time_unix_nano, aggregation_temporality) = evt
                .extent()
                .map(|extent| {
                    let (start, end, temporality) = if let Some(range) = extent.as_range() {
                        (range.start, range.end, AggregationTemporality::Delta)
                    } else {
                        (
                            *extent.as_point(),
                            *extent.as_point(),
                            AggregationTemporality::Cumulative,
                        )
                    };

                    (
                        start.to_unix().as_nanos() as u64,
                        end.to_unix().as_nanos() as u64,
                        temporality,
                    )
                })
                .unwrap_or((0, 0, AggregationTemporality::Unspecified));

            let metric_name = MessageRenderer {
                fmt: &self.name,
                evt,
            };

            let mut metric_unit = None;
            let mut dist_exp_scale = None;
            let mut dist_exp_buckets = None;
            let mut attributes = Vec::new();

            let _ = evt.props().for_each(|k, v| match k.get() {
                // Well-known fields
                emit::well_known::KEY_METRIC_UNIT => {
                    metric_unit = Some(v);

                    ControlFlow::Continue(())
                }
                emit::well_known::KEY_DIST_EXP_SCALE => {
                    dist_exp_scale = Some(v);

                    ControlFlow::Continue(())
                }
                emit::well_known::KEY_DIST_EXP_BUCKETS => {
                    dist_exp_buckets = Some(v);

                    ControlFlow::Continue(())
                }
                // Ignored
                emit::well_known::KEY_METRIC_NAME
                | emit::well_known::KEY_METRIC_VALUE
                | emit::well_known::KEY_METRIC_AGG
                | emit::well_known::KEY_SPAN_ID
                | emit::well_known::KEY_SPAN_PARENT
                | emit::well_known::KEY_TRACE_ID
                | emit::well_known::KEY_EVT_KIND
                | emit::well_known::KEY_SPAN_NAME
                | emit::well_known::KEY_SPAN_KIND => ControlFlow::Continue(()),
                // Regular attributes
                _ => {
                    if let Ok(value) = sval_buffer::stream_to_value_owned(any_value::EmitValue(v)) {
                        attributes.push(KeyValue {
                            key: k.to_owned(),
                            value,
                        });
                    }

                    ControlFlow::Continue(())
                }
            });

            let encoded = match metric_agg.and_then(|kind| kind.to_cow_str()).as_deref() {
                Some(emit::well_known::METRIC_AGG_SUM) => E::encode(Metric::<_, _, _> {
                    name: &sval::Display::new(metric_name),
                    unit: &metric_unit.map(sval::Display::new),
                    data: &MetricData::Sum::<_>(Sum::<_> {
                        aggregation_temporality,
                        is_monotonic: false,
                        data_points: &[NumberDataPoint {
                            attributes: &attributes,
                            start_time_unix_nano,
                            time_unix_nano,
                            value: NumberDataPointValue::from_value(metric_value)?,
                        }],
                    }),
                }),
                Some(emit::well_known::METRIC_AGG_COUNT) => {
                    if let Some(distribution) =
                        Distribution::from_values(dist_exp_scale, dist_exp_buckets)
                    {
                        E::encode(Metric::<_, _, _> {
                            name: &sval::Display::new(metric_name),
                            unit: &metric_unit.map(sval::Display::new),
                            data: &MetricData::ExponentialHistogram::<_>(
                                ExponentialHistogram::<_> {
                                    aggregation_temporality,
                                    data_points: &[ExponentialHistogramDataPoint {
                                        attributes: &attributes,
                                        start_time_unix_nano,
                                        time_unix_nano,
                                        count: distribution.count,
                                        scale: distribution.scale,
                                        zero_count: distribution.zero,
                                        positive: distribution.positive.as_ref().map(|buckets| {
                                            Buckets {
                                                offset: buckets.offset,
                                                bucket_counts: buckets,
                                            }
                                        }),
                                        negative: distribution.negative.as_ref().map(|buckets| {
                                            Buckets {
                                                offset: buckets.offset,
                                                bucket_counts: buckets,
                                            }
                                        }),
                                    }],
                                },
                            ),
                        })
                    } else {
                        E::encode(Metric::<_, _, _> {
                            name: &sval::Display::new(metric_name),
                            unit: &metric_unit.map(sval::Display::new),
                            data: &MetricData::Sum::<_>(Sum::<_> {
                                aggregation_temporality,
                                is_monotonic: true,
                                data_points: &[NumberDataPoint {
                                    attributes: &attributes,
                                    start_time_unix_nano,
                                    time_unix_nano,
                                    value: NumberDataPointValue::from_value(metric_value)?,
                                }],
                            }),
                        })
                    }
                }
                _ => E::encode(Metric::<_, _, _> {
                    name: &sval::Display::new(metric_name),
                    unit: &metric_unit.map(sval::Display::new),
                    data: &MetricData::Gauge(Gauge::<_> {
                        data_points: &[NumberDataPoint {
                            attributes: &attributes,
                            start_time_unix_nano,
                            time_unix_nano,
                            value: NumberDataPointValue::from_value(metric_value)?,
                        }],
                    }),
                }),
            };

            return Some(EncodedEvent {
                scope: evt.mdl().to_owned(),
                payload: encoded,
            });
        }

        None
    }
}

impl NumberDataPointValue {
    fn from_value(value: emit::Value) -> Option<Self> {
        struct Extract(Option<NumberDataPointValue>);

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
                self.0 = Some(NumberDataPointValue::AsInt(AsInt(value)));

                Ok(())
            }

            fn f64(&mut self, value: f64) -> sval::Result {
                self.0 = Some(NumberDataPointValue::AsDouble(AsDouble(value)));

                Ok(())
            }

            fn seq_begin(&mut self, _: Option<usize>) -> sval::Result {
                sval::error()
            }

            fn seq_value_begin(&mut self) -> sval::Result {
                sval::error()
            }

            fn seq_value_end(&mut self) -> sval::Result {
                sval::error()
            }

            fn seq_end(&mut self) -> sval::Result {
                sval::error()
            }
        }

        let mut extract = Extract(None);
        value.stream(&mut extract).ok()?;

        extract.0
    }
}

struct Distribution {
    positive: Option<DistributionBuckets>,
    negative: Option<DistributionBuckets>,
    zero: u64,
    count: u64,
    scale: i32,
}

struct DistributionBuckets {
    gamma: f64,
    offset: i32,
    buckets: BTreeMap<Midpoint, u64>,
}

#[derive(Clone, Copy)]
struct Midpoint(f64);

impl PartialEq for Midpoint {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == cmp::Ordering::Equal
    }
}

impl Eq for Midpoint {}

impl PartialOrd for Midpoint {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Midpoint {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}

impl Distribution {
    fn from_values(
        dist_exp_scale: Option<emit::Value>,
        dist_exp_buckets: Option<emit::Value>,
    ) -> Option<Self> {
        struct Extract {
            depth: usize,
            positive: BTreeMap<Midpoint, u64>,
            negative: BTreeMap<Midpoint, u64>,
            zero: u64,
            count: u64,
            next_midpoint: Option<f64>,
            next_count: Option<u64>,
        }

        impl Extract {
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

            fn apply(&mut self) -> sval::Result {
                if self.depth == 2 {
                    let midpoint = self
                        .next_midpoint
                        .take()
                        .ok_or_else(|| sval::Error::new())?;
                    let count = self.next_count.take().ok_or_else(|| sval::Error::new())?;

                    if !midpoint.is_finite() {
                        return sval::error();
                    }

                    let buckets = if midpoint.is_sign_positive() {
                        &mut self.positive
                    } else {
                        &mut self.negative
                    };

                    let midpoint = midpoint.abs();

                    if midpoint == 0.0 {
                        self.zero += count;
                    } else {
                        *buckets.entry(Midpoint(midpoint)).or_insert(0) += count;
                    }

                    self.count += count;

                    Ok(())
                } else {
                    Ok(())
                }
            }
        }

        impl<'sval> Stream<'sval> for Extract {
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
                self.push(|| Some(value as f64), || Some(value))
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
                self.apply()?;
                self.depth -= 1;

                Ok(())
            }
        }

        if let (Some(dist_exp_scale), Some(dist_exp_buckets)) = (
            dist_exp_scale.and_then(|v| v.cast::<i32>()),
            dist_exp_buckets,
        ) {
            let scale = dist_exp_scale;
            let gamma = 2.0f64.powf(2.0f64.powi(-scale));

            let mut extract = Extract {
                depth: 0,
                positive: BTreeMap::new(),
                negative: BTreeMap::new(),
                zero: 0,
                count: 0,
                next_midpoint: None,
                next_count: None,
            };

            sval::stream(&mut extract, &dist_exp_buckets).ok()?;

            let distribution_buckets_from_map = |buckets: BTreeMap<Midpoint, u64>| {
                let Some((min, _)) = buckets.first_key_value() else {
                    return None;
                };

                let offset = bucket(min.0, gamma).try_into().ok()?;

                Some((offset, buckets))
            };

            let positive = distribution_buckets_from_map(extract.positive);
            let negative = distribution_buckets_from_map(extract.negative);

            return Some(Distribution {
                positive: positive.map(|(offset, buckets)| DistributionBuckets {
                    gamma,
                    offset,
                    buckets,
                }),
                negative: negative.map(|(offset, buckets)| DistributionBuckets {
                    gamma,
                    offset,
                    buckets,
                }),
                zero: extract.zero,
                count: extract.count,
                scale,
            });
        }

        None
    }
}

impl Value for DistributionBuckets {
    fn stream<'sval, S: Stream<'sval> + ?Sized>(&'sval self, stream: &mut S) -> sval::Result {
        stream.seq_begin(None)?;
        let mut last_index = self.offset as isize;

        for (midpoint, count) in &self.buckets {
            let index = bucket(midpoint.0, self.gamma);

            for _ in (last_index + 1)..index {
                stream.seq_value_begin()?;
                stream.u64(0)?;
                stream.seq_value_end()?;
            }

            stream.seq_value_begin()?;
            stream.u64(*count)?;
            stream.seq_value_end()?;

            last_index = index;
        }

        stream.seq_end()
    }
}

fn bucket(value: f64, gamma: f64) -> isize {
    value.log(gamma).ceil() as isize
}

#[derive(Default)]
pub(crate) struct MetricsRequestEncoder;

impl RequestEncoder for MetricsRequestEncoder {
    fn encode_request<E: RawEncoder>(
        &self,
        resource: Option<&EncodedPayload>,
        items: &EncodedScopeItems,
    ) -> Result<EncodedPayload, Error> {
        Ok(E::encode(ExportMetricsServiceRequest {
            resource_metrics: &[ResourceMetrics {
                resource: &resource,
                scope_metrics: &EncodedScopeMetrics(items),
            }],
        }))
    }
}

struct EncodedScopeMetrics<'a>(&'a EncodedScopeItems);

impl<'a> sval::Value for EncodedScopeMetrics<'a> {
    fn stream<'sval, S: sval::Stream<'sval> + ?Sized>(&'sval self, stream: &mut S) -> sval::Result {
        stream_encoded_scope_items(stream, &self.0, |stream, path, metrics| {
            stream.value_computed(&ScopeMetrics {
                scope: &InstrumentationScope {
                    name: &sval::Display::new(path),
                },
                metrics,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use prost::Message;

    use crate::data::{generated::metrics::v1 as metrics, util::*};

    fn double_point(v: impl Into<f64>) -> metrics::number_data_point::Value {
        metrics::number_data_point::Value::AsDouble(v.into())
    }

    fn int_point(v: impl Into<i64>) -> metrics::number_data_point::Value {
        metrics::number_data_point::Value::AsInt(v.into())
    }

    #[test]
    fn encode_count_int() {
        encode_event::<MetricsEventEncoder>(
            emit::evt!(
                "{metric_agg} of {metric_name} is {metric_value}",
                evt_kind: "metric",
                metric_name: "test",
                metric_agg: "count",
                metric_value: 43,
            ),
            |buf| {
                let de = metrics::Metric::decode(buf).unwrap();

                assert_eq!("test", de.name);

                match de.data {
                    Some(metrics::metric::Data::Sum(sum)) => {
                        assert!(sum.is_monotonic);
                        assert_eq!(
                            AggregationTemporality::Unspecified as i32,
                            sum.aggregation_temporality
                        );

                        assert_eq!(Some(int_point(43)), sum.data_points[0].value);
                    }
                    other => panic!("unexpected {other:?}"),
                }
            },
        );
    }

    #[test]
    fn encode_count_double() {
        encode_event::<MetricsEventEncoder>(
            emit::evt!(
                "{metric_agg} of {metric_name} is {metric_value}",
                evt_kind: "metric",
                metric_name: "test",
                metric_agg: "count",
                metric_value: 43.1,
            ),
            |buf| {
                let de = metrics::Metric::decode(buf).unwrap();

                assert_eq!("test", de.name);

                match de.data {
                    Some(metrics::metric::Data::Sum(sum)) => {
                        assert!(sum.is_monotonic);

                        assert_eq!(Some(double_point(43.1)), sum.data_points[0].value);
                    }
                    other => panic!("unexpected {other:?}"),
                }
            },
        );
    }

    #[test]
    fn encode_count_cumulative() {
        encode_event::<MetricsEventEncoder>(
            emit::evt!(
                extent: emit::Timestamp::MIN,
                "{metric_agg} of {metric_name} is {metric_value}",
                evt_kind: "metric",
                metric_name: "test",
                metric_agg: "count",
                metric_value: 43,
            ),
            |buf| {
                let de = metrics::Metric::decode(buf).unwrap();

                match de.data {
                    Some(metrics::metric::Data::Sum(sum)) => {
                        assert_eq!(
                            AggregationTemporality::Cumulative as i32,
                            sum.aggregation_temporality
                        );
                    }
                    other => panic!("unexpected {other:?}"),
                }
            },
        );
    }

    #[test]
    fn encode_count_delta() {
        encode_event::<MetricsEventEncoder>(
            emit::evt!(
                extent: emit::Timestamp::MIN..emit::Timestamp::MIN,
                "{metric_agg} of {metric_name} is {metric_value}",
                evt_kind: "metric",
                metric_name: "test",
                metric_agg: "count",
                metric_value: 43,
            ),
            |buf| {
                let de = metrics::Metric::decode(buf).unwrap();

                match de.data {
                    Some(metrics::metric::Data::Sum(sum)) => {
                        assert_eq!(
                            AggregationTemporality::Delta as i32,
                            sum.aggregation_temporality
                        );
                    }
                    other => panic!("unexpected {other:?}"),
                }
            },
        );
    }

    #[test]
    fn encode_sum_int() {
        encode_event::<MetricsEventEncoder>(
            emit::evt!(
                "{metric_agg} of {metric_name} is {metric_value}",
                evt_kind: "metric",
                metric_name: "test",
                metric_agg: "sum",
                metric_value: 43,
            ),
            |buf| {
                let de = metrics::Metric::decode(buf).unwrap();

                assert_eq!("test", de.name);

                match de.data {
                    Some(metrics::metric::Data::Sum(sum)) => {
                        assert!(!sum.is_monotonic);

                        assert_eq!(Some(int_point(43)), sum.data_points[0].value);
                    }
                    other => panic!("unexpected {other:?}"),
                }
            },
        );
    }

    #[test]
    fn encode_sum_double() {
        encode_event::<MetricsEventEncoder>(
            emit::evt!(
                "{metric_agg} of {metric_name} is {metric_value}",
                evt_kind: "metric",
                metric_name: "test",
                metric_agg: "sum",
                metric_value: 43.1,
            ),
            |buf| {
                let de = metrics::Metric::decode(buf).unwrap();

                assert_eq!("test", de.name);

                match de.data {
                    Some(metrics::metric::Data::Sum(sum)) => {
                        assert!(!sum.is_monotonic);

                        assert_eq!(Some(double_point(43.1)), sum.data_points[0].value);
                    }
                    other => panic!("unexpected {other:?}"),
                }
            },
        );
    }

    #[test]
    fn encode_gauge_int() {
        encode_event::<MetricsEventEncoder>(
            emit::evt!(
                "{metric_agg} of {metric_name} is {metric_value}",
                evt_kind: "metric",
                metric_name: "test",
                metric_agg: "last",
                metric_value: 43,
            ),
            |buf| {
                let de = metrics::Metric::decode(buf).unwrap();

                assert_eq!("test", de.name);

                match de.data {
                    Some(metrics::metric::Data::Gauge(gauge)) => {
                        assert_eq!(Some(int_point(43)), gauge.data_points[0].value);
                    }
                    other => panic!("unexpected {other:?}"),
                }
            },
        );
    }

    #[test]
    fn encode_gauge_double() {
        encode_event::<MetricsEventEncoder>(
            emit::evt!(
                "{metric_agg} of {metric_name} is {metric_value}",
                evt_kind: "metric",
                metric_name: "test",
                metric_agg: "last",
                metric_value: 43.1,
            ),
            |buf| {
                let de = metrics::Metric::decode(buf).unwrap();

                assert_eq!("test", de.name);

                match de.data {
                    Some(metrics::metric::Data::Gauge(gauge)) => {
                        assert_eq!(Some(double_point(43.1)), gauge.data_points[0].value);
                    }
                    other => panic!("unexpected {other:?}"),
                }
            },
        );
    }

    fn midpoint_from_index(index: i32, scale: i32) -> f64 {
        let gamma = 2.0f64.powf(2.0f64.powi(-scale));
        let lower = gamma.powi(index - 1);
        let upper = lower * gamma;

        lower.midpoint(upper)
    }

    #[test]
    fn encode_histogram_seq() {
        encode_event::<MetricsEventEncoder>(
            emit::evt!(
                "{metric_agg} of {metric_name} is {metric_value}",
                evt_kind: "metric",
                metric_name: "test",
                metric_agg: "count",
                metric_value: 100,
                #[emit::as_sval]
                dist_exp_buckets: [
                    (0.0, 3),
                    (midpoint_from_index(-1, 2), 4),
                    (midpoint_from_index(3, 2), 5),
                    (midpoint_from_index(4, 2), 6),
                    (midpoint_from_index(10, 2), 1),
                    (-midpoint_from_index(1, 2), 1),
                    (-midpoint_from_index(3, 2), 2),
                ],
                dist_exp_scale: 2,
            ),
            |buf| {
                let de = metrics::Metric::decode(buf).unwrap();

                assert_eq!("test", de.name);

                match de.data {
                    Some(metrics::metric::Data::ExponentialHistogram(histogram)) => {
                        assert_eq!(1, histogram.data_points.len());

                        assert_eq!(2, histogram.data_points[0].scale);
                        assert_eq!(
                            -1,
                            histogram.data_points[0].positive.as_ref().unwrap().offset
                        );
                        assert_eq!(
                            &[4, 0, 0, 0, 5, 6, 0, 0, 0, 0, 0, 1],
                            &*histogram.data_points[0]
                                .positive
                                .as_ref()
                                .unwrap()
                                .bucket_counts
                        );

                        assert_eq!(
                            1,
                            histogram.data_points[0].negative.as_ref().unwrap().offset
                        );
                        assert_eq!(
                            &[1, 0, 2],
                            &*histogram.data_points[0]
                                .negative
                                .as_ref()
                                .unwrap()
                                .bucket_counts
                        );

                        assert_eq!(3, histogram.data_points[0].zero_count);

                        assert_eq!(22, histogram.data_points[0].count);
                    }
                    other => panic!("unexpected {other:?}"),
                }
            },
        );
    }
}
