use std::io::Write;
use std::num::Wrapping;
use events::Event;
use std::error::Error;
use LogLevel;

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
        let isots = event.timestamp().format("%FT%T%.3fZ");

        try!(write!(to, "{{\"@t\":\"{}\",\"@mt\":{}", isots, event.message_template().text()));

        if event.level() != LogLevel::Info {
            try!(write!(to, ",\"@l\":\"{}\"", event.level()));
        }

        for (n,v) in event.properties() {
            let bytes = n.as_bytes();
            if bytes.len() > 0 && bytes[0] == b'@' {
                try!(write!(to, ",\"@{}\":{}", n, v.to_json()));            
            } else {
                try!(write!(to, ",\"{}\":{}", n, v.to_json()));            
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
        let id = jenkins_hash(&event.message_template().text());
        let isots = event.timestamp().format("%FT%T%.3fZ");

        try!(write!(to, "{{\"@t\":\"{}\",\"@m\":{},\"@i\":\"{:08x}\"", isots, &event.message(), id));

        if event.level() != LogLevel::Info {
            try!(write!(to, ",\"@l\":\"{}\"", event.level()));
        }

        for (n,v) in event.properties() {
            let bytes = n.as_bytes();
            if bytes.len() > 0 && bytes[0] == b'@' {
                try!(write!(to, ",\"@{}\":{}", n, v.to_json()));            
            } else {
                try!(write!(to, ",\"{}\":{}", n, v.to_json()));            
            }
        }
                    
        try!(write!(to, "}}"));

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use formatters::WriteEvent;
    use super::{JsonFormatter,RenderedJsonFormatter};
    use test_support;

    #[test]
    fn json_is_produced() {        
        let fmt = JsonFormatter::new();
        let evt = test_support::some_event();
        let mut content = vec![];
        fmt.write_event(&evt, &mut content).is_ok();
    }

    #[test]
    fn rendered_json_is_produced() {        
        let fmt = RenderedJsonFormatter::new();
        let evt = test_support::some_event();
        let mut content = vec![];
        fmt.write_event(&evt, &mut content).is_ok();
    }
}
