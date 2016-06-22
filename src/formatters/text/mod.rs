use std::io::Write;
use events::Event;
use std::error::Error;
use templates;

pub struct PlainTextFormatter {}

impl PlainTextFormatter {
    pub fn new() -> PlainTextFormatter {
        PlainTextFormatter{}
    }
}

impl super::WriteEvent for PlainTextFormatter {
    fn write_event(&self, event: &Event<'static>, to: &mut Write) -> Result<(), Box<Error>> {
        let repl = templates::repl::MessageTemplateRepl::new(event.message_template().text());
        let content = repl.replace(event.properties());
        try!(writeln!(to, "{} {:5} {}", event.timestamp().format("%FT%TZ"), event.level(), content));
        Ok(())
    }
}
