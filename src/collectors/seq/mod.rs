use hyper;
use hyper::header::Connection;
use std::io::Read;
use std::fmt::Write;
use serde_json;
use events;
use log;

pub const DEFAULT_EVENT_BODY_LIMIT_BYTES: usize = 1024 * 256;
pub const DEFAULT_BATCH_LIMIT_BYTES: usize = 1024 * 1024 * 10;
pub const LOCAL_SERVER_URL: &'static str = "http://localhost:5341/";

// 0 is "OFF", but fatal is the best effort for rendering this if we ever get an
// event with that level.
static SEQ_LEVEL_NAMES: [&'static str; 6] = ["Fatal", "Error", "Warning", "Information", "Debug", "Verbose"];

pub struct SeqCollector {
    server_url: String, 
    api_key: Option<String>, 
    event_body_limit_bytes: usize, 
    batch_limit_bytes: usize
}

impl SeqCollector {
    pub fn new<'b>(server_url: &'b str, api_key: Option<&'b str>, event_body_limit_bytes: usize, batch_limit_bytes: usize) -> SeqCollector {
        SeqCollector {
            server_url: server_url.to_owned(),
            api_key: api_key.map(|k| k.to_owned()),
            event_body_limit_bytes: event_body_limit_bytes,
            batch_limit_bytes: batch_limit_bytes
        }
    }
    
    pub fn new_local() -> SeqCollector {
        Self::new(LOCAL_SERVER_URL, None, DEFAULT_EVENT_BODY_LIMIT_BYTES, DEFAULT_BATCH_LIMIT_BYTES)
    }
}

impl super::Collector for SeqCollector {
    type Error = hyper::Error;
    
    fn dispatch(&self, events: &[events::Event]) -> Result<(), Self::Error> {     
        for event in events {
            let payload = format_payload(event);
            let el = format!("{{\"Events\":[{}]}}", payload);
            let endpoint = format!("{}api/events/raw/", self.server_url);
            let client = hyper::Client::new();
            let req = client.post(&endpoint)
                .body(&el)
                .header(Connection::close());
            let mut res = try!(req.send());

            let mut body = String::new();
            try!(res.read_to_string(&mut body).map(|_| ()))
        }
        
        Ok(())
    }
}

fn format_payload(event: &events::Event) -> String {
    let mut body = format!("{{\"Timestamp\":\"{}\",\"Level\":\"{}\",\"MessageTemplate\":{},\"Properties\":{{",
        event.timestamp().format("%FT%TZ"),
        to_seq_level(event.level()),
        serde_json::to_string(event.message_template()).unwrap());
    
    let mut first = true;
    for (n,v) in event.properties() {
        
        if !first {
            body.push_str(",");
        } else {
            first = false;
        }
        
        write!(&mut body, "\"{}\":{}", n, v).is_ok();            
    }                    
    body.push_str("}}");
    
    body     
}

fn to_seq_level(level: log::LogLevel) -> &'static str {
    SEQ_LEVEL_NAMES[level as usize]
}

#[cfg(test)]
mod tests {
    use std::collections;
    use chrono::UTC;
    use chrono::offset::TimeZone;
    use log;
    use events;
    use collectors::seq::format_payload;
    
    #[test]
    fn events_are_formatted() {
        let timestamp = UTC.ymd(2014, 7, 8).and_hms(9, 10, 11);  
        let mut properties: collections::BTreeMap<&'static str, String> = collections::BTreeMap::new();
        properties.insert("number", "42".to_owned());
        let evt = events::Event::new(timestamp, log::LogLevel::Warn, "The number is {number}".to_owned(), properties);
        let payload = format_payload(&evt);
        assert_eq!(payload, "{\"Timestamp\":\"2014-07-08T09:10:11Z\",\"Level\":\"Warning\",\"MessageTemplate\":\"The number is {number}\",\"Properties\":{\"number\":42}}".to_owned());
    }
}