pub mod seq;
pub mod stdio;

use std::error::Error;
use events::Event;
use pipeline::chain::{PipelineElement, ChainedElement};

pub trait Collector {
    // Could use a signature re-think here
    fn dispatch(&self, events: &[Event]) -> Result<(), Box<Error>>;
}

pub struct CollectorElement<T: Collector + Sync> {
    collector: T
}

impl<T: Collector + Sync> CollectorElement<T> {
    pub fn new(collector: T) -> CollectorElement<T> {
         CollectorElement {collector: collector}
    }
}

impl<T: Collector + Sync> PipelineElement for CollectorElement<T> {
    fn emit(&self, event: Event, next: &ChainedElement) {
        let mut batch = vec![event];
        if let Err(e) = self.collector.dispatch(&batch) {
            error!("Could not dispatch events: {}", e);
        }
        next.emit(batch.pop().unwrap());
    }
}
