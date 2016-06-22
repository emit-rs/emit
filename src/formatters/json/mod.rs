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
            // TODO, escape '@'
            let name = n;
            try!(write!(to, ",\"{}\":{}", name, v));            
        }
                    
        try!(write!(to, "}}"));

        Ok(())
    }
}
