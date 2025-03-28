/*!
The [`ThreadLocalCtxt`] type.
*/

use std::{
    cell::RefCell,
    collections::HashMap,
    mem,
    ops::ControlFlow,
    sync::{Arc, Mutex},
};

use emit_core::{
    ctxt::Ctxt,
    props::Props,
    runtime::InternalCtxt,
    str::Str,
    value::{OwnedValue, ToValue, Value},
};

use crate::span::{SpanId, TraceId};

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
    props: Option<Arc<HashMap<Str<'static>, ThreadLocalValue>>>,
}

#[derive(Clone)]
enum ThreadLocalValue {
    TraceId(TraceId),
    SpanId(SpanId),
    Any(OwnedValue),
}

impl ThreadLocalValue {
    fn from_value(value: Value) -> Self {
        // Specialize a few common value types

        if let Some(trace_id) = value.downcast_ref() {
            return ThreadLocalValue::TraceId(*trace_id);
        }

        if let Some(span_id) = value.downcast_ref() {
            return ThreadLocalValue::SpanId(*span_id);
        }

        // Fall back to buffering
        ThreadLocalValue::Any(value.to_shared())
    }
}

impl ToValue for ThreadLocalValue {
    fn to_value(&self) -> Value {
        match self {
            ThreadLocalValue::TraceId(ref value) => value.to_value(),
            ThreadLocalValue::SpanId(ref value) => value.to_value(),
            ThreadLocalValue::Any(ref value) => value.to_value(),
        }
    }
}

impl Props for ThreadLocalCtxtFrame {
    fn for_each<'a, F: FnMut(Str<'a>, Value<'a>) -> ControlFlow<()>>(
        &'a self,
        mut for_each: F,
    ) -> ControlFlow<()> {
        if let Some(ref props) = self.props {
            for (k, v) in &**props {
                for_each(k.by_ref(), v.to_value())?;
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

        let _ = props.for_each(|k, v| {
            span.insert(k.to_shared(), ThreadLocalValue::from_value(v));

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

        let _ = props.for_each(|k, v| {
            span_props.insert(k.to_shared(), ThreadLocalValue::from_value(v));

            ControlFlow::Continue(())
        });

        span
    }

    fn enter(&self, frame: &mut Self::Frame) {
        swap(self.id, frame);
    }

    fn exit(&self, frame: &mut Self::Frame) {
        swap(self.id, frame);
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
    use std::thread;

    impl ThreadLocalCtxtFrame {
        fn props(&self) -> HashMap<Str<'static>, ThreadLocalValue> {
            self.props
                .as_ref()
                .map(|props| (**props).clone())
                .unwrap_or_default()
        }
    }

    #[test]
    fn can_inline() {
        use std::mem;

        // Mirrors the impl of `ErasedFrame`
        union RawErasedFrame {
            _data: *mut (),
            _inline: mem::MaybeUninit<[usize; 2]>,
        }

        assert!(
            mem::size_of::<ThreadLocalCtxt>() <= mem::size_of::<RawErasedFrame>()
                && mem::align_of::<ThreadLocalCtxt>() <= mem::align_of::<RawErasedFrame>()
        );
    }

    #[test]
    fn push_props() {
        let ctxt = ThreadLocalCtxt::new();

        ctxt.clone().with_current(|props| {
            assert_eq!(0, props.props().len());
        });

        let mut frame = ctxt.clone().open_push(("a", 1));

        assert_eq!(1, frame.props().len());
        ctxt.clone().with_current(|props| {
            assert_eq!(0, props.props().len());
        });

        ctxt.clone().enter(&mut frame);

        assert_eq!(0, frame.props().len());

        let mut inner = ctxt.clone().open_push([("b", 1), ("a", 2)]);

        ctxt.clone().with_current(|props| {
            let props = props.props();

            assert_eq!(1, props.len());
            assert_eq!(1, props["a"].to_value().cast::<i32>().unwrap());
        });

        ctxt.clone().enter(&mut inner);

        ctxt.clone().with_current(|props| {
            let props = props.props();

            assert_eq!(2, props.len());
            assert_eq!(2, props["a"].to_value().cast::<i32>().unwrap());
            assert_eq!(1, props["b"].to_value().cast::<i32>().unwrap());
        });

        ctxt.clone().exit(&mut inner);

        ctxt.clone().exit(&mut frame);

        assert_eq!(1, frame.props().len());
        ctxt.clone().with_current(|props| {
            assert_eq!(0, props.props().len());
        });
    }

    #[test]
    fn out_of_order_enter_exit() {
        let ctxt = ThreadLocalCtxt::new();

        let mut a = ctxt.open_push(("a", 1));

        ctxt.enter(&mut a);

        let mut b = ctxt.open_push(("b", 1));

        ctxt.enter(&mut b);

        // Ensure out-of-order exit doesn't panic

        ctxt.exit(&mut a);
        ctxt.exit(&mut b);
    }

    #[test]
    fn isolation() {
        let ctxt_a = ThreadLocalCtxt::new();

        let ctxt_b = ThreadLocalCtxt::new();

        let mut frame_a = ctxt_a.open_push(("a", 1));

        ctxt_a.enter(&mut frame_a);

        ctxt_a.with_current(|props| {
            assert_eq!(1, props.props().len());
        });

        ctxt_b.with_current(|props| {
            assert_eq!(0, props.props().len());
        });

        ctxt_a.exit(&mut frame_a);
    }

    #[test]
    fn frame_thread_propagation() {
        let ctxt = ThreadLocalCtxt::new();

        let mut frame = ctxt.open_push(("a", 1));

        ctxt.enter(&mut frame);

        thread::spawn({
            let ctxt = ctxt.clone();

            move || {
                ctxt.with_current(|props| {
                    assert_eq!(0, props.props().len());
                });
            }
        })
        .join()
        .unwrap();

        let mut current = ctxt.with_current(|props| props.clone());

        thread::spawn({
            let ctxt = ctxt.clone();

            move || {
                ctxt.enter(&mut current);

                ctxt.with_current(|props| {
                    assert_eq!(1, props.props().len());
                });

                ctxt.exit(&mut current);
            }
        })
        .join()
        .unwrap();
    }
}
