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
    #[allow(unused_must_use)]
    fn propagate(&self, event: Event<'static>, next: &Emit) {
        let mut batch = vec![event];
        // TODO - self-log
        self.collector.accept_events(&batch);
        next.emit(batch.pop().unwrap());
    }
}
