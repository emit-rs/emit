use events;
use elements;
use serde;

pub struct FixedPropertyEnricher {
    name: &'static str,
    value: String
}

impl FixedPropertyEnricher {
    pub fn new<T: serde::ser::Serialize>(name: &'static str, value: &T) -> FixedPropertyEnricher {
        FixedPropertyEnricher { name: name, value: events::capture_property_value(value) }
    }
}

impl elements::PipelineElement for FixedPropertyEnricher {
    fn emit(&self, event: events::Event, next: &elements::ChainedElement) {
        let mut e = event;
        e.add_or_update_property(self.name, self.value.clone());
        next.emit(e);
    }
}
