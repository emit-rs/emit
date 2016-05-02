use collectors;
use std::sync::mpsc::channel;
use log;
use pipeline::chain;
use pipeline::chain::{ChainedElement, PipelineElement};
use pipeline::ambient;
use pipeline::async::{Item, AsyncCollector, SenderElement};
use pipeline::reference::PipelineRef;

/// A handle to the background asynchronously-operating collector. When
// `drop()`ped, the background collector (if any) will be flushed and shut down.
pub struct AsyncFlushHandle {
    #[allow(dead_code)]
    async_collector: Option<AsyncCollector>
}

/// `PipelineBuilder` creates an event emitting pipeline. Calling `init()` will install the pipeline globally
/// for use by the `emit!()` family of macros. Calling `detach()` will return an independent pipeline that
/// can be used in isolation.
pub struct PipelineBuilder {
    level: log::LogLevel,
    elements: Vec<Box<PipelineElement>>,
    terminator: Option<Box<ChainedElement>>,
    async_collector: Option<AsyncCollector>
}

impl PipelineBuilder {
    pub fn new() -> PipelineBuilder {
        PipelineBuilder { 
            level: log::LogLevel::Info,
            elements: vec![],
            terminator: None,
            async_collector: None
        }
    }

    /// Set the logging level used by the pipeline. The default is `log::LogLevel::Info`.
    pub fn at_level(mut self, level: log::LogLevel) -> PipelineBuilder {
        self.level = level;
        self
    }
    
    /// Add a processing element to the pipeline. Elements run in the order in which they
    /// are added, so the output of one `pipe()`d element is fed into the next.
    pub fn pipe(mut self, element: Box<PipelineElement>) -> PipelineBuilder {
        self.elements.push(element);
        self
    }

    /// Send to a collector, asynchronously. Only one collector may receive events this way. (A non-consuming
    /// alternative wired in with `pipe()` is intended.)
    pub fn send_to<T: collectors::Collector + Send + 'static>(mut self, collector: T) -> PipelineBuilder {
        let (tx, rx) = channel::<Item>();
        self.terminator = Some(Box::new(SenderElement::new(tx.clone())));
        self.async_collector = Some(AsyncCollector::new(collector, tx, rx));
        self
    }
    
    /// Build the pipeline, but don't globally install it.
    pub fn detach(self) -> (PipelineRef, AsyncFlushHandle) {
        let head = chain::to_chain(self.elements, self.terminator);
        let pref = PipelineRef::new(head, self.level);
            
        (pref, AsyncFlushHandle {async_collector: self.async_collector})
    }

    /// Build and globally install the AsyncCollectorso that the `emit!()` macros can call it.
    pub fn init(self) -> AsyncFlushHandle {
        let (pref, flush) = self.detach();
            
        ambient::set_ambient_ref(pref);
        
        flush
    }
}

