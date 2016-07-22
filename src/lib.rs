//! Structured application logging.
//!
//! This module provides:
//!
//!  * The `emit!()` family of macros, for recording application events in an easily machine-readable format
//!  * Collectors for transmitting these events to back-end logging servers
//!
//! The emit macros are "structured" versions of the ones in the widely-used `log` crate. The `log` crate doesn't preserve the structure
//! of the events that are written through it. For example:
//!
//! ```ignore
//! info!("Hello, {}!", env::var("USERNAME").unwrap());
//! ```
//!
//! This writes `"Hello, nblumhardt!"` to the log as an block of text, which can't later be broken apart to reveal the username except
//! with regular expressions.
//!
//! The arguments passed to `emit` are named:
//!
//! ```ignore
//! info!("Hello, {}!", user: env::var("USERNAME").unwrap());
//! ```
//!
//! This event can be rendered into text identically to the `log` example, but structured data collectors also capture the
//! aguments as a key/value property pairs.
//!
//! ```js
//! {
//!   "@t": "2016-03-17T00:17:01.333Z",
//!   "@mt": "Hello, {user}!",
//!   "user": "nblumhardt",
//!   "target": "example_app"
//! }
//! ```
//!
//! Back-ends like Elasticsearch, Seq, and Splunk use these in queries such as `user == "nblumhardt"` without up-front log parsing.
//!
//! Arguments are captured using `serde`, so there's the potential for complex values to be logged so long as they're `serde::ser::Serialize`.
//!
//! Further, because the template (format) itself is captured, it can be hashed to compute an "event type" for precisely finding
//! all occurrences of the event regardless of the value of the `user` argument.
//!
//! # Examples
//!
//! The example below writes events to stdout.
//!
//! ```
//! #[macro_use]
//! extern crate emit;
//!
//! use std::env;
//! use emit::PipelineBuilder;
//! use emit::collectors::stdio::StdioCollector;
//! use emit::formatters::raw::RawFormatter;
//!
//! fn main() {
//!     let _flush = PipelineBuilder::new()
//!         .write_to(StdioCollector::new(RawFormatter::new()))
//!         .init();
//!
//!     info!("Hello, {}!", name: env::var("USERNAME").unwrap_or("User".to_string()));
//! }
//! ```
//!
//! Output:
//!
//! ```text
//! emit 2016-03-24T05:03:36Z Hello, {name}!
//!   name: "nblumhardt"
//!   target: "example_app"
//!
//! ```

extern crate chrono;

pub mod templates;
pub mod events;
pub mod pipeline;
pub mod collectors;
pub mod enrichers;
pub mod formatters;

#[cfg(test)]
mod test_support;

use std::fmt;

#[repr(usize)]
#[derive(Copy, Eq, Debug, PartialEq, Clone, Ord, PartialOrd)]
pub enum LogLevel {
    Error = 1, 
    Warn, 
    Info, 
    Debug, 
    Trace
}

#[repr(usize)]
#[derive(Copy, Eq, Debug, PartialEq, Clone, Ord, PartialOrd)]
pub enum LogLevelFilter {
    Off,
    Error, 
    Warn, 
    Info, 
    Debug, 
    Trace
}

static LOG_LEVEL_NAMES: [&'static str; 6] = ["OFF", "ERROR", "WARN", "INFO", "DEBUG", "TRACE"];

impl fmt::Display for LogLevel {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}", LOG_LEVEL_NAMES[*self as usize])
    }
}

impl LogLevelFilter {
    pub fn is_enabled(self, level: LogLevel) -> bool {
        self as usize >= level as usize
    }
}

/// Re-export `pipeline::builder::PipelineBuilder` so that clients don't need to
/// fully-qualify the hierarchy.
pub use pipeline::builder::PipelineBuilder;

#[macro_export]
#[doc(hidden)]
macro_rules! __emit_get_event_data {
    ($target:expr, $s:expr, $( $n:ident: $v:expr ),* ) => {{
        #[allow(unused_imports)]
        use std::fmt::Write;
        use std::collections;

        // Underscores avoid the unused_mut warning
        let mut _names: Vec<&str> = vec![];
        let mut properties: collections::BTreeMap<&'static str, $crate::events::Value> = collections::BTreeMap::new();

        $(
            _names.push(stringify!($n));
            properties.insert(stringify!($n), $v.into());
        )*

        properties.insert("target", $target.into());

        let template = $crate::templates::MessageTemplate::from_format($s, &_names);

        (template, properties)
    }};
}

/// Emit an event to the ambient pipeline.
///
/// # Examples
///
/// The event below collects a `user` property and is emitted if the pipeline level includes
/// `LogLevel::Info`.
///
/// ```ignore
/// emit!(emit::LogLevel::Info, "Hello, {}!", user: env::var("USERNAME").unwrap());
/// ```
///
/// A `target` expression may be specified if required. When omitted the `target` property
/// will carry the current module name.
///
/// ```ignore
/// emit!(target: "greetings", emit::LogLevel::Info, "Hello, {}!", user: env::var("USERNAME").unwrap());
/// ```
#[macro_export]
macro_rules! emit {
    (target: $target:expr, $lvl:expr, $s:expr, $($n:ident: $v:expr),*) => {{
        let lvl = $lvl;
        if $crate::pipeline::ambient::is_enabled(lvl) {
            let (template, properties) = __emit_get_event_data!($target, $s, $($n: $v),*);
            let event = $crate::events::Event::new_now(lvl, template, properties);
            $crate::pipeline::ambient::emit(event);
        }
    }};

    ($lvl:expr, $s:expr, $($n:ident: $v:expr),*) => {{
        emit!(target: module_path!(), $lvl, $s, $($n: $v),*);
    }};
}

