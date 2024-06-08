use sval_derive::Value;

use crate::data::{AnyValue, KeyValue};

#[derive(Value)]
pub struct Metric<'a, N: ?Sized = str, U: ?Sized = str, D: ?Sized = MetricData<'a>> {
    #[sval(label = "name", index = 1)]
    pub name: &'a N,
    #[sval(label = "unit", index = 3)]
    pub unit: &'a U,
    #[sval(flatten)]
    pub data: &'a D,
}

#[derive(Value)]
pub enum MetricData<'a, DP: ?Sized = [NumberDataPoint<'a>]> {
    #[sval(label = "gauge", index = 5)]
    Gauge(Gauge<'a, DP>),
    #[sval(label = "sum", index = 7)]
    Sum(Sum<'a, DP>),
}

#[derive(Value)]
pub struct Gauge<'a, DP: ?Sized = [NumberDataPoint<'a>]> {
    #[sval(label = "dataPoints", index = 1)]
    pub data_points: &'a DP,
}

#[derive(Value)]
pub struct Sum<'a, DP: ?Sized = [NumberDataPoint<'a>]> {
    #[sval(label = "dataPoints", index = 1)]
    pub data_points: &'a DP,
    #[sval(label = "aggregationTemporality", index = 2)]
    pub aggregation_temporality: AggregationTemporality,
    #[sval(label = "isMonotonic", index = 3)]
    pub is_monotonic: bool,
}

#[derive(Value)]
#[repr(i32)]
#[sval(unlabeled_variants)]
pub enum AggregationTemporality {
    Unspecified = 0,
    Delta = 1,
    Cumulative = 2,
}

#[derive(Value)]
pub struct NumberDataPoint<'a, A: ?Sized = [KeyValue<&'a str, &'a AnyValue<'a>>]> {
    #[sval(label = "attributes", index = 7)]
    pub attributes: &'a A,
    #[sval(
        label = "startTimeUnixNano",
        index = 2,
        data_tag = "sval_protobuf::tags::PROTOBUF_I64"
    )]
    pub start_time_unix_nano: u64,
    #[sval(
        label = "timeUnixNano",
        index = 3,
        data_tag = "sval_protobuf::tags::PROTOBUF_I64"
    )]
    pub time_unix_nano: u64,
    #[sval(flatten)]
    pub value: NumberDataPointValue,
}

#[derive(Value)]
pub enum NumberDataPointValue {
    #[sval(label = "value", index = 4)]
    AsDouble(AsDouble),
    #[sval(label = "value", index = 6)]
    AsInt(AsInt),
}

#[derive(Value)]
pub struct AsDouble(pub f64);

#[derive(Value)]
#[sval(tag = "sval_protobuf::tags::PROTOBUF_I64")]
pub struct AsInt(pub i64);
