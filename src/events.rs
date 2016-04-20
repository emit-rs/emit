use chrono::{DateTime,UTC};
use std::collections;
use log::LogLevel;
use serde;
use serde_json;
use templates;

/// Converts an arbitrary serializable object into the internal property format
/// carried on events (currently JSON values...)
pub fn capture_property_value<T: serde::ser::Serialize>(v: &T) -> String {
    serde_json::to_string(v).unwrap()
}

pub struct Event {
    timestamp: DateTime<UTC>,
    level: LogLevel,
    message_template: templates::MessageTemplate,
    properties: collections::BTreeMap<&'static str, String>
}

impl Event {
    pub fn new(timestamp: DateTime<UTC>, level: LogLevel, message_template: templates::MessageTemplate, properties: collections::BTreeMap<&'static str, String>) -> Event {
        Event {
            timestamp: timestamp,
            level: level,
            message_template: message_template,
            properties: properties
        }
    }
    
    pub fn new_now(level: LogLevel, message_template: templates::MessageTemplate, properties: collections::BTreeMap<&'static str, String>) -> Event {
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
    
    pub fn properties(&self) -> &collections::BTreeMap<&'static str, String> {
        &self.properties
    }
}
