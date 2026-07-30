/*!
The [`ThreadLocalCtxt`] type.
*/

use std::{
    cell::RefCell,
    collections::{HashMap, hash_map},
    mem,
    ops::ControlFlow,
    sync::{Arc, Mutex},
};

use emit_core::{
    ctxt::Ctxt,
    props::Props,
    runtime::InternalCtxt,
    str::{Str, ToStr},
    value::{FromValue, OwnedValue, ToValue, Value},
    well_known,
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
#[derive(Debug, Clone, Default)]
pub struct ThreadLocalCtxtFrame {
    // Store common contextual properties inline
    trace_id: Option<TraceId>,
    span_parent: Option<SpanId>,
    span_id: Option<SpanId>,
    generation: usize,
    // Store other properties in a map
    any_fallback: bool,
    rest: Option<Arc<HashMap<Str<'static>, ThreadLocalValue<OwnedValue>>>>,
}

#[derive(Debug, Clone)]
struct ThreadLocalValue<T> {
    inner: T,
    generation: usize,
}

impl ThreadLocalValue<OwnedValue> {
    fn from_value(generation: usize, value: Value) -> Self {
        ThreadLocalValue {
            inner: value.to_shared(),
            generation,
        }
    }
}

impl<T: ToValue> ToValue for ThreadLocalValue<T> {
    fn to_value(&self) -> Value<'_> {
        self.inner.to_value()
    }
}

