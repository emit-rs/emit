use std::marker::PhantomData;

use sval_derive::Value;

use crate::data::{
    stream_attributes, stream_field, AnyValue, EmitValue, KeyValue, Stacktrace, TextValue,
};

#[derive(Value)]
#[repr(i32)]
#[sval(unlabeled_variants)]
pub enum StatusCode {
    Ok = 1,
    Error = 2,
}

#[derive(Value)]
#[repr(i32)]
#[sval(unlabeled_variants)]
pub enum SpanKind {
    Unspecified = 0,
    Internal = 1,
    Server = 2,
    Client = 3,
    Producer = 4,
    Consumer = 5,
}

impl From<emit::span::SpanKind> for SpanKind {
    fn from(kind: emit::span::SpanKind) -> SpanKind {
        match kind {
            emit::span::SpanKind::Internal => SpanKind::Internal,
            emit::span::SpanKind::Server => SpanKind::Server,
            emit::span::SpanKind::Client => SpanKind::Client,
            emit::span::SpanKind::Producer => SpanKind::Producer,
            emit::span::SpanKind::Consumer => SpanKind::Consumer,
            _ => SpanKind::Unspecified,
        }
    }
}

#[derive(Value)]
pub struct Span<'a, N: ?Sized = str, A: ?Sized = InlineSpanAttributes<'a>> {
    #[sval(label = "name", index = 5)]
    pub name: &'a N,
    #[sval(label = "kind", index = 6)]
    pub kind: SpanKind,
    #[sval(
        label = "startTimeUnixNano",
        index = 7,
        data_tag = "sval_protobuf::tags::PROTOBUF_I64"
    )]
    pub start_time_unix_nano: u64,
    #[sval(
        label = "endTimeUnixNano",
        index = 8,
        data_tag = "sval_protobuf::tags::PROTOBUF_I64"
    )]
    pub end_time_unix_nano: u64,
    #[sval(flatten)]
    pub attributes: &'a A,
}

const SPAN_ATTRIBUTES_LABEL: sval::Label =
    sval::Label::new("attributes").with_tag(&sval::tags::VALUE_IDENT);
const SPAN_TRACE_ID_LABEL: sval::Label =
    sval::Label::new("traceId").with_tag(&sval::tags::VALUE_IDENT);
const SPAN_SPAN_ID_LABEL: sval::Label =
    sval::Label::new("spanId").with_tag(&sval::tags::VALUE_IDENT);
const SPAN_PARENT_SPAN_ID_LABEL: sval::Label =
    sval::Label::new("parentSpanId").with_tag(&sval::tags::VALUE_IDENT);
const SPAN_STATUS_LABEL: sval::Label =
    sval::Label::new("status").with_tag(&sval::tags::VALUE_IDENT);

const SPAN_EVENTS_LABEL: sval::Label =
    sval::Label::new("events").with_tag(&sval::tags::VALUE_IDENT);

const SPAN_ATTRIBUTES_INDEX: sval::Index = sval::Index::new(9);
const SPAN_TRACE_ID_INDEX: sval::Index = sval::Index::new(1);
const SPAN_SPAN_ID_INDEX: sval::Index = sval::Index::new(2);
const SPAN_PARENT_SPAN_ID_INDEX: sval::Index = sval::Index::new(4);
const SPAN_STATUS_INDEX: sval::Index = sval::Index::new(15);
const SPAN_EVENTS_INDEX: sval::Index = sval::Index::new(11);

