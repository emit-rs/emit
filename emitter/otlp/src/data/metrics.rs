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
            let mut dist_scale = None;
            let mut dist_buckets = None;
            let mut attributes = Vec::new();

            let _ = evt.props().for_each(|k, v| match k.get() {
                // Well-known fields
                emit::well_known::KEY_METRIC_UNIT => {
                    metric_unit = Some(v);

                    ControlFlow::Continue(())
                }
                emit::well_known::KEY_DIST_BUCKET_SCALE => {
                    dist_scale = Some(v);

                    ControlFlow::Continue(())
                }
                emit::well_known::KEY_DIST_BUCKET_POINTS => {
                    dist_buckets = Some(v);

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
                            value: number_data_point_from_value(metric_value)?,
                        }],
                    }),
                }),
                Some(emit::well_known::METRIC_AGG_COUNT) => {
                    if let Some(distribution) = Distribution::from_values(
                        dist_scale,
                        dist_buckets,
                    ) {
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
                                        count: distribution.total_count,
                                        scale: distribution.scale,
                                        zero_count: distribution.zero,
                                        positive: distribution.positive.as_ref().map(|buckets| Buckets {
                                            offset: bucket_index(buckets.min.0, buckets.scale),
                                            bucket_counts: buckets,
                                        }),
                                        negative: distribution.negative.as_ref().map(|buckets| Buckets {
                                            offset: bucket_index(buckets.min.0, buckets.scale),
                                            bucket_counts: buckets,
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
                                    value: number_data_point_from_value(metric_value)?,
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
                            value: number_data_point_from_value(metric_value)?,
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

fn number_data_point_from_value(value: emit::Value) -> Option<NumberDataPointValue> {
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
    let _ = value.stream(&mut extract);

    extract.0
}

struct Distribution {
    positive: Option<DistributionBuckets>,
    negative: Option<DistributionBuckets>,
    zero: u64,
    total_count: u64,
    scale: i32,
}

struct DistributionBuckets {
    scale: i32,
    min: Midpoint,
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
        dist_bucket_scale: Option<emit::Value>,
        dist_bucket_points: Option<emit::Value>,
    ) -> Option<Self> {
        if let (Some(dist_bucket_scale), Some(dist_bucket_points)) = (
            dist_bucket_scale.and_then(|v| v.cast::<i32>()),
            dist_bucket_points,
        ) {
            let target_buckets = 160;
            let mut scale = dist_bucket_scale;
            let mut positive = BTreeMap::new();
            let mut negative = BTreeMap::new();
            let mut zero = 0;
            let mut total_count = 0;
            let mut valid = true;

            if emit::metric::dist::visit_bucket_points(dist_bucket_points, |midpoint, count| {
                if !midpoint.is_finite() {
                    valid = false;

                    return ControlFlow::Break(());
                }

                let buckets = if midpoint.is_sign_positive() {
                    &mut positive
                } else {
                    &mut negative
                };

                let midpoint = midpoint.abs();

                if midpoint == 0.0 {
                    zero += count;
                } else {
                    *buckets.entry(Midpoint(midpoint)).or_insert(0) += count;
                }

                total_count += count;

                ControlFlow::Continue(())
            })
            .map_err(|_| ())
            .and_then(|_| if valid { Ok(()) } else { Err(()) })
            .is_ok()
            {
                let distribution_buckets_from_map = |buckets: BTreeMap<Midpoint, u64>, scale: &mut i32| {
                    let Some((min, _)) = buckets.first_key_value() else {
                        return None;
                    };

                    let Some((max, _)) = buckets.last_key_value() else {
                        return Some((*min, buckets));
                    };

                    *scale = rescale(min.0, max.0, *scale, target_buckets);

                    Some((*min, buckets))
                };
                
                let positive = distribution_buckets_from_map(positive, &mut scale);
                let negative = distribution_buckets_from_map(negative, &mut scale);

                return Some(Distribution {
                    positive: positive.map(|(min, buckets)| DistributionBuckets {
                        scale,
                        min,
                        buckets,
                    }),
                    negative: negative.map(|(min, buckets)| DistributionBuckets {
                        scale,
                        min,
                        buckets,
                    }),
                    zero,
                    total_count,
                    scale,
                });
            }
        }

        None
    }
}

impl Value for DistributionBuckets {
    fn stream<'sval, S: Stream<'sval> + ?Sized>(&'sval self, stream: &mut S) -> sval::Result {
        todo!()
    }
}

fn bucket_index(value: f64, scale: i32) -> i32 {
    let positive = value == 0.0 || value.is_sign_positive();
    let value = value.abs();

    let gamma = 2.0f64.powf(2.0f64.powi(-scale));
    let index = value.log(gamma).ceil();

    index as i32
}

fn rescale(min: f64, max: f64, scale: i32, target_buckets: usize) -> i32 {
    let gamma = 2.0f64.powf(2.0f64.powi(-scale));
    let min = min.abs().log(gamma).ceil();
    let max = max.abs().log(gamma).ceil();
    let size = (max + -min).abs();

    cmp::min(
        scale,
        scale - ((size / target_buckets as f64).log2().ceil()) as i32,
    )
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

    use crate::data::{generated::metrics::v1 as metrics, util::*, AnyValue};

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

    #[test]
    fn compute_bucket_indexes() {
        for (value, scale, expected) in [
            (50f64, 3, 46),
            (51f64, 3, 46),
            (-50f64, 3, 46),
            (-51f64, 3, 46),
        ] {
            let actual = bucket_index(value, scale);
            assert_eq!(
                expected, actual,
                "expected bucket_index({value}, {scale}) to be {expected:?} but got {actual:?}"
            );
        }
    }

    #[test]
    fn compute_rescale() {
        for (min, max, scale, target_buckets, expected) in [
            (1f64, 1000f64, 2, 160, 2),
            (0.1f64, 100.0f64, 3, 160, 3),
            (0.1f64, 100.0f64, 6, 160, 4),
            (0.000001f64, 100.0f64, 5, 160, 2),
            (-100.0f64, -0.1f64, 3, 160, 3),
            (-100.0f64, -0.1f64, 6, 160, 4),
            (-100.0f64, -0.000001f64, 5, 160, 2),
            (1f64, 1000000f64, -2, 3, -3),
        ] {
            let actual = rescale(min, max, scale, target_buckets);

            assert_eq!(
                expected, actual,
                "expected rescale({min}, {max}, {scale}) to be {expected} but got {actual}"
            );
            assert_eq!(actual, rescale(min, max, actual, target_buckets));
        }
    }

    #[test]
    fn decode_histogram() {
        // TODO: Replace this with a proper test once we've wired up histograms fully
        let encoded = sval_protobuf::stream_to_protobuf(ExponentialHistogram {
            data_points: &[ExponentialHistogramDataPoint {
                attributes: &[KeyValue {
                    key: "attribute_1",
                    value: AnyValue::String("value_1"),
                }],
                start_time_unix_nano: 1,
                time_unix_nano: 2,
                count: 8,
                scale: 1,
                zero_count: 3,
                positive: Some(Buckets {
                    offset: 2,
                    bucket_counts: &[1, 2, 3, 4, 5],
                }),
                negative: None,
            }],
            aggregation_temporality: AggregationTemporality::Delta,
        });

        assert!(metrics::ExponentialHistogram::decode(&*encoded.to_vec()).is_ok());
    }
}
