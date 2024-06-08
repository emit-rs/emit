use sval_derive::Value;

use crate::data::{InstrumentationScope, Resource};

use super::Metric;

#[derive(Value)]
pub struct ExportMetricsServiceRequest<'a, RM: ?Sized = [ResourceMetrics<'a>]> {
    #[sval(label = "resourceMetrics", index = 1)]
    pub resource_metrics: &'a RM,
}

#[derive(Value)]
pub struct ResourceMetrics<'a, R: ?Sized = Resource<'a>, SM: ?Sized = [ScopeMetrics<'a>]> {
    #[sval(label = "resource", index = 1)]
    pub resource: &'a R,
    #[sval(label = "scopeMetrics", index = 2)]
    pub scope_metrics: &'a SM,
}

#[derive(Value)]
pub struct ScopeMetrics<'a, IS: ?Sized = InstrumentationScope<'a>, M: ?Sized = &'a [Metric<'a>]> {
    #[sval(label = "scope", index = 1)]
    pub scope: &'a IS,
    #[sval(label = "metrics", index = 2)]
    pub metrics: &'a M,
}
