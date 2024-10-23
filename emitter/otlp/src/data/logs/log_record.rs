use std::marker::PhantomData;

use sval_derive::Value;

use crate::data::{stream_attributes, stream_field, AnyValue, KeyValue, Stacktrace, TextValue};

#[derive(Value)]
#[repr(i32)]
#[sval(unlabeled_variants)]
pub enum SeverityNumber {
    Debug = 5,
    Info = 9,
    Warn = 13,
    Error = 17,
}

#[derive(Value)]
pub struct LogRecord<'a, B: ?Sized = AnyValue<'a>, A: ?Sized = InlineLogRecordAttributes<'a>> {
    #[sval(
        label = "timeUnixNano",
        index = 1,
        data_tag = "sval_protobuf::tags::PROTOBUF_I64"
    )]
    pub time_unix_nano: u64,
    #[sval(
        label = "observedTimeUnixNano",
        index = 11,
        data_tag = "sval_protobuf::tags::PROTOBUF_I64"
    )]
    pub observed_time_unix_nano: u64,
    #[sval(label = "body", index = 5)]
    pub body: &'a B,
    #[sval(flatten)]
    pub attributes: &'a A,
}

const LOG_RECORD_SEVERITY_NUMBER_LABEL: sval::Label =
    sval::Label::new("severityNumber").with_tag(&sval::tags::VALUE_IDENT);
const LOG_RECORD_SEVERITY_TEXT_LABEL: sval::Label =
    sval::Label::new("severityText").with_tag(&sval::tags::VALUE_IDENT);
const LOG_RECORD_ATTRIBUTES_LABEL: sval::Label =
    sval::Label::new("attributes").with_tag(&sval::tags::VALUE_IDENT);
const LOG_RECORD_TRACE_ID_LABEL: sval::Label =
    sval::Label::new("traceId").with_tag(&sval::tags::VALUE_IDENT);
const LOG_RECORD_SPAN_ID_LABEL: sval::Label =
    sval::Label::new("spanId").with_tag(&sval::tags::VALUE_IDENT);

const LOG_RECORD_SEVERITY_NUMBER_INDEX: sval::Index = sval::Index::new(2);
const LOG_RECORD_SEVERITY_TEXT_INDEX: sval::Index = sval::Index::new(3);
const LOG_RECORD_ATTRIBUTES_INDEX: sval::Index = sval::Index::new(6);
const LOG_RECORD_TRACE_ID_INDEX: sval::Index = sval::Index::new(9);
const LOG_RECORD_SPAN_ID_INDEX: sval::Index = sval::Index::new(10);

#[derive(Value)]
pub struct InlineLogRecordAttributes<
    'a,
    T: ?Sized = sval::BinaryArray<16>,
    S: ?Sized = sval::BinaryArray<8>,
> {
    #[sval(label = LOG_RECORD_SEVERITY_NUMBER_LABEL, index = LOG_RECORD_SEVERITY_NUMBER_INDEX)]
    pub severity_number: SeverityNumber,
    #[sval(label = LOG_RECORD_SEVERITY_TEXT_LABEL, index = LOG_RECORD_SEVERITY_TEXT_INDEX)]
    pub severity_text: &'a str,
    #[sval(label = LOG_RECORD_ATTRIBUTES_LABEL, index = LOG_RECORD_ATTRIBUTES_INDEX)]
    pub attributes: &'a [KeyValue<&'a str, &'a AnyValue<'a>>],
    #[sval(label = LOG_RECORD_TRACE_ID_LABEL, index = LOG_RECORD_TRACE_ID_INDEX)]
    pub trace_id: &'a T,
    #[sval(label = LOG_RECORD_SPAN_ID_LABEL, index = LOG_RECORD_SPAN_ID_INDEX)]
    pub span_id: &'a S,
}

pub struct PropsLogRecordAttributes<T, S, P>(P, PhantomData<(T, S)>);

impl<T, S, P> PropsLogRecordAttributes<T, S, P> {
    pub fn new(props: P) -> Self {
        PropsLogRecordAttributes(props, PhantomData)
    }
}

impl<
        TR: From<emit::TraceId> + sval::Value,
        SP: From<emit::SpanId> + sval::Value,
        P: emit::props::Props,
    > sval::Value for PropsLogRecordAttributes<TR, SP, P>
{
    fn stream<'sval, S: sval::Stream<'sval> + ?Sized>(&'sval self, stream: &mut S) -> sval::Result {
        let mut trace_id = None;
        let mut span_id = None;
        let mut level = emit::level::Level::default();

        stream.record_tuple_begin(None, None, None, None)?;

        stream_field(
            &mut *stream,
            &LOG_RECORD_ATTRIBUTES_LABEL,
            &LOG_RECORD_ATTRIBUTES_INDEX,
            |stream| {
                stream_attributes(stream, &self.0, |mut stream, k, v| match k.get() {
                    emit::well_known::KEY_LVL => {
                        level = v.by_ref().cast::<emit::Level>().unwrap_or_default();
                        Ok(())
                    }
                    emit::well_known::KEY_SPAN_ID => {
                        span_id = v
                            .by_ref()
                            .cast::<emit::SpanId>()
                            .map(|span_id| SP::from(span_id));
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
                        // If the error has a cause chain then write it into the exception.stacktrace attribute
                        if let Some(cause) = v.to_borrowed_error().and_then(|err| err.source()) {
                            stream.stream_custom_attribute_computed(
                                emit::Str::new("exception.stacktrace"),
                                TextValue(Stacktrace::new_borrowed(cause)),
                            )?;
                        }

                        stream.stream_attribute(emit::Str::new("exception.message"), v)?;

                        Ok(())
                    }
                    _ => stream.stream_attribute(k, v),
                })
            },
        )?;

        let severity_number = match level {
            emit::level::Level::Debug => SeverityNumber::Debug as i32,
            emit::level::Level::Info => SeverityNumber::Info as i32,
            emit::level::Level::Warn => SeverityNumber::Warn as i32,
            emit::level::Level::Error => SeverityNumber::Error as i32,
        };

        stream_field(
            &mut *stream,
            &LOG_RECORD_SEVERITY_NUMBER_LABEL,
            &LOG_RECORD_SEVERITY_NUMBER_INDEX,
            |stream| stream.i32(severity_number),
        )?;

        stream_field(
            &mut *stream,
            &LOG_RECORD_SEVERITY_TEXT_LABEL,
            &LOG_RECORD_SEVERITY_TEXT_INDEX,
            |stream| sval::stream_display(stream, level),
        )?;

        if let Some(trace_id) = trace_id {
            stream_field(
                &mut *stream,
                &LOG_RECORD_TRACE_ID_LABEL,
                &LOG_RECORD_TRACE_ID_INDEX,
                |stream| stream.value_computed(&trace_id),
            )?;
        }

        if let Some(span_id) = span_id {
            stream_field(
                &mut *stream,
                &LOG_RECORD_SPAN_ID_LABEL,
                &LOG_RECORD_SPAN_ID_INDEX,
                |stream| stream.value_computed(&span_id),
            )?;
        }

        stream.record_tuple_end(None, None, None)
    }
}
