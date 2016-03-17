use chrono;
use std::collections::{BTreeMap};
use payloads;
    
pub fn emit(template: &str, properties: &BTreeMap<&'static str, String>) {
    let timestamp: chrono::DateTime<chrono::UTC> = chrono::UTC::now();
    let payload = payloads::format_payload(timestamp, template, properties);
    println!("{}", payload);
}
