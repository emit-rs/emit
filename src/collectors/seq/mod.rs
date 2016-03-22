use hyper;
use hyper::header::Connection;
use std::io::Read;
use std::fmt::Write;
use serde_json;
use events;

pub const DEFAULT_EVENT_BODY_LIMIT_BYTES: usize = 1024 * 256;
pub const DEFAULT_BATCH_LIMIT_BYTES: usize = 1024 * 1024 * 10;
pub const LOCAL_SERVER_URL: &'static str = "http://localhost:5341/";
    
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
    let mut body = "{\"Level\":\"Information\",\"Properties\":{".to_owned();
    
    let mut first = true;
    for (n,v) in event.properties() {
        
        if !first {
            body.push_str(",");
        } else {
            first = false;
        }
        
        write!(&mut body, "\"{}\":{}", n, v).is_ok();            
    }
                    
    write!(&mut body, "}},\"Timestamp\":\"{}\",\"MessageTemplate\":{}}}",
        event.timestamp().format("%FT%TZ"),
        serde_json::to_string(event.message_template()).unwrap()).is_ok();
    
    body     
}
