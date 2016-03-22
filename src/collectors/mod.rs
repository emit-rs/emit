pub mod seq;

use std::error;

pub trait Collector {
    type Error: error::Error;
    fn dispatch(&self, events: &[String]) -> Result<(),Self::Error>;
}
