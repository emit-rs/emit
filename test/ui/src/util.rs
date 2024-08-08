use emit::{Clock, Ctxt, Event, Props, Rng, Str, Timestamp, Value};

use emit::props::ErasedProps;
use emit::runtime::Runtime;
use std::{
    cell::RefCell,
    cmp,
    collections::HashMap,
    mem,
    ops::ControlFlow,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

pub type SimpleRuntime =
    Runtime<emit::emitter::FromFn, emit::filter::FromFn, SimpleCtxt, CountingClock, CountingRng>;

pub const fn simple_runtime(
    emitter: fn(&Event<&dyn ErasedProps>),
    filter: fn(&Event<&dyn ErasedProps>) -> bool,
) -> SimpleRuntime {
    Runtime::build(
        emit::emitter::FromFn::new(emitter),
        emit::filter::FromFn::new(filter),
        SimpleCtxt::new(),
        CountingClock::new(),
        CountingRng::new(),
    )
}

#[derive(Clone)]
pub(crate) struct Called(Arc<Mutex<usize>>);

impl Called {
    pub(crate) fn new() -> Self {
        Called(Arc::new(Mutex::new(0)))
    }

    pub(crate) fn record(&self) {
        *self.0.lock().unwrap() += 1;
    }

    pub(crate) fn called_times(&self) -> usize {
        *self.0.lock().unwrap()
    }

    pub(crate) fn was_called(&self) -> bool {
        self.called_times() > 0
    }
}

pub struct CountingClock(AtomicU64);

impl CountingClock {
    pub const fn new() -> Self {
        CountingClock(AtomicU64::new(0))
    }
}

impl Clock for CountingClock {
    fn now(&self) -> Option<Timestamp> {
        Timestamp::from_unix(Duration::from_secs(self.0.fetch_add(1, Ordering::Relaxed)))
    }
}

pub struct CountingRng(AtomicU64);

impl CountingRng {
    pub const fn new() -> Self {
        CountingRng(AtomicU64::new(0))
    }
}

impl Rng for CountingRng {
    fn fill<A: AsMut<[u8]>>(&self, mut arr: A) -> Option<A> {
        let mut buf = arr.as_mut();

        while buf.len() > 0 {
            let v = self.0.fetch_add(1, Ordering::Relaxed).to_le_bytes();

            let len = cmp::min(buf.len(), v.len());

            buf[..len].copy_from_slice(&v[..len]);

            buf = &mut buf[len..];
        }

        Some(arr)
    }
}

pub struct SimpleCtxt {}

thread_local! {
    static SIMPLE_CTXT_CURRENT: RefCell<SimpleCtxtProps> = RefCell::new(SimpleCtxtProps(HashMap::new()));
}

impl SimpleCtxt {
    pub const fn new() -> Self {
        SimpleCtxt {}
    }

    fn current(&self) -> SimpleCtxtProps {
        SIMPLE_CTXT_CURRENT.with(|current| current.borrow().clone())
    }

    fn swap(&self, incoming: &mut SimpleCtxtProps) {
        SIMPLE_CTXT_CURRENT.with(|current| mem::swap(&mut *current.borrow_mut(), incoming))
    }
}

pub struct SimpleCtxtFrame(SimpleCtxtProps);

#[derive(Clone)]
pub struct SimpleCtxtProps(HashMap<String, String>);

impl Props for SimpleCtxtProps {
    fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        mut for_each: F,
    ) -> ControlFlow<()> {
        for (key, value) in &self.0 {
            for_each(Str::new_ref(key), Value::from(&**value))?;
        }

        ControlFlow::Continue(())
    }
}

impl Ctxt for SimpleCtxt {
    type Current = SimpleCtxtProps;
    type Frame = SimpleCtxtFrame;

    fn open_root<P: Props>(&self, props: P) -> Self::Frame {
        let mut serialized = HashMap::new();

        props.for_each(|k, v| {
            serialized.insert(k.get().into(), v.to_string());
            ControlFlow::Continue(())
        });

        SimpleCtxtFrame(SimpleCtxtProps(serialized))
    }

    fn enter(&self, local: &mut Self::Frame) {
        self.swap(&mut local.0);
    }

    fn with_current<R, F: FnOnce(&Self::Current) -> R>(&self, with: F) -> R {
        with(&self.current())
    }

    fn exit(&self, local: &mut Self::Frame) {
        self.swap(&mut local.0)
    }

    fn close(&self, _: Self::Frame) {}
}