impl Props for ThreadLocalCtxtFrame {
    fn for_each<'a, F: FnMut(Str<'a>, Value<'a>) -> ControlFlow<()>>(
        &'a self,
        mut for_each: F,
    ) -> ControlFlow<()> {
        use ToValue as _;

        let Self {
            trace_id,
            span_parent,
            span_id,
            rest,
            any_fallback: _,
            generation: _,
        } = self;

        if let Some(trace_id) = trace_id {
            for_each(well_known::KEY_TRACE_ID.to_str(), trace_id.to_value())?;
        }
        if let Some(span_parent) = span_parent {
            for_each(well_known::KEY_SPAN_PARENT.to_str(), span_parent.to_value())?;
        }
        if let Some(span_id) = span_id {
            for_each(well_known::KEY_SPAN_ID.to_str(), span_id.to_value())?;
        }

        if let Some(rest) = rest {
            for (k, v) in rest.iter() {
                for_each(k.by_ref(), v.to_value())?;
            }
        }

        ControlFlow::Continue(())
    }

    fn get<'v, K: ToStr>(&'v self, key: K) -> Option<Value<'v>> {
        use ToValue as _;

        let Self {
            trace_id,
            span_parent,
            span_id,
            rest,
            any_fallback: _,
            generation: _,
        } = self;

        let key = key.to_str();

        match key.get() {
            well_known::KEY_TRACE_ID => {
                if let Some(trace_id) = trace_id {
                    return Some(trace_id.to_value());
                }
            }
            well_known::KEY_SPAN_ID => {
                if let Some(span_id) = span_id {
                    return Some(span_id.to_value());
                }
            }
            well_known::KEY_SPAN_PARENT => {
                if let Some(span_parent) = span_parent {
                    return Some(span_parent.to_value());
                }
            }
            _ => (),
        }

        rest.as_ref().and_then(|rest| Props::get(&**rest, key))
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
        let mut frame = ThreadLocalCtxtFrame::default();

        open_frame(&mut frame, props);

        frame
    }

    fn open_push<P: Props>(&self, props: P) -> Self::Frame {
        let mut span = current(self.id);
        span.generation = span.generation.wrapping_add(1);

        open_frame(&mut span, props);

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

fn open_frame<P: Props>(span: &mut ThreadLocalCtxtFrame, props: P) {
    struct InlineSetter<'a, T> {
        seen: bool,
        slot: &'a mut Option<T>,
    }

    impl<'a, 'b, T: FromValue<'b>> InlineSetter<'a, T> {
        #[inline]
        fn new(slot: &'a mut Option<T>) -> Self {
            InlineSetter { seen: false, slot }
        }

        #[inline]
        fn try_set(&mut self, v: Value<'b>) -> bool {
            if self.seen {
                // Short-circuit if we've already set the property
                return true;
            }

            self.seen = true;

            if let Some(val) = v.cast() {
                *self.slot = Some(val);

                true
            } else {
                *self.slot = None;

                false
            }
        }

        #[inline]
        fn is_inline(&self) -> bool {
            self.seen && self.slot.is_some()
        }
    }

    struct RestSetter<'a> {
        slot: &'a mut Option<Arc<HashMap<Str<'static>, ThreadLocalValue<OwnedValue>>>>,
        incoming: Option<HashMap<Str<'static>, ThreadLocalValue<OwnedValue>>>,
    }

    impl<'a> RestSetter<'a> {
        #[inline]
        fn new(
            slot: &'a mut Option<Arc<HashMap<Str<'static>, ThreadLocalValue<OwnedValue>>>>,
        ) -> Self {
            RestSetter {
                slot,
                incoming: None,
            }
        }

        #[inline]
        fn get_or_insert(&mut self) -> &mut HashMap<Str<'static>, ThreadLocalValue<OwnedValue>> {
            self.incoming.get_or_insert_with(|| {
                self.slot
                    .take()
                    .map(|slot| (*slot).clone())
                    .unwrap_or_else(HashMap::new)
            })
        }

        #[inline]
        fn set(self) {
            *self.slot = self
                .incoming
                .map(|incoming| Arc::new(incoming))
                .or_else(|| self.slot.take())
        }
    }

    let mut trace_id = InlineSetter::new(&mut span.trace_id);
    let mut span_id = InlineSetter::new(&mut span.span_id);
    let mut span_parent = InlineSetter::new(&mut span.span_parent);
    let mut incoming = RestSetter::new(&mut span.rest);

    let _ = props.for_each(|k, v| {
        let mut fallback = false;

        match k.get() {
            // Set specialized properties inline
            well_known::KEY_TRACE_ID => {
                if trace_id.try_set(v.by_ref()) {
                    return ControlFlow::Continue(());
                } else {
                    fallback = true;
                }
            }
            well_known::KEY_SPAN_ID => {
                if span_id.try_set(v.by_ref()) {
                    return ControlFlow::Continue(());
                } else {
                    fallback = true;
                }
            }
            well_known::KEY_SPAN_PARENT => {
                if span_parent.try_set(v.by_ref()) {
                    return ControlFlow::Continue(());
                } else {
                    fallback = true;
                }
            }
            // Fall-through to buffering in `rest`
            _ => (),
        }

        // NOTE: We only construct a map if we actually need one, but unconditionally clone the parent
        // because we'll never hold the only strong reference to the `Arc`
        let map = incoming.get_or_insert();

        match map.entry(k.to_shared()) {
            hash_map::Entry::Vacant(entry) => {
                entry.insert(ThreadLocalValue::from_value(span.generation, v));
                span.any_fallback |= fallback;
            }
            hash_map::Entry::Occupied(mut entry) => {
                let entry = entry.get_mut();

                // If the existing entry is for a previous generation then overwrite it
                if entry.generation != span.generation {
                    *entry = ThreadLocalValue::from_value(span.generation, v);
                    span.any_fallback |= fallback;
                }
            }
        }

        ControlFlow::Continue(())
    });

    // Unset any inline properties in the fallback set
    if span.any_fallback {
        let map = incoming.get_or_insert();

        let mut fallback = false;
        map.retain(|k, _| match k.get() {
            well_known::KEY_TRACE_ID => {
                if !trace_id.is_inline() {
                    fallback = true;
                    true
                } else {
                    false
                }
            }
            well_known::KEY_SPAN_ID => {
                if !span_id.is_inline() {
                    fallback = true;
                    true
                } else {
                    false
                }
            }
            well_known::KEY_SPAN_PARENT => {
                if !span_parent.is_inline() {
                    fallback = true;
                    true
                } else {
                    false
                }
            }
            _ => true,
        });

        span.any_fallback = fallback;
    }

    incoming.set();
}

// Start this id from 1 so it doesn't intersect with the `shared` variant below
static NEXT_CTXT_ID: Mutex<usize> = Mutex::new(1);

