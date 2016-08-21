use std::marker::PhantomData;
use std::borrow::Cow;
use std::fmt;
use std::error::Error;
use events::{Event,Value,IntoValue};
use std::io::Write;

/// Implementers can write a representation of an event to a binary stream.
pub trait WriteEvent {
    fn write_event(&self, event: &Event<'static>, to: &mut Write) -> Result<(), Box<Error>>;
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let formatter = DebugValueFormatter::value_formatter();
        write!(f, "{}", formatter.format(&self))
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let formatter = TextValueFormatter::value_formatter();
        write!(f, "{}", formatter.format(&self))
    }
}

#[doc(hidden)]
pub trait ValueFormatterVisitor<'a> where Self: Sized {
    fn visit_null(formatter: &ValueFormatter<'a, Self>) -> Cow<'a, str>;
    fn visit_bool(formatter: &ValueFormatter<'a, Self>, v: &'a bool) -> Cow<'a, str>;
    fn visit_i64(formatter: &ValueFormatter<'a, Self>, v: &'a i64) -> Cow<'a, str>;
    fn visit_u64(formatter: &ValueFormatter<'a, Self>, v: &'a u64) -> Cow<'a, str>;
    fn visit_f64(formatter: &ValueFormatter<'a, Self>, v: &'a f64) -> Cow<'a, str>;
    fn visit_str(formatter: &ValueFormatter<'a, Self>, v: &'a str) -> Cow<'a, str>;
    fn visit_vec(formatter: &ValueFormatter<'a, Self>, v: &'a Vec<Value>) -> Cow<'a, str>;
}

#[derive(Default)]
#[doc(hidden)]
pub struct ValueFormatter<'a, S: ValueFormatterVisitor<'a>> {
    _marker1: PhantomData<&'a ()>,
    _marker2: PhantomData<S>
}

impl <'a, S: ValueFormatterVisitor<'a>> ValueFormatter<'a, S> {
    pub fn format(&self, v: &'a Value) -> Cow<'a, str> {
        match *v {
            Value::Null => S::visit_null(&self),
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
struct DebugValueFormatter<'a> {
    _marker: PhantomData<&'a ()>
}
impl <'a> DebugValueFormatter<'a> {
    fn value_formatter() -> ValueFormatter<'a, Self> {
        ValueFormatter::default()
    }
}

impl <'a> ValueFormatterVisitor<'a> for DebugValueFormatter<'a> {
    fn visit_null(_: &ValueFormatter<'a, Self>) -> Cow<'a, str> {
        Cow::Borrowed("Null")
    }

    fn visit_bool(_: &ValueFormatter<'a, Self>, v: &'a bool) -> Cow<'a, str> {
        Cow::Owned(format!("Bool({})", v.to_string()))
    }

    fn visit_i64(_: &ValueFormatter<'a, Self>, v: &'a i64) -> Cow<'a, str> {
        Cow::Owned(format!("I64({})",v.to_string()))
    }

    fn visit_u64(_: &ValueFormatter<'a, Self>, v: &'a u64) -> Cow<'a, str> {
        Cow::Owned(format!("U64({})",v.to_string()))
    }

    fn visit_f64(_: &ValueFormatter<'a, Self>, v: &'a f64) -> Cow<'a, str> {
        Cow::Owned(format!("F64({})",v.to_string()))
    }

    fn visit_str(_: &ValueFormatter<'a, Self>, v: &'a str) -> Cow<'a, str> {
        Cow::Owned(format!("Str({})",v))
    }

    fn visit_vec(formatter: &ValueFormatter<'a, Self>, v: &'a Vec<Value>) -> Cow<'a, str> {
        Cow::Owned(format!("Vec({})", format_vec(formatter, v, true)))
    }
}

#[derive(Default)]
#[doc(hidden)]
pub struct TextValueFormatter<'a> {
    _marker: PhantomData<&'a ()>
}
impl <'a> TextValueFormatter<'a> {
    pub fn value_formatter() -> ValueFormatter<'a, Self> {
        ValueFormatter::default()
    }
}

impl <'a> ValueFormatterVisitor<'a> for TextValueFormatter<'a> {
    fn visit_null(_: &ValueFormatter<'a, Self>) -> Cow<'a, str> {
        Cow::Borrowed("null")
    }

    fn visit_bool(_: &ValueFormatter<'a, Self>, v: &'a bool) -> Cow<'a, str> {
        Cow::Owned(v.to_string())
    }

    fn visit_i64(_: &ValueFormatter<'a, Self>, v: &'a i64) -> Cow<'a, str> {
        Cow::Owned(v.to_string())
    }

    fn visit_u64(_: &ValueFormatter<'a, Self>, v: &'a u64) -> Cow<'a, str> {
        Cow::Owned(v.to_string())
    }

    fn visit_f64(_: &ValueFormatter<'a, Self>, v: &'a f64) -> Cow<'a, str> {
        Cow::Owned(v.to_string())
    }

    fn visit_str(_: &ValueFormatter<'a, Self>, v: &'a str) -> Cow<'a, str> {
        Cow::Borrowed(v)
    }

    fn visit_vec(formatter: &ValueFormatter<'a, Self>, v: &'a Vec<Value>) -> Cow<'a, str> {
        format_vec(formatter, v, true)
    }
}

#[doc(hidden)]
pub fn format_vec<'a, S: ValueFormatterVisitor<'a>>(formatter: &ValueFormatter<'a, S>, v: &'a Vec<Value>, whitespace: bool) -> Cow<'a, str> {
    if v.len() == 0 {
        return Cow::Borrowed("[]");
    }

    let mut len = 0;
    let mut results = Vec::with_capacity(v.len());

    for val in v {
        let res = formatter.format(val);
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

    Cow::Owned(json)
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

impl <V: IntoValue> IntoValue for Option<V> {
    fn into_value(self) -> Value {
        match self {
            Some(v) => v.into_value(),
            None => Value::Null
        }
    }
}

impl IntoValue for Value {
    fn into_value(self) -> Value {
        self
    }
}

#[cfg(test)]
mod tests {
    use std::str;
    use ::events::{Value,IntoValue};
    use super::{ TextValueFormatter,format_vec};

    #[test]
    fn format_vec_empty() {
        let formatter = TextValueFormatter::value_formatter();
        let v = Vec::<Value>::new();

        let fmtd = format_vec(&formatter, &v, true);

        assert_eq!("[]", fmtd);
    }

    #[test]
    fn format_vec_single() {
        let formatter = TextValueFormatter::value_formatter();
        let v = vec![
            "a".into_value()
        ];

        let fmtd = format_vec(&formatter, &v, true);

        assert_eq!("[a]", fmtd);
    }

    #[test]
    fn format_vec_many() {
        let formatter = TextValueFormatter::value_formatter();
        let v = vec![
            "a".into_value(),
            "b".into_value(),
            "c".into_value()
        ];

        let fmtd = format_vec(&formatter, &v, true);

        assert_eq!("[a, b, c]", fmtd);
    }
}