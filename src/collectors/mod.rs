pub mod seq;
pub mod stdio;

use std::error::Error;
use events::Event;
use pipeline::chain::{Propagate,Emit};

pub trait AcceptEvents {
    fn accept_events(&self, events: &[Event<'static>]) -> Result<(), Box<Error>>;
}

pub struct CollectorElement<T: AcceptEvents + Sync> {
    collector: T
}

impl<T: AcceptEvents + Sync> CollectorElement<T> {
    pub fn new(collector: T) -> CollectorElement<T> {
         CollectorElement {collector: collector}
    }
}

impl<T: AcceptEvents + Sync> Propagate for CollectorElement<T> {
    fn propagate(&self, event: Event<'static>, next: &Emit) {
        let mut batch = vec![event];
        if let Err(e) = self.collector.accept_events(&batch) {
            error!("Could not dispatch events: {}", e);
        }
        next.emit(batch.pop().unwrap());
    }
}
