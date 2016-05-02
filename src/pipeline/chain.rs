use events::Event;

/// A link to the next element in a processing chain.
pub trait ChainedElement : Sync {
    fn emit(&self, event: Event);
}

/// An element within the processing chain that controls
/// how an `Event` is passed through.
pub trait PipelineElement : Sync {
    fn emit(&self, event: Event, next: &ChainedElement);
}

struct ChainedPipelineElement {
    current: Box<PipelineElement>,
    next: Box<ChainedElement>
}

impl ChainedElement for ChainedPipelineElement {
    fn emit(&self, event: Event) {
        self.current.emit(event, &*self.next);
    }
}

struct TerminatingElement {}

impl ChainedElement for TerminatingElement {
    #[allow(unused_variables)]
    fn emit(&self, event: Event) {
    }
}

pub fn to_chain(elements: Vec<Box<PipelineElement>>, terminator: Option<Box<ChainedElement>>) -> Box<ChainedElement> {
    let mut els = elements;
    els.reverse();
    
    let mut head = terminator.unwrap_or(Box::new(TerminatingElement {}));
    for el in els.into_iter() {
        head = Box::new(ChainedPipelineElement { current: el, next: head });
    }
    
    head
}
