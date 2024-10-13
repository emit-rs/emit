/*!
Distributed trace context for `emit` with support for sampling.
*/

use std::{
    cell::RefCell,
    fmt::{self, Write as _},
    mem,
    ops::ControlFlow,
    str::FromStr,
};

use emit::{
    event::ToEvent,
    span::{SpanCtxt, SpanId, TraceId},
    well_known::{KEY_SPAN_ID, KEY_SPAN_PARENT, KEY_TRACE_ID},
    Ctxt, Empty, Filter, Frame, Props, Str, Value,
};

/**
An error encountered attempting to work with a [`Traceparent`].
*/
#[derive(Debug)]
pub struct Error {
    msg: String,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.msg, f)
    }
}

impl std::error::Error for Error {}

/**
A [W3C traceparent](https://www.w3.org/TR/trace-context).

This type contains `emit`'s [`TraceId`] and [`SpanId`], along with [`TraceFlags`] that determine sampling.

Traceparents exist at the edges of your application. On incoming requests, it may carry a traceparent header that can be parsed into a `Traceparent` and pushed onto the active trace context with [`set`]. On outgoing requests, the active trace context is pulled by [`get`] and formatted into a traceparent header.
*/
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Traceparent {
    trace_id: Option<TraceId>,
    span_id: Option<SpanId>,
    trace_flags: TraceFlags,
}

impl Traceparent {
    pub const fn new(
        trace_id: Option<TraceId>,
        span_id: Option<SpanId>,
        trace_flags: TraceFlags,
    ) -> Self {
        Traceparent {
            trace_id,
            span_id,
            trace_flags,
        }
    }

    pub const fn empty() -> Self {
        Self {
            trace_id: None,
            span_id: None,
            trace_flags: TraceFlags::EMPTY,
        }
    }

    pub fn try_from_str(header: &str) -> Result<Self, Error> {
        let mut parts = header.split('-');

        let version = parts.next().ok_or_else(|| Error {
            msg: "missing version".into(),
        })?;

        let "00" = version else {
            return Err(Error {
                msg: format!("unexpected version {version:?}. Only version '00' is supported"),
            });
        };

        let trace_id = parts.next().ok_or_else(|| Error {
            msg: "missing trace id".into(),
        })?;
        let span_id = parts.next().ok_or_else(|| Error {
            msg: "missing span id".into(),
        })?;
        let trace_flags = parts.next().ok_or_else(|| Error {
            msg: "missing flags".into(),
        })?;

        let None = parts.next() else {
            return Err(Error {
                msg: format!("traceparent {header:?} is in an invalid format"),
            });
        };

        let trace_id = if trace_id == "00000000000000000000000000000000" {
            None
        } else {
            Some(TraceId::try_from_hex(trace_id).map_err(|e| Error { msg: e.to_string() })?)
        };

        let span_id = if span_id == "0000000000000000" {
            None
        } else {
            Some(SpanId::try_from_hex(span_id).map_err(|e| Error { msg: e.to_string() })?)
        };

        let trace_flags = TraceFlags::try_from_str(trace_flags)?;

        Ok(Traceparent::new(trace_id, span_id, trace_flags))
    }

    pub fn trace_id(&self) -> Option<&TraceId> {
        self.trace_id.as_ref()
    }

    pub fn span_id(&self) -> Option<&SpanId> {
        self.span_id.as_ref()
    }

    pub fn trace_flags(&self) -> &TraceFlags {
        &self.trace_flags
    }

    /**
    Get the current trace context.
    */
    pub fn current() -> Self {
        get_active_traceparent()
            .map(|active| active.traceparent)
            .unwrap_or(Traceparent::empty())
    }

