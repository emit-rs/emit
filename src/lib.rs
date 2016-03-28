extern crate serde;
extern crate serde_json;
extern crate chrono;
extern crate hyper;

#[macro_use]
extern crate log;

pub mod message_templates;
pub mod events;
pub mod pipeline;
pub mod collectors;

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
//! ```no_run
//! emit!("Hello, {}!", user: env::var("USERNAME").unwrap());
//! ```
//! 
//! This event can be rendered into text identically to the `log` example, but structured data collectors also capture the
//! aguments as a key/value property pairs.
//! 
//! ```json
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
//! 
//! use std::env;
//! use emit::pipeline;
//! use emit::collectors::stdio;
//! 
//! fn main() {
//!     let _flush = pipeline::init(stdio::StdioCollector::new());
//! 
//!     emit!("Hello, {}!", name: env::var("USERNAME").unwrap());
//! }
//! ```
//! 
//! Output:
//! 
//! ```text
//! emit 2016-03-24T05:03:36Z Hello, {name}!
//!   name: "nblumhardt"
//!   target: "web_we_are"
//! 
//! ```

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
    ( target: $target:expr, $s:expr, $( $n:ident: $v:expr ),* ) => {{
        #[allow(unused_imports)]
        use log;
        log!(target: $target, log::LogLevel::Info, $s, $($v),*);
        let (template, properties) = __emit_get_event_data!($target, $s, $($n: $v),*);
        let event = $crate::events::Event::new_now(template, properties);
        $crate::pipeline::emit(event);
    }};
    
    ( target: $target:expr, $s:expr ) => {{
        emit!(target: $target, $s,);
    }};
    
    ( $s:expr, $( $n:ident: $v:expr ),* ) => {{
        emit!(target: module_path!(), $s, $($n: $v),*);
    }};
    
    ( $s:expr ) => {{
        emit!(target: module_path!(), $s,);
    }};
}

#[cfg(test)]
mod tests {
    use collectors::seq;
    use pipeline;
    use std::env;
    
    #[test]
    fn unparameterized_templates_are_captured() {
        let (template, properties) = __emit_get_event_data!("t", "Starting...",);
        assert!(template == "Starting...");
        assert!(properties.len() == 1);
    }
    
    #[test]
    fn template_and_properties_are_captured() {
        let u = "nblumhardt";
        let q = 42;
        
        let (template, properties) = __emit_get_event_data!("t", "User {} exceeded quota of {}!", user: u, quota: q);
        assert!(template == "User {user} exceeded quota of {quota}!");
        assert!(properties.get("user") == Some(&"\"nblumhardt\"".to_owned()));
        assert!(properties.get("quota") == Some(&"42".to_owned()));
        assert!(properties.len() == 3);
    }    

    #[test]
    pub fn emitted_events_are_flushed() {
        let _flush = pipeline::init(seq::SeqCollector::new_local());
        emit!("Hello, {}!", name: env::var("USERNAME").unwrap());
    }
}
