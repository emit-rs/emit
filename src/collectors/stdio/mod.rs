use std::io;
use std::io::Write;
use std::error::Error;
use events;
use formatters;

pub struct StdioCollector<F> {
    use_stderr: bool,
    formatter: F
}

unsafe impl<F: Sync> Sync for StdioCollector<F> {}

impl<F> StdioCollector<F> {
    pub fn new(formatter: F) -> StdioCollector<F> {
        StdioCollector {
            use_stderr: false,
            formatter: formatter
        }
    }

    pub fn new_stderr(formatter: F) -> StdioCollector<F> {
        StdioCollector {
            use_stderr: true,
            formatter: formatter
        }
    }
}

impl<F: formatters::WriteEvent + Sync> super::AcceptEvents for StdioCollector<F> {
    fn accept_events(&self, events: &[events::Event<'static>]) -> Result<(), Box<Error>> {
        let out = io::stdout();
        let err = io::stderr();
        for event in events {
            if self.use_stderr {
                let mut to = &mut err.lock();
                try!(self.formatter.write_event(&event, to));
                try!(writeln!(to, ""));
            } else {
                let mut to = &mut out.lock();
                try!(self.formatter.write_event(&event, to));
                try!(writeln!(to, ""));
            }
        }
        
        Ok(())
    }
}
