mod export_trace_service;
mod span;

use emit::{
    well_known::{KEY_SPAN_KIND, KEY_SPAN_NAME},
    Filter, Props,
};

use crate::Error;

pub use self::{export_trace_service::*, span::*};

use super::{
    stream_encoded_scope_items, EncodedEvent, EncodedPayload, EncodedScopeItems, EventEncoder,
    InstrumentationScope, MessageFormatter, MessageRenderer, RawEncoder, RequestEncoder,
};

pub(crate) struct TracesEventEncoder {
    pub name: Box<MessageFormatter>,
    pub kind: Box<KindExtractor>,
}

impl Default for TracesEventEncoder {
    fn default() -> Self {
        TracesEventEncoder {
            name: default_name_formatter(),
            kind: default_kind_extractor(),
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

type KindExtractor = dyn Fn(&emit::event::Event<&dyn emit::props::ErasedProps>) -> Option<emit::span::SpanKind>
    + Send
    + Sync;

fn default_kind_extractor() -> Box<KindExtractor> {
    Box::new(|evt| evt.props().pull(KEY_SPAN_KIND))
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
                kind: (self.kind)(&evt.erase())
                    .map(Into::into)
                    .unwrap_or(SpanKind::Unspecified),
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
        data::{
            generated::{trace::v1 as trace, util::*},
            util::*,
        },
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
                span_kind: "server",
                trace_id: "00000000000000000000000000000001",
                span_id: "0000000000000001"
            ),
            |buf| {
                let de = trace::Span::decode(buf).unwrap();

                assert_eq!("test", de.name);
                assert_eq!(trace::span::SpanKind::Server as i32, de.kind);

                assert_eq!(0x1u128.to_be_bytes(), &*de.trace_id);
                assert_eq!(0x1u64.to_be_bytes(), &*de.span_id);
            },
        );
    }

    #[test]
    fn encode_ids_structured() {
        encode_event::<TracesEventEncoder>(
            emit::evt!(
                extent: ts(1)..ts(13),
                "greet {user}",
                user: "test",
                evt_kind: "span",
                span_name: "test",
                span_kind: "server",
                trace_id: emit::span::TraceId::from_u128(0x1).unwrap(),
                span_id: emit::span::SpanId::from_u64(0x1).unwrap(),
            ),
            |buf| {
                let de = trace::Span::decode(buf).unwrap();

                assert_eq!("test", de.name);
                assert_eq!(trace::span::SpanKind::Server as i32, de.kind);

                assert_eq!(0x1u128.to_be_bytes(), &*de.trace_id);
                assert_eq!(0x1u64.to_be_bytes(), &*de.span_id);
            },
        );
    }

