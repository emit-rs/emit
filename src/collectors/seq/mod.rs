use hyper;
use hyper::header::Connection;
use std::io;
use std::io::Read;

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
    
    pub fn local() -> SeqCollector {
        Self::new(LOCAL_SERVER_URL, None, DEFAULT_EVENT_BODY_LIMIT_BYTES, DEFAULT_BATCH_LIMIT_BYTES)
    }
}

impl super::Collector for SeqCollector {
    type Error = io::Error;
    
    fn dispatch(&self, events: &[String]) -> Result<(), Self::Error> {
        for payload in events {
            let el = format!("{{\"Events\":[{}]}}", payload);

            let client = hyper::Client::new();
            let mut res = client.post(&format!("{}api/events/raw/", self.server_url))
                .body(&el)
                .header(Connection::close())
                .send().unwrap();

            let mut body = String::new();
            try!(res.read_to_string(&mut body).map(|s| ()))
        }
        
        Ok(())
    }
}
