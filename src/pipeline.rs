use chrono::{DateTime,UTC};
use std::collections::{BTreeMap};
use payloads;
use collectors;
use std::io::Read;
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

pub fn init<T: collectors::Collector + Send + 'static>(collector: T) -> Pipeline {
    let (tx, rx) = channel::<Item>();
    unsafe {
        let pr = Box::new(PipelineRef { chan: tx.clone() });
        // Should be atomic CAS etc.
        PIPELINE = mem::transmute::<Box<PipelineRef>, *const PipelineRef>(pr);
    }
    
    let coll = collector;
    let child = thread::spawn(move|| {
        loop {
            let done = match rx.recv().unwrap() {
                Item::Done => true,
                Item::Emit(payload) => {
                    coll.dispatch(&vec![payload]);
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
