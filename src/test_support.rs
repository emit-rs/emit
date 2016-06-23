use std::collections::BTreeMap;
use events::Event;
use templates::MessageTemplate;
use log::LogLevel;

pub fn some_event() -> Event<'static> {
    let mt = MessageTemplate::new("Hello, {name}");
    let mut props = BTreeMap::new();
    props.insert("name", "Alice".into());
    Event::new_now(LogLevel::Info, mt, props)
}
