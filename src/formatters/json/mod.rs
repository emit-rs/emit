use std::borrow::Cow;
use std::marker::PhantomData;
use std::io::Write;
use std::num::Wrapping;
use events::{ Event, Value, Sanitiser, SanitiserVisitor, sanitise_vec };
use std::error::Error;
use LogLevel;

#[derive(Default)]
pub struct JsonSanitiser<'a> {
    _marker: PhantomData<&'a ()>
}
impl <'a> JsonSanitiser<'a> {
    pub fn sanitiser() -> Sanitiser<'a, Self> {
        Sanitiser::default()
    }
}

impl <'a> SanitiserVisitor<'a> for JsonSanitiser<'a> {
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

    fn visit_vec(sanitiser: &Sanitiser<'a, Self>, v: &'a Vec<Value>) -> Cow<'a, str> {
        Cow::Owned(sanitise_vec(sanitiser, v, false))
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
        let sanitiser = JsonSanitiser::sanitiser();

        let isots = event.timestamp().format("%FT%T%.3fZ");

        try!(write!(to, "{{\"@t\":{},\"@mt\":{}", 
            JsonSanitiser::visit_str(&sanitiser, &isots.to_string()), 
            JsonSanitiser::visit_str(&sanitiser, event.message_template().text())
        ));

        if event.level() != LogLevel::Info {
            try!(write!(to, ",\"@l\":{}", 
                JsonSanitiser::visit_str(&sanitiser, event.level().as_ref()))
            );
        }

        for (n,v) in event.properties() {
            let bytes = n.as_bytes();
            if bytes.len() > 0 && bytes[0] == b'@' {
                try!(write!(to, ",\"@{}\":{}", n, sanitiser.sanitise(v)));
            } else {
                try!(write!(to, ",\"{}\":{}", n, sanitiser.sanitise(v)));            
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
        let sanitiser = JsonSanitiser::sanitiser();

        let id = jenkins_hash(&event.message_template().text()) as u64;
        let isots = event.timestamp().format("%FT%T%.3fZ");

        try!(write!(to, "{{\"@t\":{},\"@m\":{},\"@i\":\"{:08x}\"", 
            JsonSanitiser::visit_str(&sanitiser, &isots.to_string()), 
            JsonSanitiser::visit_str(&sanitiser, &event.message()), 
            id
        ));

        if event.level() != LogLevel::Info {
            try!(write!(to, ",\"@l\":{}", 
                JsonSanitiser::visit_str(&sanitiser, event.level().as_ref()))
            );
        }

        for (n,v) in event.properties() {
            let bytes = n.as_bytes();
            if bytes.len() > 0 && bytes[0] == b'@' {
                try!(write!(to, ",\"@{}\":{}", n, sanitiser.sanitise(v)));
            } else {
                try!(write!(to, ",\"{}\":{}", n, sanitiser.sanitise(v)));            
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
