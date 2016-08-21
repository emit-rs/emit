use chrono::{DateTime,UTC};
use std::collections;
use std::collections::btree_map::Entry;
use LogLevel;
use templates;

#[derive(Clone, PartialEq)]
pub enum Value {
    /// Represents null
    Null,

    /// Represents a Boolean
    Bool(bool),

    /// Represents a signed integer
    I64(i64),

    /// Represents an unsigned integer
    U64(u64),

    /// Represents a floating point number
    F64(f64),

    /// Represents a string
    String(String),

    /// Represents a collection
    Vec(Vec<Value>)
}

/// Represents a type that can be converted into a `Value`.
pub trait IntoValue {
    fn into_value(self) -> Value;
}

#[derive(Debug, Clone)]
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
    
    pub fn add_or_update_property<I: IntoValue>(&mut self, name: &'a str, value: I) {
        match self.properties.entry(name) {
            Entry::Vacant(v) => {v.insert(value.into_value());},
            Entry::Occupied(mut o) => {o.insert(value.into_value());}
        }
    }
    
    pub fn add_property_if_absent<I: IntoValue>(&mut self, name: &'a str, value: I) {
        if !self.properties.contains_key(name) {
            self.properties.insert(name, value.into_value());
        }
    }
}