mod export_logs_service;
mod log_record;

use crate::Error;

pub use self::{export_logs_service::*, log_record::*};

use super::{
    stream_encoded_scope_items, AnyValue, EncodedEvent, EncodedPayload, EncodedScopeItems,
    EventEncoder, InstrumentationScope, MessageFormatter, MessageRenderer, RawEncoder,
    RequestEncoder,
};

pub(crate) struct LogsEventEncoder {
    pub body: Box<MessageFormatter>,
}

impl Default for LogsEventEncoder {
    fn default() -> Self {
        LogsEventEncoder {
            body: default_message_formatter(),
        }
    }
}

fn default_message_formatter() -> Box<MessageFormatter> {
    Box::new(|evt, f| write!(f, "{}", evt.msg()))
}

impl EventEncoder for LogsEventEncoder {
    fn encode_event<E: RawEncoder>(
        &self,
        evt: &emit::event::Event<impl emit::props::Props>,
    ) -> Option<EncodedEvent> {
        let time_unix_nano = evt
            .extent()
            .map(|extent| extent.as_point().to_unix().as_nanos() as u64)
            .unwrap_or_default();

        let observed_time_unix_nano = time_unix_nano;

        Some(EncodedEvent {
            scope: evt.mdl().to_owned(),
            payload: E::encode(LogRecord {
                time_unix_nano,
                observed_time_unix_nano,
                body: &Some(AnyValue::<_>::String(&sval::Display::new(
                    MessageRenderer {
                        fmt: &self.body,
                        evt,
                    },
                ))),
                attributes: &PropsLogRecordAttributes::<E::TraceId, E::SpanId, _>::new(evt.props()),
            }),
        })
    }
}

#[derive(Default)]
pub(crate) struct LogsRequestEncoder;

impl RequestEncoder for LogsRequestEncoder {
    fn encode_request<E: RawEncoder>(
        &self,
        resource: Option<&EncodedPayload>,
        items: &EncodedScopeItems,
    ) -> Result<EncodedPayload, Error> {
        Ok(E::encode(ExportLogsServiceRequest {
            resource_logs: &[ResourceLogs {
                resource: &resource,
                scope_logs: &EncodedScopeLogs(items),
            }],
        }))
    }
}

struct EncodedScopeLogs<'a>(&'a EncodedScopeItems);

impl<'a> sval::Value for EncodedScopeLogs<'a> {
    fn stream<'sval, S: sval::Stream<'sval> + ?Sized>(&'sval self, stream: &mut S) -> sval::Result {
        stream_encoded_scope_items(stream, &self.0, |stream, path, log_records| {
            stream.value_computed(&ScopeLogs {
                scope: &InstrumentationScope {
                    name: &sval::Display::new(path),
                },
                log_records,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use prost::Message;

    use crate::data::{
        generated::{logs::v1 as logs, util::*},
        util::*,
    };

    #[test]
    fn encode_basic() {
        encode_event::<LogsEventEncoder>(emit::evt!("log for {user}", user: "test"), |buf| {
            let de = logs::LogRecord::decode(buf).unwrap();

            assert_eq!(Some(string_value("log for test")), de.body);

            assert_eq!(Some(string_value("test")), de.attributes[0].value);
        });
    }

    #[test]
    fn encode_lvl() {
        for (case, expected) in [
            (emit::Level::Debug, logs::SeverityNumber::Debug),
            (emit::Level::Info, logs::SeverityNumber::Info),
            (emit::Level::Warn, logs::SeverityNumber::Warn),
            (emit::Level::Error, logs::SeverityNumber::Error),
        ] {
            encode_event::<LogsEventEncoder>(emit::evt!("event", lvl: case), |buf| {
                let de = logs::LogRecord::decode(buf).unwrap();

                assert_eq!(0, de.attributes.len());
                assert_eq!(expected as i32, de.severity_number);
                assert_eq!(case.to_string(), de.severity_text);
            });
        }
    }

    #[test]
    fn encode_trace_id() {
        encode_event::<LogsEventEncoder>(
            emit::evt!("event", trace_id: "4bf92f3577b34da6a3ce929d0e0e4736"),
            |buf| {
                let de = logs::LogRecord::decode(buf).unwrap();

                assert_eq!(0, de.attributes.len());
                assert_eq!(
                    vec![75u8, 249, 47, 53, 119, 179, 77, 166, 163, 206, 146, 157, 14, 14, 71, 54],
                    de.trace_id
                );
            },
        );
    }

    #[test]
    fn encode_span_id() {
        encode_event::<LogsEventEncoder>(emit::evt!("event", span_id: "00f067aa0ba902b7"), |buf| {
            let de = logs::LogRecord::decode(buf).unwrap();

            assert_eq!(0, de.attributes.len());
            assert_eq!(vec![0u8, 240, 103, 170, 11, 169, 2, 183], de.span_id);
        });
    }

    #[test]
    fn encode_err_str() {
        encode_event::<LogsEventEncoder>(emit::evt!("failed: {err}", err: "test"), |buf| {
            let de = logs::LogRecord::decode(buf).unwrap();

            assert_eq!(1, de.attributes.len());

            assert_eq!("exception.message", de.attributes[0].key);
            assert_eq!(Some(string_value("test")), de.attributes[0].value);
        });
    }

    #[test]
    fn encode_err_stacktrace() {
        #[derive(Debug)]
        struct Error {
            msg: &'static str,
            source: Option<Box<dyn std::error::Error + 'static>>,
        }

        impl std::fmt::Display for Error {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                std::fmt::Display::fmt(self.msg, f)
            }
        }

        impl std::error::Error for Error {
            fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                self.source.as_ref().map(|source| &**source)
            }
        }

        let err = Error {
            msg: "something went wrong",
            source: Some(Box::new(Error {
                msg: "there was a problem",
                source: Some(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "IO error",
                ))),
            })),
        };

        encode_event::<LogsEventEncoder>(emit::evt!("failed: {err}"), |buf| {
            let de = logs::LogRecord::decode(buf).unwrap();

            assert_eq!(2, de.attributes.len());

            assert_eq!("exception.stacktrace", de.attributes[0].key);
            assert_eq!(
                Some(string_value(
                    "caused by: there was a problem\ncaused by: IO error"
                )),
                de.attributes[0].value
            );

            assert_eq!("exception.message", de.attributes[1].key);
            assert_eq!(
                Some(string_value("something went wrong")),
                de.attributes[1].value
            );
        });

        let err = Error {
            msg: "something went wrong",
            source: None,
        };

        encode_event::<LogsEventEncoder>(emit::evt!("failed: {err}"), |buf| {
            let de = logs::LogRecord::decode(buf).unwrap();

            assert_eq!(1, de.attributes.len());

            assert_eq!("exception.message", de.attributes[0].key);
            assert_eq!(
                Some(string_value("something went wrong")),
                de.attributes[0].value
            );
        });
    }
}
