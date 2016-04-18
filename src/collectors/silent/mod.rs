use events;
use std::error;
use std::fmt;

#[derive(Debug)]
pub struct NoError { }

impl fmt::Display for NoError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "No Error")
    }
}

impl error::Error for NoError {
    fn description(&self) -> &str {
        "No Error"
    }
}

impl SilentCollector {
    pub fn new() -> SilentCollector {
        SilentCollector { }
    }
}

pub struct SilentCollector { }

impl super::Collector for SilentCollector {
    type Error = NoError;
    
    #[allow(unused_variables)]
    fn dispatch(&self, events: &[events::Event]) -> Result<(), Self::Error> {
        Ok(())
    }
}
