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
}
