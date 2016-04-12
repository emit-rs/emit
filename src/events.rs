use chrono::{DateTime,UTC};
use std::collections;
use log::LogLevel;

pub struct Event {
    timestamp: DateTime<UTC>,
    level: LogLevel,
    message_template: String,
    properties: collections::BTreeMap<&'static str, String>
}

impl Event {
    pub fn new(timestamp: DateTime<UTC>, level: LogLevel, message_template: String, properties: collections::BTreeMap<&'static str, String>) -> Event {
        Event {
            timestamp: timestamp,
            level: level,
            message_template: message_template,
            properties: properties
        }
    }
    
    pub fn new_now(level: LogLevel, message_template: String, properties: collections::BTreeMap<&'static str, String>) -> Event {
        Self::new(UTC::now(), level, message_template, properties)
    }
    
    pub fn timestamp(&self) -> DateTime<UTC> {
        self.timestamp
    }
    
    pub fn level(&self) -> LogLevel {
        self.level
    }
    
    pub fn message_template(&self) -> &String {
        &self.message_template
    }
    
    pub fn properties(&self) -> &collections::BTreeMap<&'static str, String> {
        &self.properties
    }
}
