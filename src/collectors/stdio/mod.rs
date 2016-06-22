use std::io;
use std::io::Write;
use std::error::Error;
use events;

pub struct StdioCollector {
    _use_stderr: bool
}

unsafe impl Sync for StdioCollector {}

impl StdioCollector {
    pub fn new() -> StdioCollector {
        StdioCollector {
            _use_stderr: false
        }
    }
}

impl super::Collector for StdioCollector {
    fn dispatch(&self, events: &[events::Event<'static>]) -> Result<(), Box<Error>> {
        let out = io::stdout();
        let mut handle = out.lock();
        for event in events {
            try!(writeln!(handle, "emit {} {:5} {}", event.timestamp().format("%FT%TZ"), event.level(), event.message_template().text()));
            for (n,v) in event.properties() {                
                try!(writeln!(handle, "  {}: {}", n, v));            
            }
        }
        
        Ok(())
    }
}