fn ctxt_id() -> usize {
    let mut next_id = NEXT_CTXT_ID.lock().unwrap();
    let id = *next_id;
    *next_id = id.wrapping_add(1);

    id
}

thread_local! {
    // Specialize the shared context to avoid unnecessary map lookups
    static SHARED: RefCell<ThreadLocalCtxtFrame> = RefCell::new(ThreadLocalCtxtFrame::default());
    static ACTIVE: RefCell<HashMap<usize, ThreadLocalCtxtFrame>> = RefCell::new(HashMap::new());
}

fn current(id: usize) -> ThreadLocalCtxtFrame {
    if id == 0 {
        SHARED.with(|f| f.borrow().clone())
    } else {
        ACTIVE.with(|active| {
            active
                .borrow_mut()
                .entry(id)
                .or_insert_with(ThreadLocalCtxtFrame::default)
                .clone()
        })
    }
}

fn swap(id: usize, incoming: &mut ThreadLocalCtxtFrame) {
    if id == 0 {
        SHARED.with(|f| mem::swap(&mut *f.borrow_mut(), incoming));
    } else {
        ACTIVE.with(|active| {
            let mut active = active.borrow_mut();

            let current = active
                .entry(id)
                .or_insert_with(ThreadLocalCtxtFrame::default);

            mem::swap(current, incoming);
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(all(
        target_arch = "wasm32",
        target_vendor = "unknown",
        target_os = "unknown"
    ))]
    use wasm_bindgen_test::*;

    impl ThreadLocalCtxtFrame {
        fn rest_props(&self) -> HashMap<Str<'static>, ThreadLocalValue<OwnedValue>> {
            self.rest
                .as_ref()
                .map(|rest| (**rest).clone())
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
    #[cfg_attr(
        all(
            target_arch = "wasm32",
            target_vendor = "unknown",
            target_os = "unknown"
        ),
        wasm_bindgen_test
    )]
    fn push_props() {
        let ctxt = ThreadLocalCtxt::new();

        ctxt.clone().with_current(|props| {
            assert_eq!(0, props.rest_props().len());
        });

        let mut frame = ctxt.clone().open_push(("a", 1));

        assert_eq!(1, frame.rest_props().len());
        ctxt.clone().with_current(|props| {
            assert_eq!(0, props.rest_props().len());
        });

        ctxt.clone().enter(&mut frame);

        assert_eq!(0, frame.rest_props().len());

        let mut inner = ctxt.clone().open_push([("b", 1), ("a", 2)]);

        ctxt.clone().with_current(|props| {
            let props = props.rest_props();

            assert_eq!(1, props.len());
            assert_eq!(1, props["a"].to_value().cast::<i32>().unwrap());
        });

        ctxt.clone().enter(&mut inner);

        ctxt.clone().with_current(|props| {
            let props = props.rest_props();

            assert_eq!(2, props.len());
            assert_eq!(2, props["a"].to_value().cast::<i32>().unwrap());
            assert_eq!(1, props["b"].to_value().cast::<i32>().unwrap());
        });

        ctxt.clone().exit(&mut inner);

        ctxt.clone().exit(&mut frame);

        assert_eq!(1, frame.rest_props().len());
        ctxt.clone().with_current(|props| {
            assert_eq!(0, props.rest_props().len());
        });
    }

    #[test]
    fn dedup() {
        let ctxt = ThreadLocalCtxt::new();

        ctxt.clone().with_current(|props| {
            assert_eq!(0, props.rest_props().len());
        });

        let mut frame = ctxt.clone().open_push([("a", 1), ("a", 2)]);

        assert_eq!("1", frame.rest_props()["a"].to_value().to_string());

        ctxt.enter(&mut frame);

        let inner = ctxt.clone().open_push([("a", 2), ("a", 3)]);

        assert_eq!("2", inner.rest_props()["a"].to_value().to_string());

        ctxt.exit(&mut frame);
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
            assert_eq!(1, props.rest_props().len());
        });

        ctxt_b.with_current(|props| {
            assert_eq!(0, props.rest_props().len());
        });

        ctxt_a.exit(&mut frame_a);
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn frame_thread_propagation() {
        use std::thread;

        let ctxt = ThreadLocalCtxt::new();

        let mut frame = ctxt.open_push(("a", 1));

        ctxt.enter(&mut frame);

        thread::spawn({
            let ctxt = ctxt.clone();

            move || {
                ctxt.with_current(|props| {
                    assert_eq!(0, props.rest_props().len());
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
                    assert_eq!(1, props.rest_props().len());
                });

                ctxt.exit(&mut current);
            }
        })
        .join()
        .unwrap();
    }

    #[test]
    fn inline_props() {
        use emit_core::props::Props as _;

        let ctxt = ThreadLocalCtxt::new();

        let mut frame = ctxt.clone().open_push(
            (well_known::KEY_TRACE_ID, TraceId::from_u128(42).unwrap())
                .and_props((well_known::KEY_SPAN_ID, SpanId::from_u64(1).unwrap()))
                .and_props(("a", 1)),
        );

        assert_eq!(1, frame.rest_props().len());
        assert_eq!(Some(TraceId::from_u128(42).unwrap()), frame.trace_id);
        assert_eq!(Some(SpanId::from_u64(1).unwrap()), frame.span_id);

        assert_eq!(
            Some(TraceId::from_u128(42).unwrap()),
            frame.pull::<TraceId, _>(well_known::KEY_TRACE_ID)
        );
        assert_eq!(
            Some(SpanId::from_u64(1).unwrap()),
            frame.pull::<SpanId, _>(well_known::KEY_SPAN_ID)
        );
        assert_eq!(Some(1), frame.pull::<i32, _>("a"));

        assert!(!frame.any_fallback);

        // Add more props
        ctxt.enter(&mut frame);
        {
            let frame = ctxt.clone().open_push(
                (well_known::KEY_TRACE_ID, TraceId::from_u128(43).unwrap())
                    .and_props((well_known::KEY_SPAN_PARENT, SpanId::from_u64(1).unwrap()))
                    .and_props(("b", 2)),
            );

            assert_eq!(2, frame.rest_props().len());
            assert_eq!(Some(1), frame.pull::<i32, _>("a"));
            assert_eq!(Some(2), frame.pull::<i32, _>("b"));

            assert_eq!(Some(TraceId::from_u128(43).unwrap()), frame.trace_id);
            assert_eq!(Some(SpanId::from_u64(1).unwrap()), frame.span_parent);
            assert_eq!(Some(SpanId::from_u64(1).unwrap()), frame.span_id);

            assert!(!frame.any_fallback);
        }
        ctxt.exit(&mut frame);

        // Replace an inline prop with a fallback
        ctxt.enter(&mut frame);
        {
            let mut frame = ctxt.clone().open_push(
                (well_known::KEY_TRACE_ID, true)
                    .and_props((well_known::KEY_SPAN_PARENT, SpanId::from_u64(1).unwrap())),
            );

            assert_eq!(2, frame.rest_props().len());
            assert_eq!(Some(1), frame.pull::<i32, _>("a"));
            assert_eq!(Some(true), frame.pull::<bool, _>(well_known::KEY_TRACE_ID));

            assert_eq!(None, frame.trace_id);
            assert_eq!(Some(SpanId::from_u64(1).unwrap()), frame.span_parent);

            assert!(frame.any_fallback);

            // Replace a fallback prop with an inline
            ctxt.enter(&mut frame);
            {
                let frame = ctxt.clone().open_push(
                    (well_known::KEY_TRACE_ID, TraceId::from_u128(43).unwrap())
                        .and_props((well_known::KEY_SPAN_ID, SpanId::from_u64(2).unwrap())),
                );

                assert_eq!(1, frame.rest_props().len());
                assert_eq!(Some(1), frame.pull::<i32, _>("a"));

                assert_eq!(Some(TraceId::from_u128(43).unwrap()), frame.trace_id);
                assert_eq!(Some(SpanId::from_u64(1).unwrap()), frame.span_parent);
                assert_eq!(Some(SpanId::from_u64(2).unwrap()), frame.span_id);

                assert!(!frame.any_fallback);
            }
            ctxt.exit(&mut frame);
        }
        ctxt.exit(&mut frame);
    }
}
