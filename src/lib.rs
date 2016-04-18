//! Structured application logging.
//!
//! This module provides:
//!
//!  * The `emit!()` family of macros, for recording application events in an easily machine-readable format
//!  * Collectors for transmitting these events to back-end logging servers
//!
//! The widely-used `log` crate doesn't preserve the structure of the events that are written through it. For example:
//! 
//! ```ignore
//! info!("Hello, {}!", env::var("USERNAME").unwrap());
//! ```
//! 
//! This writes `"Hello, nblumhardt!"` to the log as an block of text, that can't be broken apart to reveal the username except 
//! with regular expressions.
//! 
//! The arguments passed to `emit` are named:
//! 
//! ```ignore
//! eminfo!("Hello, {}!", user: env::var("USERNAME").unwrap());
//! ```
//! 
//! This event can be rendered into text identically to the `log` example, but structured data collectors also capture the
//! aguments as a key/value property pairs.
//! 
//! ```js
//! {
//!   "timestamp": "2016-03-17T00:17:01Z",
//!   "messageTemplate": "Hello, {name}!",
//!   "properties": {
//!     "name": "nblumhardt",
//!     "target": "example_app"
//!   }
//! }
//! ```
//! 
//! Back-ends like Elasticsearch, Seq, and Splunk use these in queries like `user == "nblumhardt"` without up-front log parsing.
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
//! # extern crate log;
//! 
//! use std::env;
//! use emit::pipeline;
//! use emit::collectors::stdio;
//! 
//! fn main() {
//!     let _flush = pipeline::init(stdio::StdioCollector::new(), emit::LogLevel::Info);
//! 
//!     eminfo!("Hello, {}!", name: env::var("USERNAME").unwrap());
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

extern crate serde;
extern crate serde_json;
extern crate chrono;

#[macro_use]
extern crate hyper;

#[macro_use]
extern crate log;

pub mod message_templates;
pub mod events;
pub mod pipeline;
pub mod collectors;

/// Re-exports log::LogLevel so that users can initialize the emit
/// crate without extra imports.
pub use log::LogLevel;

#[macro_export]
#[doc(hidden)]
macro_rules! __emit_get_event_data {
    ($target:expr, $s:expr, $( $n:ident: $v:expr ),* ) => {{
        #[allow(unused_imports)]
        use std::fmt::Write;
        use std::collections;

        // Underscores avoid the unused_mut warning
        let mut _names: Vec<&str> = vec![];
        let mut properties: collections::BTreeMap<&'static str, String> = collections::BTreeMap::new();

        $(
            _names.push(stringify!($n));
            properties.insert(stringify!($n), $crate::message_templates::capture(&$v));            
        )*
        
        properties.insert("target", $crate::message_templates::capture(&$target));
        
        let template = $crate::message_templates::build_template($s, &_names);
                
        (template, properties)
    }};
}

#[macro_export]
macro_rules! emit {
    (target: $target:expr, $lvl:expr, $s:expr, $($n:ident: $v:expr),*) => {{
        let lvl = $lvl;
        if $crate::pipeline::is_enabled(lvl) {
            let (template, properties) = __emit_get_event_data!($target, $s, $($n: $v),*);
            let event = $crate::events::Event::new_now(lvl, template, properties);
            $crate::pipeline::emit(event);
        }
    }};
    
    ($lvl:expr, $s:expr, $($n:ident: $v:expr),*) => {{
        emit!(target: module_path!(), $lvl, $s, $($n: $v),*);
    }};
}

#[macro_export]
macro_rules! emerror {
    (target: $target:expr, $s:expr, $($n:ident: $v:expr),*) => {{
        #[allow(unused_imports)]
        use log;
        emit!(target: $target, log::LogLevel::Error, $s, $($n: $v),*);
    }};
    
    ($s:expr, $($n:ident: $v:expr),*) => {{
        #[allow(unused_imports)]
        use log;
        emit!(log::LogLevel::Error, $s, $($n: $v),*);
    }};
}

#[macro_export]
macro_rules! emwarn {
    (target: $target:expr, $s:expr, $($n:ident: $v:expr),*) => {{
        #[allow(unused_imports)]
        use log;
        emit!(target: $target, log::LogLevel::Warn, $s, $($n: $v),*);
    }};
    
    ($s:expr, $($n:ident: $v:expr),*) => {{
        #[allow(unused_imports)]
        use log;
        emit!(log::LogLevel::Warn, $s, $($n: $v),*);
    }};
}

#[macro_export]
macro_rules! eminfo {
    (target: $target:expr, $s:expr, $($n:ident: $v:expr),*) => {{
        #[allow(unused_imports)]
        use log;
        emit!(target: $target, log::LogLevel::Info, $s, $($n: $v),*);
    }};
    
    ($s:expr, $($n:ident: $v:expr),*) => {{
        #[allow(unused_imports)]
        use log;
        emit!(log::LogLevel::Info, $s, $($n: $v),*);
    }};
}

#[macro_export]
macro_rules! emdebug {
    (target: $target:expr, $s:expr, $($n:ident: $v:expr),*) => {{
        #[allow(unused_imports)]
        use log;
        emit!(target: $target, log::LogLevel::Debug, $s, $($n: $v),*);
    }};
    
    ($s:expr, $($n:ident: $v:expr),*) => {{
        #[allow(unused_imports)]
        use log;
        emit!(log::LogLevel::Debug, $s, $($n: $v),*);
    }};
}

#[macro_export]
macro_rules! emtrace {
    (target: $target:expr, $s:expr, $($n:ident: $v:expr),*) => {{
        #[allow(unused_imports)]
        use log;
        emit!(target: $target, log::LogLevel::Trace, $s, $($n: $v),*);
    }};
    
    ($s:expr, $($n:ident: $v:expr),*) => {{
        #[allow(unused_imports)]
        use log;
        emit!(log::LogLevel::Trace, $s, $($n: $v),*);
    }};
}

#[cfg(test)]
mod tests {
    // use collectors::stdio;
    use collectors::seq;
    use pipeline;
    use std::env;
    use log;
    
    #[test]
    fn unparameterized_templates_are_captured() {
        let (template, properties) = __emit_get_event_data!("t", "Starting...",);
        assert_eq!(template, "Starting...");
        assert_eq!(properties.len(), 1);
    }
    
    #[test]
    fn template_and_properties_are_captured() {
        let u = "nblumhardt";
        let q = 42;
        
        let (template, properties) = __emit_get_event_data!("t", "User {} exceeded quota of {}!", user: u, quota: q);
        assert_eq!(template, "User {user} exceeded quota of {quota}!");
        assert_eq!(properties.get("user"), Some(&"\"nblumhardt\"".to_owned()));
        assert_eq!(properties.get("quota"), Some(&"42".to_owned()));
        assert_eq!(properties.len(), 3);
    }    

    #[test]
    fn emitted_events_are_flushed() {
        // let _flush = pipeline::init(stdio::StdioCollector::new(), log::LogLevel::Info);
        let _flush = pipeline::init(seq::SeqCollector::new_local(), log::LogLevel::Info);
        eminfo!("Hello, {}!", name: env::var("USERNAME").unwrap());        
    }
}
