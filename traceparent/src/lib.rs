/*!
Distributed trace context for `emit` with support for sampling.
*/

pub fn set_traceparent(traceparent: Traceparent) {
    todo!()
}

pub fn get_traceparent() -> Option<Traceparent> {
    todo!()
}

/**
An error encountered attempting to work with a [`Traceparent`].
*/
#[derive(Debug)]
pub struct Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the input was not a valid id")
    }
}

impl std::error::Error for Error {}

pub struct Traceparent {
    trace_id: Option<TraceId>,
    span_id: Option<SpanId>,
    trace_flags: TraceFlags,
}

pub struct TraceFlags(u8);

impl TraceFlags {
    pub const EMPTY: Self = TraceFlags(0);
    pub const SAMPLED: Self = TraceFlags(1);

    pub fn try_from_str(flags: &str) -> Result<Self, Error> {
        todo!()
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
        todo!()
    }

    pub fn from_span_ctxt(span_ctxt: SpanCtxt) -> Self {
        Traceparent::new(span_ctxt.trace_id, span_ctxt.span_id, TraceFlags::SAMPLED)
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

impl<C: Ctxt> Ctxt for TraceparentCtxt<C> {
    type Current = C::Current;
    type Frame = C::Frame;
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
}