    /**
    Get a [`Frame`] that can set the current trace context in a scope.

    While the frame is active, [`current`] will return the value of `traceparent`.
    */
    pub fn push(&self) -> Frame<TraceparentCtxt> {
        let mut frame = Frame::current(TraceparentCtxt::new(Empty));

        frame.inner_mut().slot = Some(
            get_active_traceparent()
                .map(|active| ActiveTraceparent {
                    traceparent: *self,
                    // If the incoming traceparent is for the same trace is the current
                    // then use the current's span id as the parent id
                    span_parent: if active.is_parent_of(self.trace_id) {
                        active.traceparent.span_id
                    }
                    // If the incoming traceparent is for a different trace then
                    // treat it as a root span (with no parent id)
                    else {
                        None
                    },
                })
                .unwrap_or(ActiveTraceparent {
                    traceparent: *self,
                    span_parent: None,
                }),
        );
        frame.inner_mut().active = true;

        frame
    }

    pub fn is_valid(&self) -> bool {
        self.trace_id.is_some() && self.span_id.is_some()
    }
}

impl FromStr for Traceparent {
    type Err = Error;

    fn from_str(header: &str) -> Result<Self, Error> {
        Self::try_from_str(header)
    }
}

impl fmt::Display for Traceparent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("00-")?;

        if let Some(trace_id) = self.trace_id {
            fmt::Display::fmt(&trace_id, f)?;
            f.write_char('-')?;
        } else {
            f.write_str("00000000000000000000000000000000-")?;
        }

        if let Some(span_id) = self.span_id {
            fmt::Display::fmt(&span_id, f)?;
            f.write_char('-')?;
        } else {
            f.write_str("0000000000000000-")?;
        }

        fmt::Display::fmt(&self.trace_flags, f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TraceFlags(u8);

impl TraceFlags {
    pub const EMPTY: Self = TraceFlags(0);
    pub const SAMPLED: Self = TraceFlags(1);

    pub fn try_from_str(flags: &str) -> Result<Self, Error> {
        if flags.len() != 2 {
            return Err(Error {
                msg: "flags must be a 2 digit value".into(),
            });
        }

        Ok(TraceFlags(
            u8::from_str_radix(flags, 16).map_err(|e| Error { msg: e.to_string() })?,
        ))
    }

    pub fn from_u8(raw: u8) -> Self {
        TraceFlags(raw)
    }

    pub fn to_u8(&self) -> u8 {
        self.0
    }

    pub fn is_sampled(&self) -> bool {
        self.0 & Self::SAMPLED.0 == 1
    }
}

impl FromStr for TraceFlags {
    type Err = Error;

    fn from_str(flags: &str) -> Result<Self, Error> {
        Self::try_from_str(flags)
    }
}

impl fmt::Display for TraceFlags {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::LowerHex::fmt(&self.0, f)
    }
}

#[derive(Debug, Clone, Copy)]
struct ActiveTraceparent {
    traceparent: Traceparent,
    span_parent: Option<SpanId>,
}

impl ActiveTraceparent {
    fn is_parent_of(&self, trace_id: Option<TraceId>) -> bool {
        // A traceparent is a parent of a trace id if:
        // 1. The traceparent has a trace id
        // 2. The traceparent's trace id matches the input trace id
        //
        // That means a traceparent is never considered the parent
        // of an empty trace id

        self.traceparent.trace_id.is_some() && self.traceparent.trace_id == trace_id
    }
}

thread_local! {
    static ACTIVE_TRACEPARENT: RefCell<Option<ActiveTraceparent>> = RefCell::new(None);
}

fn set_active_traceparent(traceparent: Option<ActiveTraceparent>) -> Option<ActiveTraceparent> {
    ACTIVE_TRACEPARENT.with(|slot| {
        let mut slot = slot.borrow_mut();

        mem::replace(&mut *slot, traceparent)
    })
}

fn get_active_traceparent() -> Option<ActiveTraceparent> {
    ACTIVE_TRACEPARENT.with(|slot| *slot.borrow())
}

/**
A [`Ctxt`] that synchronizes [`Traceparent`]s with an underlying ambient context.

The trace context is shared by all instances of `TraceparentCtxt`, and any calls to [`current`] and [`set`].
*/
#[derive(Debug, Clone, Copy)]
pub struct TraceparentCtxt<C = Empty> {
    inner: C,
}

