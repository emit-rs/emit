//! An asynchronous/buffered log event pipeline from producers to a single dispatching consumer.
//! Currently based on channels, but highly likely this will change.

use collectors;
use events;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;
use std::mem;
use std::sync;
use std::sync::atomic;
use log;
use elements;

enum Item {
    Done,
    Emit(events::Event)
}

/// `PipelineRef` is the (eventually highly-concurrent) "mouth" of the pipeline,
/// into which events are fed.
struct PipelineRef {
    head: Box<elements::ChainedElement>,
    filter: log::LogLevelFilter
}

unsafe impl Sync for PipelineRef { }

impl PipelineRef {
    pub fn is_enabled(&self, level: log::LogLevel) -> bool {
        self.filter >= level
    }
    
    pub fn emit(&self, event: events::Event) {
        self.head.emit(event);
    }
}

static PIPELINE_REF: atomic::AtomicUsize = atomic::ATOMIC_USIZE_INIT;

fn get_ambient_ref() -> *const PipelineRef {
    PIPELINE_REF.load(atomic::Ordering::Relaxed) as *const PipelineRef
}

pub fn is_enabled(level: log::LogLevel) -> bool {
    let p = get_ambient_ref();    
    if p != 0 as *const PipelineRef {
        unsafe {
            (*p).is_enabled(level)
        }
    } else {
        false
    }
}
    
pub fn emit(event: events::Event) {
    let p = get_ambient_ref();    
    if p != 0 as *const PipelineRef {
        unsafe {
            (*p).emit(event);
        }
    }
}

/// `Pipeline` is the  "consumer" end that dispatches events to collectors.
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

impl Pipeline {
    fn new<T: collectors::Collector + Send + 'static>(collector: T, tx: Sender<Item>, rx: Receiver<Item>) -> Pipeline {
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
}

struct ChainTerminator {
    chan: sync::Mutex<Sender<Item>>
}

impl elements::ChainedElement for ChainTerminator {
    fn emit(&self, event: events::Event) {
        self.chan.lock().unwrap().send(Item::Emit(event)).expect("The event could not be emitted to the pipeline");
    }
}

pub fn init<T: collectors::Collector + Send + 'static>(level: log::LogLevel, elements: Vec<Box<elements::PipelineElement>>, collector: T) -> Pipeline {
    let (tx, rx) = channel::<Item>();
    let terminator = ChainTerminator { chan: sync::Mutex::new(tx.clone()) };
    let head = elements::to_chain(elements, Box::new(terminator));
    let pr = Box::new(PipelineRef {
            head: head,
            filter: level.to_log_level_filter()
        });
        
    PIPELINE_REF.store(unsafe { mem::transmute::<Box<PipelineRef>, *const PipelineRef>(pr) } as usize, atomic::Ordering::SeqCst);
    
    Pipeline::new(collector, tx, rx)
}

#[cfg(test)]
mod tests {
    use pipeline;
    use collectors::silent::SilentCollector;
    use log;
    
    #[test]
    fn info_is_enabled_at_info() {
        let _flush = pipeline::init(log::LogLevel::Info, vec![], SilentCollector::new());
        assert!(pipeline::is_enabled(log::LogLevel::Info));
    }
    
    #[test]
    fn warn_is_enabled_at_info() {
        let _flush = pipeline::init(log::LogLevel::Info, vec![], SilentCollector::new());
        assert!(pipeline::is_enabled(log::LogLevel::Warn));
    }  
      
    #[test]
    fn debug_is_disabled_at_info() {
        let _flush = pipeline::init(log::LogLevel::Info, vec![], SilentCollector::new());
        assert!(!pipeline::is_enabled(log::LogLevel::Debug));
    }
}
