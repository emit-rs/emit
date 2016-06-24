use events::Event;
use std::sync::atomic;
use LogLevel;
use std::mem;
use pipeline::reference::PipelineRef;

static PIPELINE_REF: atomic::AtomicUsize = atomic::ATOMIC_USIZE_INIT;

fn get_ambient_ref() -> *const PipelineRef {
    PIPELINE_REF.load(atomic::Ordering::Relaxed) as *const PipelineRef
}

pub fn set_ambient_ref(pref: PipelineRef) {
    let bpref = Box::new(pref);
    PIPELINE_REF.store(unsafe { mem::transmute::<Box<PipelineRef>, *const PipelineRef>(bpref) } as usize, atomic::Ordering::SeqCst);
}

pub fn is_enabled(level: LogLevel) -> bool {
    let p = get_ambient_ref();    
    if p != 0 as *const PipelineRef {
        unsafe {
            (*p).is_enabled(level)
        }
    } else {
        false
    }
}
    
pub fn emit(event: Event<'static>) {
    let p = get_ambient_ref();    
    if p != 0 as *const PipelineRef {
        unsafe {
            (*p).emit(event);
        }
    }
}
