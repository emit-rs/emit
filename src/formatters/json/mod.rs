use std::io::Write;
use events::Event;
use std::error::Error;
use serde_json;
use LogLevel;

pub struct JsonFormatter {}

impl JsonFormatter {
    pub fn new() -> JsonFormatter {
        JsonFormatter{}
    }
}

impl super::WriteEvent for JsonFormatter {
    fn write_event(&self, event: &Event<'static>, to: &mut Write) -> Result<(), Box<Error>> {
        let template = try!(serde_json::to_string(event.message_template().text()));
        let isots = event.timestamp().format("%FT%T%.3fZ");

        try!(write!(to, "{{\"@t\":\"{}\",\"@mt\":{}", isots, template));

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
    use super::JsonFormatter;
    use test_support;

    #[test]
    fn json_is_produced() {        
        let fmt = JsonFormatter::new();
        let evt = test_support::some_event();
        let mut content = vec![];
        fmt.write_event(&evt, &mut content).is_ok();
    }
}
