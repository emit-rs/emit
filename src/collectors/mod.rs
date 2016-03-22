pub mod seq;

use std::error;
use events;

pub trait Collector {
    type Error: error::Error;
    fn dispatch(&self, events: &[events::Event]) -> Result<(),Self::Error>;
}
