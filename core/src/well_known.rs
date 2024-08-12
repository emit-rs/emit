/*!
Extensions to the diagnostic model using well-known properties.

Components can use the presence of well-known properties to change the way they interpret events. The current categories of well-known properties are:

- Built-in:
    - [`KEY_MODULE`]: The [`crate::event::Event::module()`].
    - [`KEY_TS`]: The [`crate::event::Event::ts()`].
    - [`KEY_TS_START`]: The [`crate::event::Event::ts_start()`].
    - [`KEY_TPL`]: The [`crate::event::Event::tpl()`].
    - [`KEY_MSG`]: The [`crate::event::Event::msg()`].

- Logging:
    - [`KEY_LVL`]: A severity level to categorize the event by.
        - [`LVL_DEBUG`]: A weakly informative event.
        - [`LVL_INFO`]: An informative event.
        - [`LVL_WARN`]: A weakly erroneous event.
        - [`LVL_ERROR`]: An erroneous event.
    - [`KEY_ERR`]: A [`std::error::Error`] associated with the event.

Extensions to the data model are signaled by the well-known [`KEY_EVT_KIND`] property.

- Tracing [`KEY_EVT_KIND`] = [`EVENT_KIND_SPAN`]:
    - [`KEY_SPAN_NAME`]: The informative name of the span.
    - [`KEY_TRACE_ID`]: The trace id.
    - [`KEY_SPAN_ID`]: The span id.
    - [`KEY_SPAN_PARENT`]: The parent span id.

- Metrics [`KEY_EVT_KIND`] = [`EVENT_KIND_METRIC`]:
    - [`KEY_METRIC_NAME`]: The name of the underlying data source.
    - [`KEY_METRIC_AGG`]: The aggregation applied to the underlying data source to produce a sample.
        - [`METRIC_AGG_SUM`]: The sample is the possibly non-monotonic sum of values.
        - [`METRIC_AGG_COUNT`]: The sample is the count of defined values. The value is non-negative and monotonic.
        - [`METRIC_AGG_MIN`]: The sample is the minimum defined value.
        - [`METRIC_AGG_MAX`]: The sample is the maximum defined value.
        - [`METRIC_AGG_LAST`]: The sample is the last or most recent value.
    - [`KEY_METRIC_VALUE`]: The sample itself.
    - [`KEY_METRIC_UNIT`]: The measurement unit the sample is in.
*/

// Event
/** The [`crate::event::Event::module()`]. */
pub const KEY_MODULE: &'static str = "module";
/** The [`crate::event::Event::ts()`]. */
pub const KEY_TS: &'static str = "ts";
/** The [`crate::event::Event::ts_start()`]. */
pub const KEY_TS_START: &'static str = "ts_start";
/** The [`crate::event::Event::tpl()`]. */
pub const KEY_TPL: &'static str = "tpl";
/** The [`crate::event::Event::msg()`]. */
pub const KEY_MSG: &'static str = "msg";
/** The kind of extension the event belongs to. */
pub const KEY_EVT_KIND: &'static str = "evt_kind";

/** The event is a span in a distributed trace. */
pub const EVENT_KIND_SPAN: &'static str = "span";
/** The event is a metric sample. */
pub const EVENT_KIND_METRIC: &'static str = "metric";

// Log
/** A severity level to categorize the event by. */
pub const KEY_LVL: &'static str = "lvl";

/** A weakly informative event. */
pub const LVL_DEBUG: &'static str = "debug";
/** An informative event. */
pub const LVL_INFO: &'static str = "info";
/** A weakly erroneous event. */
pub const LVL_WARN: &'static str = "warn";
/** An erroneous event. */
pub const LVL_ERROR: &'static str = "error";

// Error
/**  A [`std::error::Error`] associated with the event. */
pub const KEY_ERR: &'static str = "err";

// Trace
/** The informative name of the span. */
pub const KEY_SPAN_NAME: &'static str = "span_name";
/** The trace id. */
pub const KEY_TRACE_ID: &'static str = "trace_id";
/** The span id. */
pub const KEY_SPAN_ID: &'static str = "span_id";
/** The parent span id. */
pub const KEY_SPAN_PARENT: &'static str = "span_parent";

// Metric
/** The name of the underlying data source. */
pub const KEY_METRIC_NAME: &'static str = "metric_name";
/** The aggregation applied to the underlying data source to produce a sample. */
pub const KEY_METRIC_AGG: &'static str = "metric_agg";
/** The sample itself. */
pub const KEY_METRIC_VALUE: &'static str = "metric_value";
/** The measurement unit the sample is in. */
pub const KEY_METRIC_UNIT: &'static str = "metric_unit";

/** The sample is the possibly non-monotonic sum of values. */
pub const METRIC_AGG_SUM: &'static str = "sum";
/** The sample is the count of defined values. The value is non-negative and monotonic. */
pub const METRIC_AGG_COUNT: &'static str = "count";
/** The sample is the minimum defined value. */
pub const METRIC_AGG_MIN: &'static str = "min";
/** The sample is the maximum defined value. */
pub const METRIC_AGG_MAX: &'static str = "max";
/** The sample is the last or most recent value. */
pub const METRIC_AGG_LAST: &'static str = "last";
