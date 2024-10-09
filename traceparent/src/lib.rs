/*!
Distributed trace context for `emit` with support for sampling.
*/

use std::{cell::RefCell, fmt, mem, str::FromStr};

use emit::{
    event::ToEvent,
    span::{SpanCtxt, SpanId, TraceId},
    well_known::{KEY_SPAN_ID, KEY_TRACE_ID},
    Ctxt, Filter, Props,
};

thread_local! {
    static ACTIVE_TRACEPARENT: RefCell<Option<Traceparent>> = RefCell::new(None);
}

// TODO: Need a guard that unsets on Drop
pub fn set_traceparent(traceparent: Option<Traceparent>) -> Option<Traceparent> {
    ACTIVE_TRACEPARENT.with(|slot| {
        let mut slot = slot.borrow_mut();

        mem::replace(&mut *slot, traceparent)
    })
}

pub fn get_traceparent() -> Option<Traceparent> {
    ACTIVE_TRACEPARENT.with(|slot| *slot.borrow())
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

#[derive(Debug, Clone, Copy)]
pub struct Traceparent {
    trace_id: Option<TraceId>,
    span_id: Option<SpanId>,
    trace_flags: TraceFlags,
}

#[derive(Debug, Clone, Copy)]
pub struct TraceFlags(u8);

impl TraceFlags {
    pub const EMPTY: Self = TraceFlags(0);
    pub const SAMPLED: Self = TraceFlags(1);

    pub fn try_from_str(flags: &str) -> Result<Self, Error> {
        match flags {
            "00" => Ok(TraceFlags::EMPTY),
            "01" => Ok(TraceFlags::SAMPLED),
            _ => Err(Error {
                msg: format!("unexpected flags {flags:?}"),
            }),
        }
    }

    pub fn from_u8(raw: u8) -> Self {
        TraceFlags(raw)
    }

    pub fn to_u8(&self) -> u8 {
        self.0
    }

    pub fn is_sampled(&self) -> bool {
        self.0 & Self::SAMPLED.0 == 0
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
        todo!()
    }
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

        let trace_id = TraceId::try_from_hex(trace_id).map_err(|e| Error { msg: e.to_string() })?;
        let span_id = SpanId::try_from_hex(span_id).map_err(|e| Error { msg: e.to_string() })?;
        let trace_flags = TraceFlags::try_from_str(trace_flags)?;

        Ok(Traceparent::new(Some(trace_id), Some(span_id), trace_flags))
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
        todo!()
    }
}

// Intercept calls to `open` and look for `trace_id` and `span_id`
// Push then to the trace context; that means getting the current
// and using its sampled flag if present
pub struct TraceparentCtxt<C> {
    inner: C,
}

pub struct TraceparentCtxtFrame<F> {
    inner: F,
    active: bool,
    slot: Option<Traceparent>,
}

impl<C: Ctxt> Ctxt for TraceparentCtxt<C> {
    type Current = C::Current;
    type Frame = TraceparentCtxtFrame<C::Frame>;

    fn with_current<R, F: FnOnce(&Self::Current) -> R>(&self, with: F) -> R {
        self.inner.with_current(with)
    }

    fn open_root<P: Props>(&self, props: P) -> Self::Frame {
        let slot = incoming_traceparent(&props);

        let inner = self.inner.open_root(props);

        TraceparentCtxtFrame {
            inner,
            slot,
            active: slot.is_some(),
        }
    }

    fn open_push<P: Props>(&self, props: P) -> Self::Frame {
        let slot = incoming_traceparent(&props);

        let inner = self.inner.open_push(props);

        TraceparentCtxtFrame {
            inner,
            slot,
            active: slot.is_some(),
        }
    }

    fn enter(&self, frame: &mut Self::Frame) {
        if frame.active {
            frame.slot = set_traceparent(frame.slot);
        }

        self.inner.enter(&mut frame.inner)
    }

    fn exit(&self, frame: &mut Self::Frame) {
        if frame.active {
            frame.slot = set_traceparent(frame.slot);
        }

        self.inner.exit(&mut frame.inner)
    }

    fn close(&self, frame: Self::Frame) {
        self.inner.close(frame.inner)
    }
}

fn incoming_traceparent(props: impl Props) -> Option<Traceparent> {
    let trace_id = props.pull::<TraceId, _>(KEY_TRACE_ID);
    let span_id = props.pull::<SpanId, _>(KEY_SPAN_ID);

    if let Some(span_id) = span_id {
        let current_traceparent = get_traceparent();

        let traceparent = Traceparent::new(
            trace_id
                .or_else(|| current_traceparent.and_then(|current| current.trace_id().copied())),
            Some(span_id),
            current_traceparent
                .map(|current| *current.trace_flags())
                .unwrap_or(TraceFlags::SAMPLED),
        );

        Some(traceparent)
    } else {
        None
    }
}

pub struct TraceparentFilter {}

impl Filter for TraceparentFilter {
    fn matches<E: ToEvent>(&self, _: E) -> bool {
        let Some(traceparent) = get_traceparent() else {
            return true;
        };

        traceparent.trace_flags().is_sampled()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn traceparent_ctxt() {
        todo!()
    }

    #[test]
    fn traceparent_ctxt_across_threads() {
        todo!()
    }
}
