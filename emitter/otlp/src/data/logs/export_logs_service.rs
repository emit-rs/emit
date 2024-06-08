use sval_derive::Value;

use crate::data::{InstrumentationScope, Resource};

use super::log_record::LogRecord;

#[derive(Value)]
pub struct ExportLogsServiceRequest<'a, RL: ?Sized = [ResourceLogs<'a>]> {
    #[sval(label = "resourceLogs", index = 1)]
    pub resource_logs: &'a RL,
}

#[derive(Value)]
pub struct ResourceLogs<'a, R: ?Sized = Resource<'a>, SL: ?Sized = [ScopeLogs<'a>]> {
    #[sval(label = "resource", index = 1)]
    pub resource: &'a R,
    #[sval(label = "scopeLogs", index = 2)]
    pub scope_logs: &'a SL,
}

#[derive(Value)]
pub struct ScopeLogs<'a, IS: ?Sized = InstrumentationScope<'a>, LR: ?Sized = &'a [LogRecord<'a>]> {
    #[sval(label = "scope", index = 1)]
    pub scope: &'a IS,
    #[sval(label = "logRecords", index = 2)]
    pub log_records: &'a LR,
}
