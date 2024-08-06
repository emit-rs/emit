/*!
The [`ThreadLocalCtxt`] type.
*/

use core::mem;
use std::{cell::RefCell, collections::HashMap, ops::ControlFlow, sync::Mutex};

use alloc::sync::Arc;
use emit_core::{
    ctxt::Ctxt,
    props::Props,
    runtime::InternalCtxt,
    str::Str,
    value::{OwnedValue, Value},
};

/**
A [`Ctxt`] that stores ambient state in thread local storage.

Frames fully encapsulate all properties that were active when they were created so can be sent across threads to move that state with them.
*/
#[derive(Debug, Clone, Copy)]
pub struct ThreadLocalCtxt {
    id: usize,
}

impl Default for ThreadLocalCtxt {
    fn default() -> Self {
        Self::new()
    }
}

impl ThreadLocalCtxt {
    /**
    Create a new thread local store with fully isolated storage.
    */
    pub fn new() -> Self {
        ThreadLocalCtxt { id: ctxt_id() }
    }

    /**
    Create a new thread local store sharing the same storage as any other [`ThreadLocalCtxt::shared`].
    */
    pub const fn shared() -> Self {
        ThreadLocalCtxt { id: 0 }
    }
}

/**
A [`Ctxt::Frame`] on a [`ThreadLocalCtxt`].
*/
#[derive(Clone)]
pub struct ThreadLocalCtxtFrame {
    props: Option<Arc<HashMap<Str<'static>, OwnedValue>>>,
}

impl Props for ThreadLocalCtxtFrame {
    fn for_each<'a, F: FnMut(Str<'a>, Value<'a>) -> ControlFlow<()>>(
        &'a self,
        mut for_each: F,
    ) -> ControlFlow<()> {
        if let Some(ref props) = self.props {
            for (k, v) in &**props {
                for_each(k.by_ref(), v.by_ref())?;
            }
        }

        ControlFlow::Continue(())
    }

    fn get<'v, K: emit_core::str::ToStr>(&'v self, key: K) -> Option<Value<'v>> {
        self.props.as_ref().and_then(|props| props.get(key))
    }

    fn is_unique(&self) -> bool {
        true
    }
}

impl Ctxt for ThreadLocalCtxt {
    type Current = ThreadLocalCtxtFrame;
    type Frame = ThreadLocalCtxtFrame;

    fn with_current<R, F: FnOnce(&Self::Current) -> R>(&self, with: F) -> R {
        let current = current(self.id);
        with(&current)
    }

    fn open_root<P: Props>(&self, props: P) -> Self::Frame {
        let mut span = HashMap::new();

        props.for_each(|k, v| {
            span.insert(k.to_shared(), v.to_shared());
            ControlFlow::Continue(())
        });

        ThreadLocalCtxtFrame {
            props: Some(Arc::new(span)),
        }
    }

    fn open_push<P: Props>(&self, props: P) -> Self::Frame {
        let mut span = current(self.id);

        if span.props.is_none() {
            span.props = Some(Arc::new(HashMap::new()));
        }

        let span_props = Arc::make_mut(span.props.as_mut().unwrap());

        props.for_each(|k, v| {
            span_props.insert(k.to_shared(), v.to_shared());
            ControlFlow::Continue(())
        });

        span
    }

    fn enter(&self, link: &mut Self::Frame) {
        swap(self.id, link);
    }

    fn exit(&self, link: &mut Self::Frame) {
        swap(self.id, link);
    }

    fn close(&self, _: Self::Frame) {}
}

impl InternalCtxt for ThreadLocalCtxt {}

// Start this id from 1 so it doesn't intersect with the `shared` variant below
static NEXT_CTXT_ID: Mutex<usize> = Mutex::new(1);

fn ctxt_id() -> usize {
    let mut next_id = NEXT_CTXT_ID.lock().unwrap();
    let id = *next_id;
    *next_id = id.wrapping_add(1);

    id
}

thread_local! {
    static ACTIVE: RefCell<HashMap<usize, ThreadLocalCtxtFrame>> = RefCell::new(HashMap::new());
}

fn current(id: usize) -> ThreadLocalCtxtFrame {
    ACTIVE.with(|active| {
        active
            .borrow_mut()
            .entry(id)
            .or_insert_with(|| ThreadLocalCtxtFrame { props: None })
            .clone()
    })
}

fn swap(id: usize, incoming: &mut ThreadLocalCtxtFrame) {
    ACTIVE.with(|active| {
        let mut active = active.borrow_mut();

        let current = active
            .entry(id)
            .or_insert_with(|| ThreadLocalCtxtFrame { props: None });

        mem::swap(current, incoming);
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_props() {
        todo!()
    }

    #[test]
    fn isolation() {
        todo!()
    }

    #[test]
    fn frame_thread_propagation() {
        todo!()
    }
}
