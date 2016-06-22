use events;
use events::Event;
use pipeline::chain::{Emit,Propagate};
use serde;

pub struct FixedPropertyEnricher<'a> {
    name: &'a str,
    value: String
}

impl<'a> FixedPropertyEnricher<'a> {
    pub fn new<T: serde::ser::Serialize>(name: &'a str, value: &T) -> FixedPropertyEnricher<'a> {
        FixedPropertyEnricher { name: name, value: events::capture_property_value(value) }
    }
}

impl Propagate for FixedPropertyEnricher<'static> {
    fn propagate(&self, event: Event<'static>, next: &Emit) {
        let mut e = event;
        e.add_or_update_property(self.name, self.value.clone());
        next.emit(e);
    }
}
