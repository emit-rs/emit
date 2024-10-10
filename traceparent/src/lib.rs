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
    well_known::{KEY_SPAN_ID, KEY_TRACE_ID},
    Ctxt, Empty, Filter, Frame, Props, Str, Value,
};

/**
Get the current trace context.
*/
pub fn current() -> Option<Traceparent> {
    get_traceparent_internal()
}

/**
Get a [`Frame`] that can set the current trace context in a scope.

While the frame is active, [`current`] will return the value of `traceparent`.
*/
pub fn set(traceparent: Option<Traceparent>) -> Frame<TraceparentCtxt> {
    let mut frame = Frame::current(TraceparentCtxt::new(Empty));

    frame.inner_mut().slot = traceparent;
    frame.inner_mut().active = true;

    frame
}

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

    pub fn from_span_ctxt(span_ctxt: SpanCtxt) -> Self {
        Traceparent::new(
            span_ctxt.trace_id().copied(),
            span_ctxt.span_id().copied(),
            TraceFlags::SAMPLED,
        )
    }

    pub fn to_span_ctxt(&self) -> Option<SpanCtxt> {
        if self.trace_flags.is_sampled() {
            // TODO: Should the parent come from somewhere?
            Some(SpanCtxt::new(self.trace_id, None, self.span_id))
        } else {
            None
        }
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

thread_local! {
    static ACTIVE_TRACEPARENT: RefCell<Option<Traceparent>> = RefCell::new(None);
}

fn set_traceparent_internal(traceparent: Option<Traceparent>) -> Option<Traceparent> {
    ACTIVE_TRACEPARENT.with(|slot| {
        let mut slot = slot.borrow_mut();

        mem::replace(&mut *slot, traceparent)
    })
}

fn get_traceparent_internal() -> Option<Traceparent> {
    ACTIVE_TRACEPARENT.with(|slot| *slot.borrow())
}

/**
A [`Ctxt`] that synchronizes [`Traceparent`]s with an underlying ambient context.

The trace context is shared by all instances of `TraceparentCtxt`, and any calls to [`current`] and [`set`].
*/
pub struct TraceparentCtxt<C = Empty> {
    inner: C,
}

pub struct TraceparentCtxtFrame<F = Empty> {
    inner: F,
    active: bool,
    slot: Option<Traceparent>,
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
        unsafe { &*self.inner }.for_each(|k, v| {
            // TODO: Should we also consider the parent span id?
            if k != emit::well_known::KEY_TRACE_ID && k != emit::well_known::KEY_SPAN_ID {
                for_each(k, v)?;
            }

            ControlFlow::Continue(())
        })
    }
}

impl<C> TraceparentCtxt<C> {
    pub fn new(inner: C) -> Self {
        TraceparentCtxt { inner }
    }
}

impl<C: Ctxt> Ctxt for TraceparentCtxt<C> {
    type Current = TraceparentCtxtProps<C::Current>;
    type Frame = TraceparentCtxtFrame<C::Frame>;

    fn with_current<R, F: FnOnce(&Self::Current) -> R>(&self, with: F) -> R {
        // Get the current span context
        let traceparent = get_traceparent_internal();

        let ctxt = traceparent
            .and_then(|traceparent| traceparent.to_span_ctxt())
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

    fn enter(&self, frame: &mut Self::Frame) {
        if frame.active {
            frame.slot = set_traceparent_internal(frame.slot);
        }

        self.inner.enter(&mut frame.inner)
    }

    fn exit(&self, frame: &mut Self::Frame) {
        if frame.active {
            frame.slot = set_traceparent_internal(frame.slot);
        }

        self.inner.exit(&mut frame.inner)
    }

    fn close(&self, frame: Self::Frame) {
        self.inner.close(frame.inner)
    }
}

fn incoming_traceparent(props: impl Props) -> (Option<Traceparent>, impl Props) {
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

    let current_traceparent = get_traceparent_internal();

    // Only consider an incoming traceparent if it's different
    if Some(span_id) != current_traceparent.and_then(|traceparent| traceparent.span_id().copied()) {
        return (
            None,
            ExcludeTraceparentProps {
                check: false,
                inner: props,
            },
        );
    }

    let traceparent = Traceparent::new(
        trace_id.or_else(|| current_traceparent.and_then(|current| current.trace_id().copied())),
        Some(span_id),
        current_traceparent
            .map(|current| *current.trace_flags())
            .unwrap_or(TraceFlags::SAMPLED),
    );

    (
        Some(traceparent),
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

        self.inner.for_each(|str, value| match str.get() {
            KEY_TRACE_ID | KEY_SPAN_ID => ControlFlow::Continue(()),
            _ => for_each(str, value),
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
        let Some(traceparent) = current() else {
            return true;
        };

        traceparent.trace_flags().is_sampled()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn traceparent_to_span_ctxt() {
        todo!()
    }

    #[test]
    fn traceparent_set_current() {
        let rng = emit::platform::rand_rng::RandRng::new();

        assert_eq!(None, current());

        let traceparent = Traceparent::new(
            TraceId::random(&rng),
            SpanId::random(&rng),
            TraceFlags::SAMPLED,
        );

        let frame = set(Some(traceparent));

        assert_eq!(None, current());

        frame.call(|| {
            assert_eq!(Some(traceparent), current());

            // Any `TraceparentCtxt` should observe the current trace context
            let emit_ctxt = SpanCtxt::current(TraceparentCtxt::new(Empty));

            assert_eq!(traceparent.trace_id(), emit_ctxt.trace_id());
            assert!(emit_ctxt.span_parent().is_none());
            assert_eq!(traceparent.span_id(), emit_ctxt.span_id());
        });

        assert_eq!(None, current());
    }

    #[test]
    fn traceparent_ctxt() {
        todo!()
    }

    #[test]
    fn traceparent_ctxt_across_threads() {
        todo!()
    }
}
