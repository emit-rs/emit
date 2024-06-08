mod export_metrics_service;
mod metric;

use std::ops::ControlFlow;

use crate::Error;

pub use self::{export_metrics_service::*, metric::*};

use emit::{
    well_known::{
        KEY_EVENT_KIND, KEY_METRIC_AGG, KEY_METRIC_NAME, KEY_METRIC_UNIT, KEY_METRIC_VALUE,
        KEY_SPAN_ID, KEY_SPAN_PARENT, KEY_TRACE_ID, METRIC_AGG_COUNT, METRIC_AGG_SUM,
    },
    Filter, Props,
};

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
        if let Some(name) = evt.props().get(KEY_METRIC_NAME) {
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
            evt.props().get(KEY_METRIC_VALUE),
            evt.props().get(KEY_METRIC_AGG),
        ) {
            let (start_time_unix_nano, time_unix_nano, aggregation_temporality) = evt
                .extent()
                .map(|extent| {
                    let range = extent.as_range();

                    (
                        range.start.to_unix().as_nanos() as u64,
                        range.end.to_unix().as_nanos() as u64,
                        if extent.is_span() {
                            AggregationTemporality::Delta
                        } else {
                            AggregationTemporality::Cumulative
                        },
                    )
                })
                .unwrap_or((0, 0, AggregationTemporality::Unspecified));

            let metric_name = MessageRenderer {
                fmt: &self.name,
                evt,
            };

            let mut metric_unit = None;
            let mut attributes = Vec::new();

            evt.props().for_each(|k, v| match k.get() {
                KEY_METRIC_UNIT => {
                    metric_unit = Some(v);

                    ControlFlow::Continue(())
                }
                KEY_METRIC_NAME | KEY_METRIC_VALUE | KEY_METRIC_AGG | KEY_SPAN_ID
                | KEY_SPAN_PARENT | KEY_TRACE_ID | KEY_EVENT_KIND => ControlFlow::Continue(()),
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
                Some(METRIC_AGG_SUM) => E::encode(Metric::<_, _, _> {
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
                Some(METRIC_AGG_COUNT) => E::encode(Metric::<_, _, _> {
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
                scope: evt.module().to_owned(),
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
