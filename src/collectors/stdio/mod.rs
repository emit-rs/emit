use std::io;
use std::io::Write;
use events;

pub struct StdioCollector {
    _use_stderr: bool
}

impl StdioCollector {
    pub fn new() -> StdioCollector {
        StdioCollector {
            _use_stderr: false
        }
    }
}

impl super::Collector for StdioCollector {
    type Error = io::Error;
    
    fn dispatch(&self, events: &[events::Event]) -> Result<(), Self::Error> {
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
