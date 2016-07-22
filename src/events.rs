use chrono::{DateTime,UTC};
use std::collections;
use std::collections::btree_map::Entry;
use LogLevel;
use templates;
use std::fmt;
use std::convert::Into;

/// Currently just used as a marker; plan is to
/// adopt the same scheme as serde_json's Value: https://github.com/serde-rs/json/blob/master/json/src/value.rs
#[derive(Clone, PartialEq)]
pub enum Value {
    Json(String)
}

impl Value {
    // JSON serialization belongs elsewhere, but keeps
    // the distinction between regular string and JSON data clear.
    pub fn to_json<'a>(&'a self) -> &'a str {
        match self {
            &Value::Json(ref s) => &s
        }
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Value::Json(ref s) => write!(f, "Value({})", s)
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Value::Json(ref s) => {                    
                let bytes = s.as_bytes();            
                if bytes.len() > 1 && bytes[0] == b'"' {
                    write!(f, "{}", &s[1..s.len() - 1])
                } else {
                    write!(f, "{}", s)
                }
            }
        }
    }
}

impl Into<Value> for String {
    fn into(self) -> Value {
        Value::Json(self)
    }
}

impl<'a> Into<Value> for &'a str {
    fn into(self) -> Value {
        Value::Json(self.into())
    }
}

pub struct Event<'a> {
    timestamp: DateTime<UTC>,
    level: LogLevel,
    message_template: templates::MessageTemplate,
    properties: collections::BTreeMap<&'a str, Value>
}

impl<'a> Event<'a> {
    pub fn new(timestamp: DateTime<UTC>, level: LogLevel, message_template: templates::MessageTemplate, properties: collections::BTreeMap<&'a str, Value>) -> Event<'a> {
        Event {
            timestamp: timestamp,
            level: level,
            message_template: message_template,
            properties: properties
        }
    }
    
    pub fn new_now(level: LogLevel, message_template: templates::MessageTemplate, properties: collections::BTreeMap<&'a str, Value>) -> Event<'a> {
        Self::new(UTC::now(), level, message_template, properties)
    }
    
    pub fn timestamp(&self) -> DateTime<UTC> {
        self.timestamp
    }
    
    pub fn level(&self) -> LogLevel {
        self.level
    }
    
    pub fn message_template(&self) -> &templates::MessageTemplate {
        &self.message_template
    }

    pub fn message(&self) -> String {
        let repl = self.message_template.parse();
        repl.replace(self.properties())
    }
    
    pub fn properties(&self) -> &collections::BTreeMap<&'a str, Value> {
        &self.properties
    }
    
    pub fn add_or_update_property(&mut self, name: &'a str, value: Value) {
        match self.properties.entry(name) {
            Entry::Vacant(v) => {v.insert(value);},
            Entry::Occupied(mut o) => {o.insert(value);}
        }
    }
    
    pub fn add_property_if_absent(&mut self, name: &'a str, value: Value) {
        if !self.properties.contains_key(name) {
            self.properties.insert(name, value);
        }
    }
}