/// Emit an error event to the ambient pipeline.
///
/// # Examples
///
/// The example below will be emitted at the `emit::LogLevel::Error` level.
///
/// ```ignore
/// error!("Could not start {} on {}", cmd: "emitd", machine: "s123456");
/// ```
#[macro_export]
macro_rules! error {
    (target: $target:expr, $s:expr, $($n:ident: $v:expr),*) => {{
        emit!(target: $target, $crate::LogLevel::Error, $s, $($n: $v),*);
    }};

    ($s:expr, $($n:ident: $v:expr),*) => {{
        emit!($crate::LogLevel::Error, $s, $($n: $v),*);
    }};
}

/// Emit a warning event to the ambient pipeline.
///
/// # Examples
///
/// The example below will be emitted at the `emit::LogLevel::Warn` level.
///
/// ```ignore
/// warn!("SQL query took {} ms", elapsed: 7890);
/// ```
#[macro_export]
macro_rules! warn {
    (target: $target:expr, $s:expr, $($n:ident: $v:expr),*) => {{
        emit!(target: $target, $crate::LogLevel::Warn, $s, $($n: $v),*);
    }};

    ($s:expr, $($n:ident: $v:expr),*) => {{
        emit!($crate::LogLevel::Warn, $s, $($n: $v),*);
    }};
}

/// Emit an informational event to the ambient pipeline.
///
/// # Examples
///
/// The example below will be emitted at the `emit::LogLevel::Info` level.
///
/// ```ignore
/// info!("Hello, {}!", user: env::var("USERNAME").unwrap());
/// ```
#[macro_export]
macro_rules! info {
    (target: $target:expr, $s:expr, $($n:ident: $v:expr),*) => {{
        emit!(target: $target, $crate::LogLevel::Info, $s, $($n: $v),*);
    }};

    ($s:expr, $($n:ident: $v:expr),*) => {{
        emit!($crate::LogLevel::Info, $s, $($n: $v),*);
    }};
}

/// Emit a debugging event to the ambient pipeline.
///
/// # Examples
///
/// The example below will be emitted at the `emit::LogLevel::Debug` level.
///
/// ```ignore
/// debug!("Opening config file {}", filename: "dir/config.json");
/// ```
#[macro_export]
macro_rules! debug {
    (target: $target:expr, $s:expr, $($n:ident: $v:expr),*) => {{
        emit!(target: $target, $crate::LogLevel::Debug, $s, $($n: $v),*);
    }};

    ($s:expr, $($n:ident: $v:expr),*) => {{
        emit!($crate::LogLevel::Debug, $s, $($n: $v),*);
    }};
}

/// Emit a trace event to the ambient pipeline.
///
/// # Examples
///
/// The example below will be emitted at the `emit::LogLevel::Trace` level.
///
/// ```ignore
/// emdtrace!("{} called with arg {}", method: "start_frobbles()", count: 123);
/// ```
#[macro_export]
macro_rules! trace {
    (target: $target:expr, $s:expr, $($n:ident: $v:expr),*) => {{
        emit!(target: $target, $crate::LogLevel::Trace, $s, $($n: $v),*);
    }};

    ($s:expr, $($n:ident: $v:expr),*) => {{
        emit!($crate::LogLevel::Trace, $s, $($n: $v),*);
    }};
}

#[cfg(test)]
mod tests {
    use enrichers::fixed_property::FixedPropertyEnricher;
    use pipeline::builder::PipelineBuilder;
    use std::env;
    use LogLevelFilter;
    use collectors::stdio::StdioCollector;
    use formatters::json::{JsonFormatter,RenderedJsonFormatter};
    use formatters::text::PlainTextFormatter;
    use formatters::raw::RawFormatter;

    #[test]
    fn unparameterized_templates_are_captured() {
        let (template, properties) = __emit_get_event_data!("t", "Starting...",);
        assert_eq!(template.text(), "Starting...");
        assert_eq!(properties.len(), 1);
    }

    #[test]
    fn template_and_properties_are_captured() {
        let u = "nblumhardt";
        let q = 42;

        let (template, properties) = __emit_get_event_data!("t", "User {} exceeded quota of {}!", user: u, quota: q);
        assert_eq!(template.text(), "User {user} exceeded quota of {quota}!");
        assert_eq!(properties.get("user"), Some(&"\"nblumhardt\"".into()));
        assert_eq!(properties.get("quota"), Some(&42.into()));
        assert_eq!(properties.len(), 3);
    }

    #[test]
    fn pipeline_example() {
        let _flush = PipelineBuilder::new()
            .at_level(LogLevelFilter::Info)
            .pipe(Box::new(FixedPropertyEnricher::new("app", "Test")))
            .write_to(StdioCollector::new(PlainTextFormatter::new()))
            .write_to(StdioCollector::new(JsonFormatter::new()))
            .write_to(StdioCollector::new(RenderedJsonFormatter::new()))
            .write_to(StdioCollector::new(RawFormatter::new()))
            .init();

        info!("Hello, {} at {} in {}!", name: env::var("USERNAME").unwrap_or("User".to_string()), time: 2139, room: "office");
    }
}
