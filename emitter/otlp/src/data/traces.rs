mod export_trace_service;
mod span;

use emit::{well_known::KEY_SPAN_NAME, Filter, Props};

use crate::Error;

pub use self::{export_trace_service::*, span::*};

use super::{
    stream_encoded_scope_items, EncodedEvent, EncodedPayload, EncodedScopeItems, EventEncoder,
    InstrumentationScope, MessageFormatter, MessageRenderer, RawEncoder, RequestEncoder,
};

pub(crate) struct TracesEventEncoder {
    pub name: Box<MessageFormatter>,
}

impl Default for TracesEventEncoder {
    fn default() -> Self {
        TracesEventEncoder {
            name: default_name_formatter(),
        }
    }
}

fn default_name_formatter() -> Box<MessageFormatter> {
    Box::new(|evt, f| {
        if let Some(name) = evt.props().get(KEY_SPAN_NAME) {
            write!(f, "{}", name)
        } else {
            write!(f, "{}", evt.msg())
        }
    })
}

impl EventEncoder for TracesEventEncoder {
    fn encode_event<E: RawEncoder>(
        &self,
        evt: &emit::event::Event<impl emit::props::Props>,
    ) -> Option<EncodedEvent> {
        if !emit::kind::is_span_filter().matches(evt) {
            return None;
        }

        let (start_time_unix_nano, end_time_unix_nano) = evt
            .extent()
            .and_then(|extent| extent.as_range())
            .map(|range| {
                (
                    range.start.to_unix().as_nanos() as u64,
                    range.end.to_unix().as_nanos() as u64,
                )
            })?;

        Some(EncodedEvent {
            scope: evt.mdl().to_owned(),
            payload: E::encode(Span {
                start_time_unix_nano,
                end_time_unix_nano,
                name: &sval::Display::new(MessageRenderer {
                    fmt: &self.name,
                    evt,
                }),
                attributes: &PropsSpanAttributes::<E::TraceId, E::SpanId, _>::new(
                    end_time_unix_nano,
                    evt.props(),
                ),
                kind: SpanKind::Unspecified,
            }),
        })
    }
}

#[derive(Default)]
pub(crate) struct TracesRequestEncoder;

impl RequestEncoder for TracesRequestEncoder {
    fn encode_request<E: RawEncoder>(
        &self,
        resource: Option<&EncodedPayload>,
        items: &EncodedScopeItems,
    ) -> Result<EncodedPayload, Error> {
        Ok(E::encode(ExportTraceServiceRequest {
            resource_spans: &[ResourceSpans {
                resource: &resource,
                scope_spans: &EncodedScopeSpans(items),
            }],
        }))
    }
}

struct EncodedScopeSpans<'a>(&'a EncodedScopeItems);

impl<'a> sval::Value for EncodedScopeSpans<'a> {
    fn stream<'sval, S: sval::Stream<'sval> + ?Sized>(&'sval self, stream: &mut S) -> sval::Result {
        stream_encoded_scope_items(stream, &self.0, |stream, path, spans| {
            stream.value_computed(&ScopeSpans {
                scope: &InstrumentationScope {
                    name: &sval::Display::new(path),
                },
                spans,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use prost::Message;

    use crate::{
        data::{generated::trace::v1 as trace, util::*},
        util::*,
    };

    #[test]
    fn encode_basic() {
        encode_event::<TracesEventEncoder>(
            emit::evt!(
                extent: ts(1)..ts(13),
                "greet {user}",
                user: "test",
                evt_kind: "span",
                span_name: "test",
                trace_id: "00000000000000000000000000000001",
                span_id: "0000000000000001"
            ),
            |buf| {
                let de = trace::Span::decode(buf).unwrap();

                assert_eq!("test", de.name);
            },
        );
    }
}
