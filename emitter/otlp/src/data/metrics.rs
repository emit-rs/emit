mod export_metrics_service;
mod metric;

use std::ops::ControlFlow;

use crate::Error;

pub use self::{export_metrics_service::*, metric::*};

use emit::{Filter as _, Props as _};

use sval::Value;

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
            let mut attributes = Vec::new();

            let _ = evt.props().for_each(|k, v| match k.get() {
                // Well-known fields
                emit::well_known::KEY_METRIC_UNIT => {
                    metric_unit = Some(v);

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
                        data_points: &SumPoints::new(&attributes).points_from_value(
                            start_time_unix_nano,
                            time_unix_nano,
                            metric_value,
                        )?,
                    }),
                }),
                Some(emit::well_known::METRIC_AGG_COUNT) => E::encode(Metric::<_, _, _> {
                    name: &sval::Display::new(metric_name),
                    unit: &metric_unit.map(sval::Display::new),
                    data: &MetricData::Sum::<_>(Sum::<_> {
                        aggregation_temporality,
                        is_monotonic: true,
                        data_points: &SumPoints::new(&attributes).points_from_value(
                            start_time_unix_nano,
                            time_unix_nano,
                            metric_value,
                        )?,
                    }),
                }),
                _ => E::encode(Metric::<_, _, _> {
                    name: &sval::Display::new(metric_name),
                    unit: &metric_unit.map(sval::Display::new),
                    data: &MetricData::Gauge(Gauge::<_> {
                        data_points: &RawPointSet::new(&attributes).points_from_value(
                            start_time_unix_nano,
                            time_unix_nano,
                            metric_value,
                        )?,
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

trait DataPointBuilder {
    type Points;

    fn points_from_value(
        self,
        start_time_unix_nano: u64,
        time_unix_nano: u64,
        value: emit::Value<'_>,
    ) -> Option<Self::Points>
    where
        Self: Sized,
    {
        struct Extract<A> {
            in_seq: bool,
            aggregator: A,
        }

        impl<'sval, A: DataPointBuilder> sval::Stream<'sval> for Extract<A> {
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
                self.aggregator.push_point_i64(value);

                Ok(())
            }

            fn f64(&mut self, value: f64) -> sval::Result {
                self.aggregator.push_point_f64(value);

                Ok(())
            }

            fn seq_begin(&mut self, _: Option<usize>) -> sval::Result {
                if self.in_seq {
                    return sval::error();
                }

                self.in_seq = true;

                Ok(())
            }

            fn seq_value_begin(&mut self) -> sval::Result {
                Ok(())
            }

            fn seq_value_end(&mut self) -> sval::Result {
                Ok(())
            }

            fn seq_end(&mut self) -> sval::Result {
                self.in_seq = false;

                Ok(())
            }
        }

        let mut extract = Extract {
            in_seq: false,
            aggregator: self,
        };
        value.stream(&mut extract).ok()?;

        extract
            .aggregator
            .into_points(start_time_unix_nano, time_unix_nano)
    }

    fn push_point_i64(&mut self, value: i64);
    fn push_point_f64(&mut self, value: f64);

    fn into_points(self, start_time_unix_nano: u64, time_unix_nano: u64) -> Option<Self::Points>;
}

struct SumPoints<'a, A>(NumberDataPoint<'a, A>);

impl<'a, A> SumPoints<'a, A> {
    fn new(attributes: &'a A) -> Self {
        SumPoints(NumberDataPoint {
            attributes,
            start_time_unix_nano: Default::default(),
            time_unix_nano: Default::default(),
            value: NumberDataPointValue::AsInt(AsInt(0)),
        })
    }
}

impl<'a, A> DataPointBuilder for SumPoints<'a, A> {
    type Points = [NumberDataPoint<'a, A>; 1];

    fn push_point_i64(&mut self, value: i64) {
        self.0.value = match self.0.value {
            NumberDataPointValue::AsInt(AsInt(current)) => current
                .checked_add(value)
                .map(|value| NumberDataPointValue::AsInt(AsInt(value)))
                .unwrap_or(NumberDataPointValue::AsDouble(AsDouble(f64::INFINITY))),
            NumberDataPointValue::AsDouble(AsDouble(current)) => {
                NumberDataPointValue::AsDouble(AsDouble(current + value as f64))
            }
        };
    }

    fn push_point_f64(&mut self, value: f64) {
        self.0.value = match self.0.value {
            NumberDataPointValue::AsInt(AsInt(current)) => {
                NumberDataPointValue::AsDouble(AsDouble(value + current as f64))
            }
            NumberDataPointValue::AsDouble(AsDouble(current)) => {
                NumberDataPointValue::AsDouble(AsDouble(current + value))
            }
        };
    }

    fn into_points(
        mut self,
        start_time_unix_nano: u64,
        time_unix_nano: u64,
    ) -> Option<Self::Points> {
        self.0.start_time_unix_nano = start_time_unix_nano;
        self.0.time_unix_nano = time_unix_nano;

        Some([self.0])
    }
}

struct RawPointSet<'a, A> {
    attributes: &'a A,
    points: Vec<NumberDataPoint<'a, A>>,
}

impl<'a, A> RawPointSet<'a, A> {
    fn new(attributes: &'a A) -> Self {
        RawPointSet {
            attributes,
            points: Vec::new(),
        }
    }
}

impl<'a, A> DataPointBuilder for RawPointSet<'a, A> {
    type Points = Vec<NumberDataPoint<'a, A>>;

    fn push_point_i64(&mut self, value: i64) {
        self.points.push(NumberDataPoint {
            attributes: self.attributes,
            start_time_unix_nano: Default::default(),
            time_unix_nano: Default::default(),
            value: NumberDataPointValue::AsInt(AsInt(value)),
        });
    }

    fn push_point_f64(&mut self, value: f64) {
        self.points.push(NumberDataPoint {
            attributes: self.attributes,
            start_time_unix_nano: Default::default(),
            time_unix_nano: Default::default(),
            value: NumberDataPointValue::AsDouble(AsDouble(value)),
        });
    }

    fn into_points(
        mut self,
        start_time_unix_nano: u64,
        time_unix_nano: u64,
    ) -> Option<Self::Points> {
        match self.points.len() as u64 {
            0 => None,
            1 => {
                self.points[0].start_time_unix_nano = start_time_unix_nano;
                self.points[0].time_unix_nano = time_unix_nano;

                Some(self.points)
            }
            points => {
                let point_time_range = time_unix_nano.saturating_sub(start_time_unix_nano);
                let step = point_time_range / points;

                let mut point_time = start_time_unix_nano;
                for point in &mut self.points {
                    point.start_time_unix_nano = point_time;
                    point_time += step;
                    point.time_unix_nano = point_time;
                }

                Some(self.points)
            }
        }
    }
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

    use std::time::Duration;

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
    fn encode_count_histogram_int() {
        encode_event::<MetricsEventEncoder>(
            emit::evt!(
                extent: emit::Timestamp::MIN..(emit::Timestamp::MIN + Duration::from_secs(10)),
                "{metric_agg} of {metric_name} is {metric_value}",
                evt_kind: "metric",
                metric_name: "test",
                metric_agg: "count",
                #[emit::as_value]
                metric_value: [
                    1,
                    1,
                    1,
                    1,
                    1,
                    1,
                    1,
                    1,
                    1,
                    1,
                ],
            ),
            |buf| {
                let de = metrics::Metric::decode(buf).unwrap();

                assert_eq!("test", de.name);

                match de.data {
                    Some(metrics::metric::Data::Sum(sum)) => {
                        assert!(sum.is_monotonic);

                        assert_eq!(Some(int_point(10)), sum.data_points[0].value);
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
    fn encode_sum_histogram_int() {
        encode_event::<MetricsEventEncoder>(
            emit::evt!(
                extent: emit::Timestamp::MIN..(emit::Timestamp::MIN + Duration::from_secs(10)),
                "{metric_agg} of {metric_name} is {metric_value}",
                evt_kind: "metric",
                metric_name: "test",
                metric_agg: "sum",
                #[emit::as_value]
                metric_value: [
                    -1,
                    -1,
                    -1,
                    -1,
                    -1,
                    -1,
                    -1,
                    -1,
                    -1,
                    -1,
                ],
            ),
            |buf| {
                let de = metrics::Metric::decode(buf).unwrap();

                assert_eq!("test", de.name);

                match de.data {
                    Some(metrics::metric::Data::Sum(sum)) => {
                        assert!(!sum.is_monotonic);

                        assert_eq!(Some(int_point(-10)), sum.data_points[0].value);
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
    fn encode_gauge_histogram_int() {
        encode_event::<MetricsEventEncoder>(
            emit::evt!(
                extent: emit::Timestamp::MIN..(emit::Timestamp::MIN + Duration::from_secs(10)),
                "{metric_agg} of {metric_name} is {metric_value}",
                evt_kind: "metric",
                metric_name: "test",
                metric_agg: "max",
                #[emit::as_value]
                metric_value: [
                    -1,
                    -1,
                    -1,
                    -1,
                    -1,
                    -1,
                    -1,
                    -1,
                    -1,
                    -1,
                ],
            ),
            |buf| {
                let de = metrics::Metric::decode(buf).unwrap();

                assert_eq!("test", de.name);

                match de.data {
                    Some(metrics::metric::Data::Gauge(gauge)) => {
                        assert_eq!(10, gauge.data_points.len());
                    }
                    other => panic!("unexpected {other:?}"),
                }
            },
        );
    }
}