#[derive(Debug, Clone, Copy)]
pub struct TraceparentCtxtFrame<F = Empty> {
    inner: F,
    active: bool,
    slot: Option<ActiveTraceparent>,
}

pub struct TraceparentCtxtProps<P: ?Sized> {
    inner: *const P,
    ctxt: SpanCtxt,
}

impl<P: Props + ?Sized> Props for TraceparentCtxtProps<P> {
    fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        mut for_each: F,
    ) -> ControlFlow<()> {
        self.ctxt.for_each(&mut for_each)?;

        // SAFETY: This type is only exposed for arbitrarily short (`for<'a>`) lifetimes
        // so inner it's guaranteed to be valid for `'kv`, which must be shorter than its
        // original lifetime
        unsafe { &*self.inner }.for_each(for_each)
    }
}

impl<C> TraceparentCtxt<C> {
    pub const fn new(inner: C) -> Self {
        TraceparentCtxt { inner }
    }
}

impl<C: Ctxt> Ctxt for TraceparentCtxt<C> {
    type Current = TraceparentCtxtProps<C::Current>;
    type Frame = TraceparentCtxtFrame<C::Frame>;

    fn with_current<R, F: FnOnce(&Self::Current) -> R>(&self, with: F) -> R {
        // Get the current span context
        let ctxt = get_active_traceparent()
            .and_then(|active| {
                if active.traceparent.trace_flags.is_sampled() {
                    Some(SpanCtxt::new(
                        active.traceparent.trace_id,
                        active.span_parent,
                        active.traceparent.span_id,
                    ))
                } else {
                    None
                }
            })
            .unwrap_or(SpanCtxt::empty());

        self.inner.with_current(|props| {
            let props = TraceparentCtxtProps {
                ctxt,
                inner: props as *const C::Current,
            };

            with(&props)
        })
    }

    fn open_root<P: Props>(&self, props: P) -> Self::Frame {
        let (slot, props) = incoming_traceparent(props);

        let inner = self.inner.open_root(props);

        TraceparentCtxtFrame {
            inner,
            slot,
            active: slot.is_some(),
        }
    }

    fn open_push<P: Props>(&self, props: P) -> Self::Frame {
        let (slot, props) = incoming_traceparent(props);

        let inner = self.inner.open_push(props);

        TraceparentCtxtFrame {
            inner,
            slot,
            active: slot.is_some(),
        }
    }

    fn open_disabled<P: Props>(&self, props: P) -> Self::Frame {
        todo!()
    }

    fn enter(&self, frame: &mut Self::Frame) {
        if frame.active {
            frame.slot = set_active_traceparent(frame.slot);
        }

        self.inner.enter(&mut frame.inner)
    }

    fn exit(&self, frame: &mut Self::Frame) {
        if frame.active {
            frame.slot = set_active_traceparent(frame.slot);
        }

        self.inner.exit(&mut frame.inner)
    }

    fn close(&self, frame: Self::Frame) {
        self.inner.close(frame.inner)
    }
}

fn incoming_traceparent(props: impl Props) -> (Option<ActiveTraceparent>, impl Props) {
    let trace_id = props.pull::<TraceId, _>(KEY_TRACE_ID);
    let span_id = props.pull::<SpanId, _>(KEY_SPAN_ID);

    // Only consider props that carry a span id
    let Some(span_id) = span_id else {
        return (
            None,
            ExcludeTraceparentProps {
                check: false,
                inner: props,
            },
        );
    };

    // Only consider the current traceparent if the trace id matches
    let parent = get_active_traceparent()
        .filter(|active| trace_id.is_none() || active.is_parent_of(trace_id));

    // Only consider an incoming traceparent if the span id has changed
    if Some(span_id) == parent.and_then(|parent| parent.traceparent.span_id) {
        return (
            None,
            ExcludeTraceparentProps {
                check: false,
                inner: props,
            },
        );
    }

    let incoming = if let Some(parent) = parent {
        // The incoming traceparent is a child of the current one
        // Construct a traceparent from it with the same trace id and flags,
        // using the span id of the parent as the parent id of the incoming
        ActiveTraceparent {
            traceparent: Traceparent::new(
                parent.traceparent.trace_id,
                Some(span_id),
                parent.traceparent.trace_flags,
            ),
            span_parent: parent.traceparent.span_id,
        }
    } else {
        // The incoming traceparent is for a root span
        // We consider traces created by `emit` this way to be sampled
        ActiveTraceparent {
            traceparent: Traceparent::new(trace_id, Some(span_id), TraceFlags::SAMPLED),
            span_parent: None,
        }
    };

    (
        Some(incoming),
        ExcludeTraceparentProps {
            check: true,
            inner: props,
        },
    )
}

