use std::io::Write;
use events::Event;
use std::error::Error;
use log;
use serde_json;

pub struct JsonFormatter {}

impl JsonFormatter {
    pub fn new() -> JsonFormatter {
        JsonFormatter{}
    }
}

impl super::TextFormatter for JsonFormatter {
    fn format(&self, event: &Event<'static>, to: &mut Write) -> Result<(), Box<Error>> {
        let template = try!(serde_json::to_string(event.message_template().text()));
        let isots = event.timestamp().format("%FT%TZ");

        try!(write!(to, "{{\"@t\":\"{}\",\"@mt\":{}", isots, template));

        if event.level() != log::LogLevel::Info {
            try!(write!(to, ",\"@l\":\"{}\"", event.level()));
        }

        for (n,v) in event.properties() {
            let bytes = n.as_bytes();
            if bytes.len() > 0 && bytes[0] == b'@' {
                try!(write!(to, ",\"@{}\":{}", n, v));            
            } else {
                try!(write!(to, ",\"{}\":{}", n, v));            
            }
        }
                    
        try!(write!(to, "}}"));

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use formatters::TextFormatter;
    use super::JsonFormatter;
    use test_support;

    #[test]
    fn json_is_produced() {        
        let fmt = JsonFormatter::new();
        let evt = test_support::some_event();
        let mut content = vec![];
        fmt.format(&evt, &mut content).is_ok();
    }
}
