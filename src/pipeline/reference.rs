use events::Event;
use pipeline::chain::Emit;
use super::super::{LogLevel,LogLevelFilter};

/// `PipelineRef` is "mouth" of the pipeline, into which events are fed.
pub struct PipelineRef {
    head: Box<Emit + Sync>,
    filter: LogLevelFilter
}

//unsafe impl Sync for PipelineRef { }

impl PipelineRef {
    pub fn new(head: Box<Emit + Sync>, level: LogLevelFilter) -> PipelineRef {
        PipelineRef {
            head: head,
            filter: level
        }
    }
    
    /// Check if the specified log level is enabled for the pipeline.
    pub fn is_enabled(&self, level: LogLevel) -> bool {
        self.filter.is_enabled(level)
    }
    
    /// Emit an event through the pipeline. Code wishing to _conditionally_
    /// emit events based on the level should call `is_enabled()` first.
    pub fn emit(&self, event: Event<'static>) {
        self.head.emit(event);
    }
}

#[cfg(test)]
mod tests {
    use pipeline::builder::PipelineBuilder;
    use LogLevel;
    
    #[test]
    fn info_is_enabled_at_info() {
        let (p, _flush) = PipelineBuilder::new().detach();
        assert!(p.is_enabled(LogLevel::Info));
    }
    
    #[test]
    fn warn_is_enabled_at_info() {
        let (p, _flush) = PipelineBuilder::new().detach();
        assert!(p.is_enabled(LogLevel::Warn));
    }  
      
    #[test]
    fn debug_is_disabled_at_info() {
        let (p, _flush) = PipelineBuilder::new().detach();
        assert!(!p.is_enabled(LogLevel::Debug));
    }
}