#[derive(Value)]
pub struct InlineSpanAttributes<
    'a,
    T: ?Sized = sval::BinaryArray<16>,
    S: ?Sized = sval::BinaryArray<8>,
    E: ?Sized = [Event<'a>],
> {
    #[sval(label = SPAN_ATTRIBUTES_LABEL, index = SPAN_ATTRIBUTES_INDEX)]
    pub attributes: &'a [KeyValue<&'a str, &'a AnyValue<'a>>],
    #[sval(label = SPAN_TRACE_ID_LABEL, index = SPAN_TRACE_ID_INDEX)]
    pub trace_id: &'a T,
    #[sval(label = SPAN_SPAN_ID_LABEL, index = SPAN_SPAN_ID_INDEX)]
    pub span_id: &'a S,
    #[sval(label = SPAN_PARENT_SPAN_ID_LABEL, index = SPAN_PARENT_SPAN_ID_INDEX)]
    pub parent_span_id: &'a S,
    #[sval(label = SPAN_STATUS_LABEL, index = SPAN_STATUS_INDEX)]
    pub status: Status<'a>,
    #[sval(label = SPAN_EVENTS_LABEL, index = SPAN_EVENTS_INDEX)]
    pub events: &'a E,
}

pub struct PropsSpanAttributes<T, S, P> {
    time_unix_nano: u64,
    props: P,
    _marker: PhantomData<(T, S)>,
}

impl<T, S, P> PropsSpanAttributes<T, S, P> {
    pub fn new(time_unix_nano: u64, props: P) -> Self {
        PropsSpanAttributes {
            time_unix_nano,
            props,
            _marker: PhantomData,
        }
    }
}

impl<
        TR: From<emit::TraceId> + sval::Value,
        SP: From<emit::SpanId> + sval::Value,
        P: emit::props::Props,
    > sval::Value for PropsSpanAttributes<TR, SP, P>
{
    fn stream<'sval, S: sval::Stream<'sval> + ?Sized>(&'sval self, stream: &mut S) -> sval::Result {
        let mut trace_id = None;
        let mut span_id = None;
        let mut parent_span_id = None;
        let mut level = emit::level::Level::default();
        let mut has_err = false;

        stream.record_tuple_begin(None, None, None, None)?;

        stream_field(
            &mut *stream,
            &SPAN_ATTRIBUTES_LABEL,
            &SPAN_ATTRIBUTES_INDEX,
            |stream| {
                stream_attributes(stream, &self.props, |mut stream, k, v| match k.get() {
                    // Well-known fields
                    emit::well_known::KEY_LVL => {
                        level = v.by_ref().cast().unwrap_or_default();
                        Ok(())
                    }
                    emit::well_known::KEY_SPAN_ID => {
                        span_id = v
                            .by_ref()
                            .cast::<emit::SpanId>()
                            .map(|span_id| SP::from(span_id));
                        Ok(())
                    }
                    emit::well_known::KEY_SPAN_PARENT => {
                        parent_span_id = v
                            .by_ref()
                            .cast::<emit::SpanId>()
                            .map(|parent_span_id| SP::from(parent_span_id));
                        Ok(())
                    }
                    emit::well_known::KEY_TRACE_ID => {
                        trace_id = v
                            .by_ref()
                            .cast::<emit::TraceId>()
                            .map(|trace_id| TR::from(trace_id));
                        Ok(())
                    }
                    emit::well_known::KEY_ERR => {
                        has_err = true;
                        Ok(())
                    }
                    // Ignored
                    emit::well_known::KEY_EVT_KIND
                    | emit::well_known::KEY_SPAN_NAME
                    | emit::well_known::KEY_SPAN_KIND => Ok(()),
                    // Regular attributes
                    _ => stream.stream_attribute(k, v),
                })
            },
        )?;

        if let Some(trace_id) = trace_id {
            stream_field(
                &mut *stream,
                &SPAN_TRACE_ID_LABEL,
                &SPAN_TRACE_ID_INDEX,
                |stream| stream.value_computed(&trace_id),
            )?;
        }

        if let Some(span_id) = span_id {
            stream_field(
                &mut *stream,
                &SPAN_SPAN_ID_LABEL,
                &SPAN_SPAN_ID_INDEX,
                |stream| stream.value_computed(&span_id),
            )?;
        }

        if let Some(parent_span_id) = parent_span_id {
            stream_field(
                &mut *stream,
                &SPAN_PARENT_SPAN_ID_LABEL,
                &SPAN_PARENT_SPAN_ID_INDEX,
                |stream| stream.value_computed(&parent_span_id),
            )?;
        }

        // If the span has an error on it then set the conventional error event
        // along with the span status
        if has_err {
            let err = self.props.get(emit::well_known::KEY_ERR).unwrap();

            stream_field(
                &mut *stream,
                &SPAN_EVENTS_LABEL,
                &SPAN_EVENTS_INDEX,
                |stream| {
                    let err = err.by_ref();

                    // If the error has a cause chain then write it into the exception.stacktrace attribute
                    // We need to duplicate the whole event because the type of its attributes collection
                    // changes depending on whether there's a stacktrace or not
                    if let Some(cause) = err.to_borrowed_error().and_then(|err| err.source()) {
                        stream.value_computed(&[Event {
                            name: "exception",
                            time_unix_nano: self.time_unix_nano,
                            attributes: &InlineEventAttributes {
                                attributes: &[
                                    KeyValue {
                                        key: "exception.stacktrace",
                                        value: &TextValue(Stacktrace::new_borrowed(cause))
                                            as &dyn sval_dynamic::Value,
                                    },
                                    KeyValue {
                                        key: "exception.message",
                                        value: &EmitValue(err) as &dyn sval_dynamic::Value,
                                    },
                                ],
                            },
                        }])
                    } else {
                        stream.value_computed(&[Event {
                            name: "exception",
                            time_unix_nano: self.time_unix_nano,
                            attributes: &InlineEventAttributes {
                                attributes: &[KeyValue {
                                    key: "exception.message",
                                    value: EmitValue(err),
                                }],
                            },
                        }])
                    }
                },
            )?;

            let status = Status {
                code: StatusCode::Error,
                message: sval::Display::new_borrowed(&err),
            };

            stream_field(
                &mut *stream,
                &SPAN_STATUS_LABEL,
                &SPAN_STATUS_INDEX,
                |stream| stream.value_computed(&status),
            )?;
        }
        // If the span doesn't have an error then use the level to determine
        // the span status
        else {
            let code = match level {
                emit::Level::Debug | emit::Level::Info => StatusCode::Ok,
                emit::Level::Warn | emit::Level::Error => StatusCode::Error,
            };

            let status = Status {
                code,
                message: sval::Display::new_borrowed(&level),
            };

            stream_field(
                &mut *stream,
                &SPAN_STATUS_LABEL,
                &SPAN_STATUS_INDEX,
                |stream| stream.value_computed(&status),
            )?;
        }

        stream.record_tuple_end(None, None, None)
    }
}

