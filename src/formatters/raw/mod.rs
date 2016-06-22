use std::io::Write;
use events::Event;
use std::error::Error;

pub struct RawFormatter {}

impl RawFormatter {
    pub fn new() -> RawFormatter {
        RawFormatter{}
    }
}

impl super::TextFormatter for RawFormatter {
    fn format(&self, event: &Event<'static>, to: &mut Write) -> Result<(), Box<Error>> {
        try!(writeln!(to, "emit {} {:5} {}", event.timestamp().format("%FT%TZ"), event.level(), event.message_template().text()));
        for (n,v) in event.properties() {                
            try!(writeln!(to, "  {}: {}", n, v));            
        }

        Ok(())
    }
}

