use chrono::{DateTime,UTC};
use std::marker::PhantomData;
use std::borrow::Cow;
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

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let sanitiser = DebugSanitiser::sanitiser();
        write!(f, "{}", sanitiser.sanitise(&self))
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let sanitiser = TextSanitiser::sanitiser();
        write!(f, "{}", sanitiser.sanitise(&self))
    }
}

pub trait SanitiserVisitor<'a> where Self: Sized {
    fn visit_bool(sanitiser: &Sanitiser<'a, Self>, v: &'a bool) -> Cow<'a, str>;
    fn visit_i64(sanitiser: &Sanitiser<'a, Self>, v: &'a i64) -> Cow<'a, str>;
    fn visit_u64(sanitiser: &Sanitiser<'a, Self>, v: &'a u64) -> Cow<'a, str>;
    fn visit_f64(sanitiser: &Sanitiser<'a, Self>, v: &'a f64) -> Cow<'a, str>;
    fn visit_str(sanitiser: &Sanitiser<'a, Self>, v: &'a str) -> Cow<'a, str>;
    fn visit_vec(sanitiser: &Sanitiser<'a, Self>, v: &'a Vec<Value>) -> Cow<'a, str>;
}

#[derive(Default)]
pub struct Sanitiser<'a, S: SanitiserVisitor<'a>> {
    _marker1: PhantomData<&'a ()>,
    _marker2: PhantomData<S>
}

impl <'a, S: SanitiserVisitor<'a>> Sanitiser<'a, S> {
    pub fn sanitise(&self, v: &'a Value) -> Cow<'a, str> {
        match *v {
            Value::Bool(ref v) => S::visit_bool(&self, v),
            Value::I64(ref v) => S::visit_i64(&self, v),
            Value::U64(ref v) => S::visit_u64(&self, v),
            Value::F64(ref v) => S::visit_f64(&self, v),
            Value::String(ref v) => S::visit_str(&self, v),
            Value::Vec(ref v) => S::visit_vec(&self, v)
        }
    }
}

#[derive(Default)]
struct DebugSanitiser<'a> {
    _marker: PhantomData<&'a ()>
}
impl <'a> DebugSanitiser<'a> {
    fn sanitiser() -> Sanitiser<'a, Self> {
        Sanitiser::default()
    }
}

impl <'a> SanitiserVisitor<'a> for DebugSanitiser<'a> {
    fn visit_bool(_: &Sanitiser<'a, Self>, v: &'a bool) -> Cow<'a, str> {
        Cow::Owned(format!("Bool({})", v.to_string()))
    }

    fn visit_i64(_: &Sanitiser<'a, Self>, v: &'a i64) -> Cow<'a, str> {
        Cow::Owned(format!("I64({})",v.to_string()))
    }

    fn visit_u64(_: &Sanitiser<'a, Self>, v: &'a u64) -> Cow<'a, str> {
        Cow::Owned(format!("U64({})",v.to_string()))
    }

    fn visit_f64(_: &Sanitiser<'a, Self>, v: &'a f64) -> Cow<'a, str> {
        Cow::Owned(format!("F64({})",v.to_string()))
    }

    fn visit_str(_: &Sanitiser<'a, Self>, v: &'a str) -> Cow<'a, str> {
        Cow::Owned(format!("Str({})",v))
    }

    fn visit_vec(sanitiser: &Sanitiser<'a, Self>, v: &'a Vec<Value>) -> Cow<'a, str> {
        Cow::Owned(format!("Vec({})", sanitise_vec(sanitiser, v, true)))
    }
}

#[derive(Default)]
pub struct TextSanitiser<'a> {
    _marker: PhantomData<&'a ()>
}
impl <'a> TextSanitiser<'a> {
    pub fn sanitiser() -> Sanitiser<'a, Self> {
        Sanitiser::default()
    }
}

impl <'a> SanitiserVisitor<'a> for TextSanitiser<'a> {
    fn visit_bool(_: &Sanitiser<'a, Self>, v: &'a bool) -> Cow<'a, str> {
        Cow::Owned(v.to_string())
    }

    fn visit_i64(_: &Sanitiser<'a, Self>, v: &'a i64) -> Cow<'a, str> {
        Cow::Owned(v.to_string())
    }

    fn visit_u64(_: &Sanitiser<'a, Self>, v: &'a u64) -> Cow<'a, str> {
        Cow::Owned(v.to_string())
    }

    fn visit_f64(_: &Sanitiser<'a, Self>, v: &'a f64) -> Cow<'a, str> {
        Cow::Owned(v.to_string())
    }

    fn visit_str(_: &Sanitiser<'a, Self>, v: &'a str) -> Cow<'a, str> {
        Cow::Borrowed(v)
    }

    fn visit_vec(sanitiser: &Sanitiser<'a, Self>, v: &'a Vec<Value>) -> Cow<'a, str> {
        Cow::Owned(sanitise_vec(sanitiser, v, true))
    }
}

pub fn sanitise_vec<'a, S: SanitiserVisitor<'a>>(sanitiser: &Sanitiser<'a, S>, v: &'a Vec<Value>, whitespace: bool) -> String {
    let mut len = 0;
    let mut results = Vec::with_capacity(v.len());

    for val in v {
        let res = sanitiser.sanitise(val);
        len += res.len();
        results.push(res);
    }

    let (open, close, comma) = {
        if whitespace {
            ("[", "]", ", ")
        }
        else {
            ("[", "]", ",")
        }
    };

    let mut json = String::with_capacity(
        len + //item data
        (2 * open.len()) + //brackets
        (results.len() - 1) * comma.len() //commas
    );

    let mut first = true;
    json.push_str(open);
    for res in results {
        if !first {
            json.push_str(comma);
        }
        else {
            first = false;
        }
        json.push_str(res.as_ref());
    }
    json.push_str(close);

    json
}

pub trait IntoValue {
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