#[derive(Value)]
pub struct Status<'a, M: ?Sized = str> {
    #[sval(label = "message", index = 2)]
    pub message: &'a M,
    #[sval(label = "code", index = 3)]
    pub code: StatusCode,
}

#[derive(Value)]
pub struct Event<'a, N: ?Sized = str, A: ?Sized = InlineEventAttributes<'a>> {
    #[sval(label = "name", index = 2)]
    pub name: &'a N,
    #[sval(
        label = "timeUnixNano",
        index = 1,
        data_tag = "sval_protobuf::tags::PROTOBUF_I64"
    )]
    pub time_unix_nano: u64,
    #[sval(flatten)]
    pub attributes: &'a A,
}

const EVENT_ATTRIBUTES_LABEL: sval::Label =
    sval::Label::new("attributes").with_tag(&sval::tags::VALUE_IDENT);

const EVENT_ATTRIBUTES_INDEX: sval::Index = sval::Index::new(3);

#[derive(Value)]
pub struct InlineEventAttributes<'a, A: ?Sized = [KeyValue<&'a str, &'a AnyValue<'a>>]> {
    #[sval(label = EVENT_ATTRIBUTES_LABEL, index = EVENT_ATTRIBUTES_INDEX)]
    pub attributes: &'a A,
}
