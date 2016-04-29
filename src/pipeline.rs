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
/// into which events are fed. TODO: This type is misnamed, from the user's perspective
/// it's the pipeline.
pub struct PipelineRef {
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

/// `Pipeline` is the asynchronous "consumer" end that dispatches events to collectors.
/// TODO: This type is mis-named: from the user's perspective it is more akin to `thread::JoinHandle`
/// and needs a name echoing that role (`AsyncFlushHandle`?)
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

struct SenderElement {
    chan: sync::Mutex<Sender<Item>>
}

impl elements::ChainedElement for SenderElement {
    fn emit(&self, event: events::Event) {
        self.chan.lock().unwrap().send(Item::Emit(event)).expect("The event could not be emitted to the pipeline");
    }
}

struct TerminatingElement {}

impl elements::ChainedElement for TerminatingElement {
    #[allow(unused_variables)]
    fn emit(&self, event: events::Event) {
    }
}

pub struct PipelineBuilder {
    level: log::LogLevel,
    elements: Vec<Box<elements::PipelineElement>>,
    terminator: Option<Box<elements::ChainedElement>>,
    flush: Option<Pipeline>
}

impl PipelineBuilder {
    pub fn at_level(mut self, level: log::LogLevel) -> PipelineBuilder {
        self.level = level;
        self
    }
    
    pub fn pipe(mut self, element: Box<elements::PipelineElement>) -> PipelineBuilder {
        self.elements.push(element);
        self
    }

    /// Send to a collector, asynchronously. Only one collector may receive events this way. (A non-consuming
    /// alternative wired in with `pipe()` is intended.)
    pub fn send_to<T: collectors::Collector + Send + 'static>(mut self, collector: T) -> PipelineBuilder {
        let (tx, rx) = channel::<Item>();
        self.terminator = Some(Box::new(SenderElement { chan: sync::Mutex::new(tx.clone()) }));
        self.flush = Some(Pipeline::new(collector, tx, rx));
        self
    }
    
    /// Build the pipeline, but don't globally install it.
    pub fn detach(self) -> (PipelineRef, Option<Pipeline>) {
        let terminator = self.terminator.unwrap_or(Box::new(TerminatingElement {}));
        let head = elements::to_chain(self.elements, terminator);
        let pref = PipelineRef {
            head: head,
            filter: self.level.to_log_level_filter()
        };
            
        (pref, self.flush)
    }

    /// Build and globally install the pipeline so that the `emit!()` macros can call it.
    pub fn init(self) -> Option<Pipeline> {
        let (pref, flush) = self.detach();
            
        let bpref = Box::new(pref);
        PIPELINE_REF.store(unsafe { mem::transmute::<Box<PipelineRef>, *const PipelineRef>(bpref) } as usize, atomic::Ordering::SeqCst);
        
        flush
    }
}

pub fn builder() -> PipelineBuilder {
    PipelineBuilder { 
        level: log::LogLevel::Info,
        elements: vec![],
        terminator: None,
        flush: None
    }
}

#[cfg(test)]
mod tests {
    use pipeline;
    use log;
    
    #[test]
    fn info_is_enabled_at_info() {
        let (p, _flush) = pipeline::builder().detach();
        assert!(p.is_enabled(log::LogLevel::Info));
    }
    
    #[test]
    fn warn_is_enabled_at_info() {
        let (p, _flush) = pipeline::builder().detach();
        assert!(p.is_enabled(log::LogLevel::Warn));
    }  
      
    #[test]
    fn debug_is_disabled_at_info() {
        let (p, _flush) = pipeline::builder().detach();
        assert!(!p.is_enabled(log::LogLevel::Debug));
    }
}
