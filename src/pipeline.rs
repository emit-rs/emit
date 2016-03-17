use chrono::{DateTime,UTC};
use std::collections::{BTreeMap};
use payloads;
use std::io::Read;
use hyper::Client;
use hyper::header::Connection;
    
pub fn emit(template: &str, properties: &BTreeMap<&'static str, String>) {
    let timestamp: DateTime<UTC> = UTC::now();
    let payload = payloads::format_payload(timestamp, template, properties);

    let events = format!("{{\"Events\":[{}]}}", payload);

    let client = Client::new();
    let mut res = client.post("http://localhost:5341/api/events/raw/")
        .body(&events)
        .header(Connection::close())
        .send().unwrap();


    let mut body = String::new();
    res.read_to_string(&mut body).unwrap();

    println!("Response: {}", body);
}
