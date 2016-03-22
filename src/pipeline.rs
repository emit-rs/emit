use collectors;
use events;
use std::sync::mpsc::{channel, Sender};
use std::thread;
use std::mem;

pub enum Item {
    Done,
    Emit(events::Event)
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
    
pub fn emit(event: events::Event) {
    // Should be an atomic read, etc.
    let p = unsafe { PIPELINE };    
    if p != 0 as *const PipelineRef {
        // Not sound, as chan is not Sync :-(
        unsafe {
            (*p).chan.send(Item::Emit(event)).is_ok();
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
                    if let Err(e) = coll.dispatch(&vec![payload]) {
                        error!("Could not dispatch events: {}", e);
                    }
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