    #[test]
    fn encode_span_name() {
        let encoder = TracesEventEncoder {
            name: Box::new(|_, f| f.write_str("custom name")),
            kind: default_kind_extractor(),
        };

        encode_event_with(
            encoder,
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

                assert_eq!("custom name", de.name);
            },
        );
    }

    #[test]
    fn encode_span_kind_default() {
        let encoder = TracesEventEncoder {
            name: default_name_formatter(),
            kind: Box::new(|_| Some(emit::span::SpanKind::Client)),
        };

        encode_event_with(
            encoder,
            emit::evt!(
                extent: ts(1)..ts(13),
                "greet {user}",
                user: "test",
                evt_kind: "span",
                trace_id: "00000000000000000000000000000001",
                span_id: "0000000000000001"
            ),
            |buf| {
                let de = trace::Span::decode(buf).unwrap();

                assert_eq!(trace::span::SpanKind::Client as i32, de.kind);
            },
        );
    }

    #[test]
    fn encode_span_links() {
        let encoder = TracesEventEncoder {
            name: default_name_formatter(),
            kind: Box::new(|_| Some(emit::span::SpanKind::Server)),
        };

        encode_event_with(
            encoder,
            emit::evt!(
                extent: ts(1)..ts(13),
                "greet {user}",
                user: "test",
                evt_kind: "span",
                trace_id: "00000000000000000000000000000001",
                span_id: "0000000000000001",
                #[emit::as_sval]
                span_links: [
                    "00000000000000000000000000000001-0000000000000001",
                    "00000000000000000000000000000002-0000000000000002",
                ],
            ),
            |buf| {
                let de = trace::Span::decode(buf).unwrap();

                assert_eq!(2, de.links.len());

                assert_eq!(0x1u128.to_be_bytes(), &*de.links[0].trace_id);
                assert_eq!(0x1u64.to_be_bytes(), &*de.links[0].span_id);

                assert_eq!(0x2u128.to_be_bytes(), &*de.links[1].trace_id);
                assert_eq!(0x2u64.to_be_bytes(), &*de.links[1].span_id);
            },
        );
    }

    #[test]
    fn encode_span_links_structured() {
        let encoder = TracesEventEncoder {
            name: default_name_formatter(),
            kind: Box::new(|_| Some(emit::span::SpanKind::Server)),
        };

        encode_event_with(
            encoder,
            emit::evt!(
                extent: ts(1)..ts(13),
                "greet {user}",
                user: "test",
                evt_kind: "span",
                trace_id: "00000000000000000000000000000001",
                span_id: "0000000000000001",
                #[emit::as_sval]
                span_links: [
                    emit::span::SpanLink::new(
                        emit::span::TraceId::from_u128(0x1).unwrap(),
                        emit::span::SpanId::from_u64(0x1).unwrap(),
                    ),
                    emit::span::SpanLink::new(
                        emit::span::TraceId::from_u128(0x2).unwrap(),
                        emit::span::SpanId::from_u64(0x2).unwrap(),
                    ),
                ],
            ),
            |buf| {
                let de = trace::Span::decode(buf).unwrap();

                assert_eq!(2, de.links.len());

                assert_eq!(0x1u128.to_be_bytes(), &*de.links[0].trace_id);
                assert_eq!(0x1u64.to_be_bytes(), &*de.links[0].span_id);

                assert_eq!(0x2u128.to_be_bytes(), &*de.links[1].trace_id);
                assert_eq!(0x2u64.to_be_bytes(), &*de.links[1].span_id);
            },
        );
    }

    #[test]
    fn encode_err_str() {
        encode_event::<TracesEventEncoder>(
            emit::evt!(
                extent: ts(1)..ts(13),
                "failed: {err}",
                err: "test",
                evt_kind: "span",
                span_name: "test",
                trace_id: "00000000000000000000000000000001",
                span_id: "0000000000000001"
            ),
            |buf| {
                let de = trace::Span::decode(buf).unwrap();

                assert_eq!(1, de.events.len());

                let de = &de.events[0];

                assert_eq!("exception", de.name);

                assert_eq!(1, de.attributes.len());

                assert_eq!("exception.message", de.attributes[0].key);
                assert_eq!(Some(string_value("test")), de.attributes[0].value);
            },
        );
    }

    #[test]
    fn encode_err_stacktrace() {
        #[derive(Debug)]
        struct Error {
            source: Option<std::io::Error>,
        }

        impl std::fmt::Display for Error {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "something went wrong")
            }
        }

        impl std::error::Error for Error {
            fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                self.source
                    .as_ref()
                    .map(|source| source as &(dyn std::error::Error + 'static))
            }
        }

        let err = Error {
            source: Some(std::io::Error::new(std::io::ErrorKind::Other, "IO error")),
        };

        encode_event::<TracesEventEncoder>(
            emit::evt!(
                extent: ts(1)..ts(13),
                "failed: {err}",
                err,
                evt_kind: "span",
                span_name: "test",
                trace_id: "00000000000000000000000000000001",
                span_id: "0000000000000001"
            ),
            |buf| {
                let de = trace::Span::decode(buf).unwrap();

                assert_eq!(1, de.events.len());

                let de = &de.events[0];

                assert_eq!("exception", de.name);

                assert_eq!(2, de.attributes.len());

                assert_eq!("exception.stacktrace", de.attributes[0].key);
                assert_eq!(
                    Some(string_value("caused by: IO error")),
                    de.attributes[0].value
                );

                assert_eq!("exception.message", de.attributes[1].key);
                assert_eq!(
                    Some(string_value("something went wrong")),
                    de.attributes[1].value
                );
            },
        );

        let err = Error { source: None };

        encode_event::<TracesEventEncoder>(
            emit::evt!(
                extent: ts(1)..ts(13),
                "failed: {err}",
                err,
                evt_kind: "span",
                span_name: "test",
                trace_id: "00000000000000000000000000000001",
                span_id: "0000000000000001"
            ),
            |buf| {
                let de = trace::Span::decode(buf).unwrap();

                assert_eq!(1, de.events.len());

                let de = &de.events[0];

                assert_eq!("exception", de.name);

                assert_eq!(1, de.attributes.len());

                assert_eq!("exception.message", de.attributes[0].key);
                assert_eq!(
                    Some(string_value("something went wrong")),
                    de.attributes[0].value
                );
            },
        );
    }

    #[test]
    fn encode_request_basic() {
        todo!()
    }
}
