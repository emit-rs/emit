use chrono::{DateTime,UTC};
use std::collections::{BTreeMap};
use payloads;
use std::io::Read;
use hyper::Client;
use hyper::header::Connection;
use std::sync::mpsc::{channel, Sender};
use std::thread;
use std::mem;

pub enum Item {
    Done,
    Emit(String)
}

pub struct PipelineRef {
    chan: Sender<Item>
}

static mut PIPELINE: *const PipelineRef = 0 as *const PipelineRef;

pub struct Pipeline {
    chan: Sender<Item>,
    worker: Option<thread::JoinHandle<()>>
}

impl Drop for Pipeline {
    fn drop(&mut self) {
        self.chan.send(Item::Done).is_ok();
        let thread = mem::replace(&mut self.worker, None);
        if let Some(handle) = thread {
            handle.join().is_ok();
        }
    }
}
    
pub fn emit(template: &str, properties: &BTreeMap<&'static str, String>) {
    // Should be an atomic read, etc.
    let p = unsafe { PIPELINE };    
    if p != 0 as *const PipelineRef {
        let timestamp: DateTime<UTC> = UTC::now();
        let payload = payloads::format_payload(timestamp, template, properties);
        unsafe {
            (*p).chan.send(Item::Emit(payload)).is_ok();
        }
    }
}

fn dispatch(server_url: &str, api_key: Option<&str>, payload: String) {
    let events = format!("{{\"Events\":[{}]}}", payload);

    let client = Client::new();
    let mut res = client.post(&format!("{}api/events/raw/", server_url))
        .body(&events)
        .header(Connection::close())
        .send().unwrap();

    let mut body = String::new();
    res.read_to_string(&mut body).unwrap();

    info!("Response: {}", body);
}

pub fn init(server_url: &str, api_key: Option<&str>) -> Pipeline {
    let (tx, rx) = channel::<Item>();
    unsafe {
        let pr = Box::new(PipelineRef { chan: tx.clone() });
        // Should be atomic CAS etc.
        PIPELINE = mem::transmute::<Box<PipelineRef>, *const PipelineRef>(pr);
    }
    
    let url = server_url.to_owned();
    let child = thread::spawn(move|| {
        loop {
            let done = match rx.recv().unwrap() {
                Item::Done => true,
                Item::Emit(payload) => {
                    dispatch(&url, None, payload);
                    false
                }
            };
            
            if done {
                break;
            }
        }
    });
    
    Pipeline {
         worker: Some(child),
         chan: tx
    }
}
