use std::collections::BTreeMap;
use chrono::{ UTC, TimeZone };
use events::{ Event, IntoValue };
use templates::MessageTemplate;
use LogLevel;

pub fn some_event() -> Event<'static> {
	let ts = UTC.ymd(2014, 7, 8).and_hms(9, 10, 11);
    let mt = MessageTemplate::new("Hello, {name}. Your data is: {data}");
    let mut props = BTreeMap::new();
    props.insert("name", "Alice".into_value());
    props.insert("data", vec!["a", "b", "c"].into_value());
    Event::new(ts, LogLevel::Info, mt, props)
}