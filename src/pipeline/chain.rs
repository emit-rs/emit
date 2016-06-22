use events::Event;

/// A link to the next element in a processing chain.
pub trait Emit {
    fn emit(&self, event: Event<'static>);
}

/// An element within the processing chain that controls
/// how an `Event` is passed through.
pub trait Propagate {
    fn propagate(&self, event: Event<'static>, next: &Emit);
}

struct ChainedPipelineElement {
    current: Box<Propagate + Sync>,
    next: Box<Emit + Sync>
}

impl Emit for ChainedPipelineElement {
    fn emit(&self, event: Event<'static>) {
        self.current.propagate(event, &*self.next);
    }
}

struct TerminatingElement {}

impl Emit for TerminatingElement {
    #[allow(unused_variables)]
    fn emit(&self, event: Event<'static>) {
    }
}

pub fn to_chain(elements: Vec<Box<Propagate + Sync>>, terminator: Option<Box<Emit + Sync>>) -> Box<Emit + Sync> {
    let mut els = elements;
    els.reverse();
    
    let mut head = terminator.unwrap_or(Box::new(TerminatingElement {}));
    for el in els.into_iter() {
        head = Box::new(ChainedPipelineElement { current: el, next: head });
    }
    
    head
}
