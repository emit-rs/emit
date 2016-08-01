use std::io::Write;
use events::{ Event, TextValueFormatter };
use std::error::Error;

pub struct RawFormatter {}

impl RawFormatter {
    pub fn new() -> RawFormatter {
        RawFormatter{}
    }
}

impl super::WriteEvent for RawFormatter {
    fn write_event(&self, event: &Event<'static>, to: &mut Write) -> Result<(), Box<Error>> {
        let formatter = TextValueFormatter::value_formatter();
        try!(writeln!(to, "emit {} {:5} {}", event.timestamp().format("%FT%T%.9fZ"), event.level(), event.message_template().text()));
        for (n,v) in event.properties() {                
            try!(writeln!(to, "  {}: {}", n, formatter.format(v)));
        }

        Ok(())
    }
}

