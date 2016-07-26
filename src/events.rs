use chrono::{DateTime,UTC};
use std::collections;
use std::collections::btree_map::Entry;
use LogLevel;
use templates;
use std::fmt;
use std::convert::Into;

#[derive(Clone, PartialEq)]
pub enum Value {
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

impl Value {
    // JSON serialization belongs elsewhere, but keeps
    // the distinction between regular string and JSON data clear.
    pub fn to_json<'a>(&'a self) -> String {
        match *self {
            Value::Bool(ref b) => b.to_string(),
            Value::I64(ref n) => n.to_string(),
            Value::U64(ref n) => n.to_string(),
            Value::F64(ref n) => n.to_string(),
            Value::String(ref s) => {
                let mut quoted = String::with_capacity(s.len() + 2);
                quoted.push('"');
                quoted.push_str(s);
                quoted.push('"');

                quoted
            },
            Value::Vec(ref v) => {
                let mut len = 0;
                let mut results = Vec::with_capacity(v.len());

                for val in v {
                    let res = val.to_json();
                    len += res.len();
                    results.push(res);
                }

                let mut json = String::with_capacity(
                    len + //item data
                    2 + //brackets
                    results.len() - 1 //commas
                );

                let mut first = true;
                json.push('[');
                for res in results {
                    if !first {
                        json.push(',');
                    }
                    else {
                        first = false;
                    }
                    json.push_str(res.as_ref());
                }
                json.push(']');

                json
            }
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
            Value::String(ref s) => write!(f, "String(\"{}\")", s),
            Value::Vec(_) => write!(f, "Vec()")
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
            Value::String(ref s) => write!(f, "{}", s),
            Value::Vec(_) => write!(f, "vec[]")
        }
    }
}

trait IntoValue {
    fn into_value(self) -> Value;
}

impl IntoValue for bool {
    fn into_value(self) -> Value {
        Value::Bool(self)
    }
}

impl IntoValue for i64 {
    fn into_value(self) -> Value {
        Value::I64(self)
    }
}

impl IntoValue for i8 {
    fn into_value(self) -> Value {
        Value::I64(self.into())
    }
}

impl IntoValue for i16 {
    fn into_value(self) -> Value {
        Value::I64(self.into())
    }
}

impl IntoValue for i32 {
    fn into_value(self) -> Value {
        Value::I64(self.into())
    }
}

impl IntoValue for u64 {
    fn into_value(self) -> Value {
        Value::U64(self)
    }
}

impl IntoValue for u8 {
    fn into_value(self) -> Value {
        Value::U64(self.into())
    }
}

impl IntoValue for u16 {
    fn into_value(self) -> Value {
        Value::U64(self.into())
    }
}

impl IntoValue for u32 {
    fn into_value(self) -> Value {
        Value::U64(self.into())
    }
}

impl IntoValue for f64 {
    fn into_value(self) -> Value {
        Value::F64(self)
    }
}

impl IntoValue for f32 {
    fn into_value(self) -> Value {
        Value::F64(self.into())
    }
}


impl IntoValue for String {
    fn into_value(self) -> Value {
        Value::String(self)
    }
}

impl<'a> IntoValue for &'a str {
    fn into_value(self) -> Value {
        Value::String(self.into())
    }
}

impl <V: IntoValue> IntoValue for Vec<V> {
    fn into_value(self) -> Value {
        Value::Vec(self.into_iter().map(|v| v.into_value()).collect())
    }
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
