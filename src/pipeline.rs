use collectors;
use events;
use std::sync::mpsc::{channel, Sender};
use std::thread;
use std::mem;
use std::sync;
use std::sync::atomic;
use log;

pub enum Item {
    Done,
    Emit(events::Event)
}

pub struct PipelineRef {
    chan: sync::Mutex<Sender<Item>>,
    filter: log::LogLevelFilter
}

static PIPELINE: atomic::AtomicUsize = atomic::ATOMIC_USIZE_INIT;

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

pub fn is_enabled(level: log::LogLevel) -> bool {
    let p = PIPELINE.load(atomic::Ordering::Relaxed) as *const PipelineRef;    
    if p != 0 as *const PipelineRef {
        unsafe {
            (*p).filter >= level
        }
    } else {
        false
    }
}
    
pub fn emit(event: events::Event) {
    let p = PIPELINE.load(atomic::Ordering::Relaxed) as *const PipelineRef;    
    if p != 0 as *const PipelineRef {
        unsafe {
            (*p).chan.lock().unwrap().send(Item::Emit(event)).is_ok();
        }
    }
}

pub fn init<T: collectors::Collector + Send + 'static>(collector: T, level: log::LogLevel) -> Pipeline {
    let (tx, rx) = channel::<Item>();
    let pr = Box::new(PipelineRef {
            chan: sync::Mutex::new(tx.clone()),
            filter: level.to_log_level_filter()
        });
        
    PIPELINE.store(unsafe { mem::transmute::<Box<PipelineRef>, *const PipelineRef>(pr) } as usize, atomic::Ordering::SeqCst);
    
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

#[cfg(test)]
mod tests {
    use pipeline;
    use collectors::silent::SilentCollector;
    use log;
    
    #[test]
    fn info_is_enabled_at_info() {
        let _flush = pipeline::init(SilentCollector::new(), log::LogLevel::Info);
        assert!(pipeline::is_enabled(log::LogLevel::Info));
    }
    
    #[test]
    fn warn_is_enabled_at_info() {
        let _flush = pipeline::init(SilentCollector::new(), log::LogLevel::Info);
        assert!(pipeline::is_enabled(log::LogLevel::Warn));
    }  
      
    #[test]
    fn debug_is_disabled_at_info() {
        let _flush = pipeline::init(SilentCollector::new(), log::LogLevel::Info);
        assert!(!pipeline::is_enabled(log::LogLevel::Debug));
    }
}