struct ExcludeTraceparentProps<P> {
    check: bool,
    inner: P,
}

impl<P: Props> Props for ExcludeTraceparentProps<P> {
    fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        mut for_each: F,
    ) -> ControlFlow<()> {
        if !self.check {
            return self.inner.for_each(for_each);
        }

        self.inner.for_each(|key, value| match key.get() {
            // Properties that come from the traceparent context
            KEY_TRACE_ID | KEY_SPAN_ID | KEY_SPAN_PARENT => ControlFlow::Continue(()),
            // Properties to pass through to the underlying context
            _ => for_each(key, value),
        })
    }
}

/**
A filter that excludes events when the current trace context is unsampled.

This filter uses [`current`] and checks [`TraceFlags::is_sampled`] on the returned [`Traceparent`] to determine whether the current context is sampled or not.
*/
pub struct TraceparentFilter {}

impl Filter for TraceparentFilter {
    fn matches<E: ToEvent>(&self, _: E) -> bool {
        let traceparent = Traceparent::current();

        traceparent.is_valid() && traceparent.trace_flags().is_sampled()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::thread;

    #[test]
    fn traceparent_roundtrip() {
        todo!()
    }

    #[test]
    fn traceparent_parse_valid() {
        todo!()
    }

    #[test]
    fn traceparent_parse_invalid() {
        todo!()
    }

    #[test]
    fn traceparent_set_current() {
        let rng = emit::platform::rand_rng::RandRng::new();

        assert_eq!(None, Traceparent::current().trace_id());
        assert_eq!(None, Traceparent::current().span_id());
        assert_eq!(TraceFlags::EMPTY, *Traceparent::current().trace_flags());

        let traceparent = Traceparent::new(
            TraceId::random(&rng),
            SpanId::random(&rng),
            TraceFlags::SAMPLED,
        );

        let frame = traceparent.push();

        assert_eq!(None, Traceparent::current().trace_id());
        assert_eq!(None, Traceparent::current().span_id());
        assert_eq!(TraceFlags::EMPTY, *Traceparent::current().trace_flags());

        frame.call(|| {
            assert_eq!(traceparent, Traceparent::current());

            // Any `TraceparentCtxt` should observe the current trace context
            let emit_ctxt = SpanCtxt::current(TraceparentCtxt::new(Empty));

            assert_eq!(traceparent.trace_id(), emit_ctxt.trace_id());
            assert_eq!(traceparent.span_id(), emit_ctxt.span_id());
            assert!(emit_ctxt.span_parent().is_none());
        });

        assert_eq!(None, Traceparent::current().trace_id());
        assert_eq!(None, Traceparent::current().span_id());
        assert_eq!(TraceFlags::EMPTY, *Traceparent::current().trace_flags());
    }

    #[test]
    fn traceparent_ctxt() {
        let rng = emit::platform::rand_rng::RandRng::new();
        let ctxt = TraceparentCtxt::new(emit::platform::thread_local_ctxt::ThreadLocalCtxt::new());

        let span_ctxt_1 = SpanCtxt::current(&ctxt).new_child(&rng);

        span_ctxt_1.push(&ctxt).call(|| {
            let span_ctxt_2 = SpanCtxt::current(&ctxt).new_child(&rng);

            span_ctxt_2.push(&ctxt).call(|| {
                let traceparent = Traceparent::current();

                let span_ctxt_3 = SpanCtxt::current(&ctxt);

                assert_eq!(span_ctxt_2, span_ctxt_3);

                assert_eq!(
                    span_ctxt_1.trace_id().unwrap(),
                    span_ctxt_2.trace_id().unwrap()
                );
                assert_eq!(
                    span_ctxt_1.span_id().unwrap(),
                    span_ctxt_2.span_parent().unwrap()
                );
                assert!(span_ctxt_2.span_id().is_some());

                assert_eq!(
                    traceparent.trace_id().unwrap(),
                    span_ctxt_2.trace_id().unwrap()
                );
                assert_eq!(
                    traceparent.span_id().unwrap(),
                    span_ctxt_2.span_id().unwrap()
                );
                assert!(traceparent.trace_flags().is_sampled());
            });
        });
    }

    #[test]
    fn traceparent_across_threads() {
        let rng = emit::platform::rand_rng::RandRng::new();

        let traceparent = Traceparent::new(
            TraceId::random(&rng),
            SpanId::random(&rng),
            TraceFlags::SAMPLED,
        );

        let frame = traceparent.push();

        thread::spawn(move || {
            frame.call(|| {
                let current = Traceparent::current();

                assert_eq!(traceparent, current);
            })
        })
        .join()
        .unwrap();
    }

    #[test]
    fn traceparent_ctxt_across_threads() {
        let rng = emit::platform::rand_rng::RandRng::new();
        let ctxt = TraceparentCtxt::new(emit::platform::thread_local_ctxt::ThreadLocalCtxt::new());

        let span_ctxt = SpanCtxt::current(&ctxt).new_child(&rng).push(ctxt.clone());

        thread::spawn(move || {
            span_ctxt.call(|| {
                let traceparent = Traceparent::current();
                let span_ctxt = SpanCtxt::current(&ctxt);

                assert_eq!(span_ctxt.trace_id(), traceparent.trace_id());
                assert_eq!(span_ctxt.span_id(), traceparent.span_id());
                assert_eq!(TraceFlags::SAMPLED, *traceparent.trace_flags());
            })
        })
        .join()
        .unwrap();
    }

    #[test]
    fn traceparent_span() {
        use emit::{
            and::And,
            filter,
            platform::{
                rand_rng::RandRng, system_clock::SystemClock, thread_local_ctxt::ThreadLocalCtxt,
            },
            runtime::Runtime,
            Empty,
        };

        static RT: Runtime<
            Empty,
            And<filter::FromFn, TraceparentFilter>,
            TraceparentCtxt<ThreadLocalCtxt>,
            SystemClock,
            RandRng,
        > = Runtime::build(
            Empty,
            And::new(
                filter::FromFn::new(|evt| evt.mdl() != emit::path!("unsampled")),
                TraceparentFilter {},
            ),
            TraceparentCtxt::new(ThreadLocalCtxt::shared()),
            SystemClock::new(),
            RandRng::new(),
        );

        #[emit::span(rt: RT, mdl: emit::path!("unsampled"), "a")]
        fn unsampled() {
            unsampled_sampled();
        }

        #[emit::span(rt: RT, mdl: emit::path!("sampled"), "a")]
        fn unsampled_sampled() {
            let ctxt = SpanCtxt::current(RT.ctxt());
            let traceparent = Traceparent::current();

            assert!(ctxt.trace_id().is_none());
            assert!(ctxt.span_id().is_none());

            assert!(traceparent.trace_id().is_some());
            assert!(traceparent.span_id().is_some());
            assert!(!traceparent.trace_flags().is_sampled());
        }

        #[emit::span(rt: RT, mdl: emit::path!("sampled"), "a")]
        fn sampled() {
            let ctxt = SpanCtxt::current(RT.ctxt());
            let traceparent = Traceparent::current();

            assert!(ctxt.trace_id().is_some());
            assert!(ctxt.span_id().is_some());

            assert_eq!(traceparent.trace_id(), ctxt.trace_id());
            assert_eq!(traceparent.span_id(), ctxt.span_id());
            assert!(traceparent.trace_flags().is_sampled());
        }

        sampled();
        unsampled();
    }
}
