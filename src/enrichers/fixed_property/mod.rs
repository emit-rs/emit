use events::{Event,IntoValue,Value};
use pipeline::chain::{Emit,Propagate};

pub struct FixedPropertyEnricher<'a> {
    name: &'a str,
    value: Value
}

impl<'a> FixedPropertyEnricher<'a> {
    pub fn new<T: IntoValue>(name: &'a str, value: T) -> FixedPropertyEnricher<'a> {
        FixedPropertyEnricher { name: name, value: value.into_value() }
    }
}

impl Propagate for FixedPropertyEnricher<'static> {
    fn propagate(&self, event: Event<'static>, next: &Emit) {
        let mut e = event;
        e.add_or_update_property(self.name, self.value.clone());
        next.emit(e);
    }
}
