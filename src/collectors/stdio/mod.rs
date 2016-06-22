use std::io;
use std::io::Write;
use std::error::Error;
use events;
use formatters;

pub struct StdioCollector {
    use_stderr: bool,
    formatter: Box<formatters::TextFormatter + Sync>
}

unsafe impl Sync for StdioCollector {}

impl StdioCollector {
    pub fn new() -> StdioCollector {
        StdioCollector::new_format(Box::new(formatters::raw::RawFormatter::new()))
    }

    pub fn new_format(formatter: Box<formatters::TextFormatter + Sync>) -> StdioCollector {
        StdioCollector {
            use_stderr: false,
            formatter: formatter
        }
    }

    pub fn new_stderr() -> StdioCollector {
        StdioCollector::new_stderr_format(Box::new(formatters::raw::RawFormatter::new()))
    }

    pub fn new_stderr_format(formatter: Box<formatters::TextFormatter + Sync>) -> StdioCollector {
        StdioCollector {
            use_stderr: true,
            formatter: formatter
        }
    }
}

impl super::Collector for StdioCollector {
    fn dispatch(&self, events: &[events::Event<'static>]) -> Result<(), Box<Error>> {
        let out = io::stdout();
        let err = io::stderr();
        for event in events {
            if self.use_stderr {
                let mut to = &mut err.lock();
                try!(self.formatter.format(&event, to));
                try!(writeln!(to, ""));
            } else {
                let mut to = &mut out.lock();
                try!(self.formatter.format(&event, to));
                try!(writeln!(to, ""));
            }
        }
        
        Ok(())
    }
}
