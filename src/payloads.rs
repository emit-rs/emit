use chrono;
use std::collections::{BTreeMap};
use std::fmt::Write;
use serde_json;

pub fn format_payload(timestamp: chrono::DateTime<chrono::UTC>, template: &str, properties: &BTreeMap<&'static str, String>) -> String {
    let mut body = "{\"Level\":\"Information\",\"Properties\":{".to_owned();
    
    let mut first = true;
    for (n,v) in properties {
        
        if !first {
            body.push_str(",");
        } else {
            first = false;
        }
        
        write!(&mut body, "\"{}\":{}", n, v).is_ok();            
    }
                    
    write!(&mut body, "}},\"Timestamp\":\"{}\",\"MessageTemplate\":{}}}",
        timestamp.format("%FT%TZ"),
        serde_json::to_string(&template).unwrap()).is_ok();
    
    body     
}
