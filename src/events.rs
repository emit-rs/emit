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
    /// Represents a JSON Boolean
    Bool(bool),

    /// Represents a JSON signed integer
    I64(i64),

    /// Represents a JSON unsigned integer
    U64(u64),

    /// Represents a JSON floating point number
    F64(f64),

    /// Represents a JSON string
    String(String),
}

impl Value {
    // JSON serialization belongs elsewhere, but keeps
    // the distinction between regular string and JSON data clear.
    pub fn to_json<'a>(&'a self) -> String {
        match *self {
            Value::Bool(ref b) => b.to_string(),
            Value::I64(ref n) => n.to_string(),
            Value::U64(ref n) => n.to_string(),
            Value::F64(ref n) => n.to_string(),
            Value::String(ref s) => s.clone()
        }
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Value::Bool(ref b) => write!(f, "Bool({})", b),
            Value::I64(ref n) => write!(f, "I64({})", n),
            Value::U64(ref n) => write!(f, "U64({})", n),
            Value::F64(ref n) => write!(f, "F64({})", n),
            Value::String(ref s) => write!(f, "String({})", s)
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match *self {
            Value::Bool(ref b) => write!(f, "{}", b),
            Value::I64(ref n) => write!(f, "{}", n),
            Value::U64(ref n) => write!(f, "{}", n),
            Value::F64(ref n) => write!(f, "{}", n),
            //TODO: Determine if this is needed
            Value::String(ref s) => {
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

impl Into<Value> for bool {
    fn into(self) -> Value {
        Value::Bool(self)
    }
}

impl Into<Value> for i64 {
    fn into(self) -> Value {
        Value::I64(self)
    }
}

impl Into<Value> for i8 {
    fn into(self) -> Value {
        Value::I64(self.into())
    }
}

impl Into<Value> for i16 {
    fn into(self) -> Value {
        Value::I64(self.into())
    }
}

impl Into<Value> for i32 {
    fn into(self) -> Value {
        Value::I64(self.into())
    }
}

impl Into<Value> for u64 {
    fn into(self) -> Value {
        Value::U64(self)
    }
}

impl Into<Value> for u8 {
    fn into(self) -> Value {
        Value::U64(self.into())
    }
}

impl Into<Value> for u16 {
    fn into(self) -> Value {
        Value::U64(self.into())
    }
}

impl Into<Value> for u32 {
    fn into(self) -> Value {
        Value::U64(self.into())
    }
}

impl Into<Value> for f64 {
    fn into(self) -> Value {
        Value::F64(self)
    }
}

impl Into<Value> for f32 {
    fn into(self) -> Value {
        Value::F64(self.into())
    }
}


impl Into<Value> for String {
    fn into(self) -> Value {
        string_to_value(&self)
    }
}

impl<'a> Into<Value> for &'a str {
    fn into(self) -> Value {
       string_to_value(self)
    }
}

//TODO: Determine if this is needed
fn string_to_value(string: &str) -> Value {
    let bytes = string.as_bytes();
    if bytes.len() > 1 && bytes[0] == b'"' {
        Value::String((&string[1..string.len() - 1]).into())
    } else {
        Value::String(string.into())
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
