use sval_derive::Value;

use crate::data::{InstrumentationScope, Resource};

use super::Span;

#[derive(Value)]
pub struct ExportTraceServiceRequest<'a, RS: ?Sized = [ResourceSpans<'a>]> {
    #[sval(label = "resourceSpans", index = 1)]
    pub resource_spans: &'a RS,
}

#[derive(Value)]
pub struct ResourceSpans<'a, R: ?Sized = Resource<'a>, SS: ?Sized = [ScopeSpans<'a>]> {
    #[sval(label = "resource", index = 1)]
    pub resource: &'a R,
    #[sval(label = "scopeSpans", index = 2)]
    pub scope_spans: &'a SS,
}

#[derive(Value)]
pub struct ScopeSpans<'a, IS: ?Sized = InstrumentationScope<'a>, S: ?Sized = &'a [Span<'a>]> {
    #[sval(label = "scope", index = 1)]
    pub scope: &'a IS,
    #[sval(label = "spans", index = 2)]
    pub spans: &'a S,
}
