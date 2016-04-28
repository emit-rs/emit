use events::Event;

pub trait ChainedElement : Sync {
    fn emit(&self, event: Event);
}

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

pub fn to_chain(elements: Vec<Box<PipelineElement>>, terminator: Box<ChainedElement>) -> Box<ChainedElement> {
    let mut els = elements;
    els.reverse();
    
    let mut head = terminator;
    for el in els.into_iter() {
        head = Box::new(ChainedPipelineElement { current: el, next: head });
    }
    
    head
}
