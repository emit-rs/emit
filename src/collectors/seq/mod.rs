use hyper;
use hyper::header::Connection;
use std::io::Read;
use std::error::Error;
use std::fmt::Write;
use serde_json;
use events;
use log;

pub const DEFAULT_EVENT_BODY_LIMIT_BYTES: usize = 1024 * 256;
pub const DEFAULT_BATCH_LIMIT_BYTES: usize = 1024 * 1024 * 10;
pub const LOCAL_SERVER_URL: &'static str = "http://localhost:5341/";

header! { (XSeqApiKey, "X-Seq-ApiKey") => [String] }

// 0 is "OFF", but fatal is the best effort for rendering this if we ever get an
// event with that level.
static SEQ_LEVEL_NAMES: [&'static str; 6] = ["Fatal", "Error", "Warning", "Information", "Debug", "Verbose"];

pub struct SeqCollector {
    api_key: Option<String>, 
    event_body_limit_bytes: usize, 
    batch_limit_bytes: usize,
    endpoint: String
}

impl SeqCollector {
    pub fn new<'b>(server_url: &'b str, api_key: Option<&'b str>, event_body_limit_bytes: usize, batch_limit_bytes: usize) -> SeqCollector {
        SeqCollector {
            api_key: api_key.map(|k| k.to_owned()),
            event_body_limit_bytes: event_body_limit_bytes,
            batch_limit_bytes: batch_limit_bytes,
            endpoint: format!("{}api/events/raw/", server_url)
        }
    }
    
    pub fn new_local() -> SeqCollector {
        Self::new(LOCAL_SERVER_URL, None, DEFAULT_EVENT_BODY_LIMIT_BYTES, DEFAULT_BATCH_LIMIT_BYTES)
    }
    
    fn send_batch(&self, payload: &String)  -> Result<(), Box<Error>> {
        let client = hyper::Client::new();
        let mut req = client.post(&self.endpoint)
            .body(payload)
            .header(Connection::close());
            
        if let Some(ref api_key) = self.api_key {
            req = req.header(XSeqApiKey(api_key.clone()));
        }
            
        let mut res = try!(req.send());

        let mut body = String::new();
        try!(res.read_to_string(&mut body));
        Ok(())
    }
}

const HEADER: &'static str = "{\"Events\":[";
const HEADER_LEN: usize = 11;
const FOOTER: &'static str = "]}";
const FOOTER_LEN: usize = 2;

impl super::AcceptEvents for SeqCollector {
    fn accept_events(&self, events: &[events::Event<'static>]) -> Result<(), Box<Error>> {
        let mut next = HEADER.to_owned();
        let mut count = HEADER_LEN + FOOTER_LEN;
        let mut delim = "";
        
        for event in events {
            let mut payload = format_payload(event);
            if payload.len() > self.event_body_limit_bytes {
                payload = format_oversize_placeholder(event);
                if payload.len() > self.event_body_limit_bytes {
                    error!("An oversize event was detected but the size limit is so low a placeholder cannot be substituted");
                    continue;
                }
            }
            
            // Make sure at least one event is included in each batch
            if delim != "" && count + delim.len() + payload.len() > self.batch_limit_bytes {
                write!(next, "{}", FOOTER).is_ok();
                try!(self.send_batch(&next));
                
                next = format!("{}{}", HEADER, payload);
                count = HEADER_LEN + FOOTER_LEN + payload.len();
                delim = ",";
            } else {
                write!(next, "{}{}", delim, payload).is_ok();
                count += delim.len() + payload.len();
                delim = ",";
            }            
        }

        write!(next, "{}", FOOTER).is_ok();
        try!(self.send_batch(&next));
        
        Ok(())
    }
}

fn format_payload(event: &events::Event) -> String {
    let mut body = format!("{{\"Timestamp\":\"{}\",\"Level\":\"{}\",\"MessageTemplate\":{},\"Properties\":{{",
        event.timestamp().format("%FT%TZ"),
        to_seq_level(event.level()),
        serde_json::to_string(event.message_template().text()).unwrap());
    
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

fn format_oversize_placeholder(event: &events::Event) -> String {
    let initial: String = if event.message_template().text().len() > 64 {
        event.message_template().text().chars().take(64).into_iter().collect()
    } else {
        event.message_template().text().clone()
    };
    
    format!("{{\"Timestamp\":\"{}\",\"Level\":\"{}\",\"MessageTemplate\":\"(Event too large) {{initial}}...\",\"Properties\":{{\"target\":\"emit::collectors::seq\",\"initial\":{}}}}}",
        event.timestamp().format("%FT%TZ"),
        to_seq_level(event.level()),
        serde_json::to_string(&initial).unwrap())
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
    use templates;
    
    #[test]
    fn events_are_formatted() {
        let timestamp = UTC.ymd(2014, 7, 8).and_hms(9, 10, 11);  
        let mut properties: collections::BTreeMap<&'static str, String> = collections::BTreeMap::new();
        properties.insert("number", "42".to_owned());
        let evt = events::Event::new(timestamp, log::LogLevel::Warn, templates::MessageTemplate::new("The number is {number}"), properties);
        let payload = format_payload(&evt);
        assert_eq!(payload, "{\"Timestamp\":\"2014-07-08T09:10:11Z\",\"Level\":\"Warning\",\"MessageTemplate\":\"The number is {number}\",\"Properties\":{\"number\":42}}".to_owned());
    }
}
