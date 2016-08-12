use std::borrow::Cow;
use std::marker::PhantomData;
use std::io::Write;
use std::num::Wrapping;
use events::{Event, Value};
use super::{ValueFormatter, ValueFormatterVisitor, format_vec};
use std::error::Error;
use LogLevel;

#[derive(Default)]
#[doc(hidden)]
pub struct JsonValueFormatter<'a> {
    _marker: PhantomData<&'a ()>
}
impl <'a> JsonValueFormatter<'a> {
    pub fn value_formatter() -> ValueFormatter<'a, Self> {
        ValueFormatter::default()
    }
}

impl <'a> ValueFormatterVisitor<'a> for JsonValueFormatter<'a> {
    fn visit_null(_: &ValueFormatter<'a, Self>) -> Cow<'a, str> {
        Cow::Borrowed("null")
    }

    fn visit_bool(_: &ValueFormatter<'a, Self>, v: &'a bool) -> Cow<'a, str> {
        if *v {
            Cow::Borrowed("true")
        }
        else {
            Cow::Borrowed("false")
        }
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
        let bytes = v.as_bytes();
        if v.len() > 0 && bytes[0] == b'"' {
            Cow::Borrowed(v)
        }
        else {
            let mut quoted = String::with_capacity(v.len() + 2);
            quoted.push('"');
            quoted.push_str(v);
            quoted.push('"');

            Cow::Owned(quoted)
        }
    }

    fn visit_vec(formatter: &ValueFormatter<'a, Self>, v: &'a Vec<Value>) -> Cow<'a, str> {
        format_vec(formatter, v, false)
    }
}

/// Translate events into a compact JSON format. A message template and
/// associated properties are recorded. To include the rendered message
/// and computed event type instead, see `RenderedJsonFormatter`.
pub struct JsonFormatter {}

impl JsonFormatter {
    pub fn new() -> JsonFormatter {
        JsonFormatter{}
    }
}

impl super::WriteEvent for JsonFormatter {
    fn write_event(&self, event: &Event<'static>, to: &mut Write) -> Result<(), Box<Error>> {
        let formatter = JsonValueFormatter::value_formatter();

        let isots = event.timestamp().format("%FT%T%.3fZ");

        try!(write!(to, "{{\"@t\":{},\"@mt\":{}", 
            JsonValueFormatter::visit_str(&formatter, &isots.to_string()), 
            JsonValueFormatter::visit_str(&formatter, event.message_template().text())
        ));

        if event.level() != LogLevel::Info {
            try!(write!(to, ",\"@l\":{}", 
                JsonValueFormatter::visit_str(&formatter, event.level().as_ref()))
            );
        }

        for (n,v) in event.properties() {
            let bytes = n.as_bytes();
            if bytes.len() > 0 && bytes[0] == b'@' {
                try!(write!(to, ",\"@{}\":{}", n, formatter.format(v)));
            } else {
                try!(write!(to, ",\"{}\":{}", n, formatter.format(v)));            
            }
        }
                    
        try!(write!(to, "}}"));

        Ok(())
    }
}

/// Translate events into a compact JSON format. The message is rendered
/// into text and a 32-bit _event type_ is computed from the original
/// message template. To record the template itself instead, see `JsonFormatter`.
pub struct RenderedJsonFormatter {}

impl RenderedJsonFormatter {
    pub fn new() -> RenderedJsonFormatter {
        RenderedJsonFormatter{}
    }
}

fn jenkins_hash(text: &str) -> u32 {
    let mut hash = Wrapping(0u32);
    for ch in text.chars() {
        hash += Wrapping(ch as u32);
        hash += hash << 10;
        hash ^= hash >> 6;
    }
    hash += hash << 3;
    hash ^= hash >> 11;
    hash += hash << 15;
    hash.0
}

impl super::WriteEvent for RenderedJsonFormatter {
    fn write_event(&self, event: &Event<'static>, to: &mut Write) -> Result<(), Box<Error>> {
        let formatter = JsonValueFormatter::value_formatter();

        let id = jenkins_hash(&event.message_template().text()) as u64;
        let isots = event.timestamp().format("%FT%T%.3fZ");

        try!(write!(to, "{{\"@t\":{},\"@m\":{},\"@i\":\"{:08x}\"", 
            JsonValueFormatter::visit_str(&formatter, &isots.to_string()), 
            JsonValueFormatter::visit_str(&formatter, &event.message()), 
            id
        ));

        if event.level() != LogLevel::Info {
            try!(write!(to, ",\"@l\":{}", 
                JsonValueFormatter::visit_str(&formatter, event.level().as_ref()))
            );
        }

        for (n,v) in event.properties() {
            let bytes = n.as_bytes();
            if bytes.len() > 0 && bytes[0] == b'@' {
                try!(write!(to, ",\"@{}\":{}", n, formatter.format(v)));
            } else {
                try!(write!(to, ",\"{}\":{}", n, formatter.format(v)));            
            }
        }
                    
        try!(write!(to, "}}"));

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::str;
    use formatters::WriteEvent;
    use super::{JsonFormatter,RenderedJsonFormatter};
    use test_support;

    #[test]
    fn json_is_produced() {        
        let fmt = JsonFormatter::new();
        let evt = test_support::some_event();
        let mut content = vec![];
        fmt.write_event(&evt, &mut content).is_ok();

        assert_eq!(str::from_utf8(&content).unwrap(), "{\"@t\":\"2014-07-08T09:10:11.000Z\",\"@mt\":\"Hello, {name}. Your data is: {data}\",\"data\":[\"a\",\"b\",\"c\"],\"name\":\"Alice\"}");
    }

    #[test]
    fn rendered_json_is_produced() {        
        let fmt = RenderedJsonFormatter::new();
        let evt = test_support::some_event();
        let mut content = vec![];
        fmt.write_event(&evt, &mut content).is_ok();

        assert_eq!(str::from_utf8(&content).unwrap(), "{\"@t\":\"2014-07-08T09:10:11.000Z\",\"@m\":\"Hello, Alice. Your data is: [a, b, c]\",\"@i\":\"90e7bed3\",\"data\":[\"a\",\"b\",\"c\"],\"name\":\"Alice\"}");
    }
}
