use events;
use events::Event;
use pipeline::chain::{ChainedElement,PipelineElement};
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

impl PipelineElement for FixedPropertyEnricher<'static> {
    fn emit(&self, event: Event<'static>, next: &ChainedElement) {
        let mut e = event;
        e.add_or_update_property(self.name, self.value.clone());
        next.emit(e);
    }
}
