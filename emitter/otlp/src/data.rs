/*!
Conversion of `emit` events into OTLP payloads.

Events are converted into individual payloads on-thread by an [`EventEncoder`]. The background receiver then stitches all of these encoded events into a single request using a [`RequestEncoder`].

Each signal (logs, traces, metrics) implements `EventEncoder` and `ReceiverEncoder`.

Each protocol (protobuf and JSON) implements [`RawEncoder`]. This manages the difference between trace/span id encoding between them.
*/

use std::{collections::HashMap, fmt, ops::ControlFlow};

use bytes::Buf;
use sval_derive::Value;
use sval_json::JsonStr;
use sval_protobuf::buf::{ProtoBuf, ProtoBufCursor};

use emit::Props as _;

pub mod logs;
pub mod metrics;
pub mod traces;

mod any_value;
mod instrumentation_scope;
mod resource;

#[cfg(test)]
pub(crate) mod generated;

use crate::Error;

pub use self::{any_value::*, instrumentation_scope::*, resource::*};

pub(crate) struct EncodedEvent {
    pub scope: emit::Path<'static>,
    pub payload: EncodedPayload,
}

pub(crate) trait EventEncoder {
    fn encode_event<E: RawEncoder>(
        &self,
        evt: &emit::event::Event<impl emit::props::Props>,
    ) -> Option<EncodedEvent>;
}

pub(crate) trait RequestEncoder {
    fn encode_request<E: RawEncoder>(
        &self,
        resource: Option<&EncodedPayload>,
        items: &EncodedScopeItems,
    ) -> Result<EncodedPayload, Error>;
}

pub(crate) trait RawEncoder {
    type TraceId: From<emit::span::TraceId> + sval::Value;
    type SpanId: From<emit::span::SpanId> + sval::Value;

    fn encode<V: sval::Value>(value: V) -> EncodedPayload;
}

#[derive(Default)]
pub(crate) struct EncodedScopeItems {
    items: HashMap<emit::Path<'static>, Vec<EncodedPayload>>,
}

impl EncodedScopeItems {
    pub fn new() -> Self {
        EncodedScopeItems {
            items: HashMap::new(),
        }
    }

    pub fn push(&mut self, evt: EncodedEvent) {
        let entry = self.items.entry(evt.scope).or_default();
        entry.push(evt.payload);
    }

    pub fn total_scopes(&self) -> usize {
        self.items.len()
    }

    pub fn total_items(&self) -> usize {
        self.items.values().map(|v| v.len()).sum()
    }

    pub fn items(&self) -> impl Iterator<Item = (emit::Path, &[EncodedPayload])> {
        self.items.iter().map(|(k, v)| (k.by_ref(), &**v))
    }

    pub fn clear(&mut self) {
        self.items.clear()
    }
}

fn stream_encoded_scope_items<'sval, S: sval::Stream<'sval> + ?Sized>(
    stream: &mut S,
    batch: &EncodedScopeItems,
    stream_item: impl Fn(&mut S, emit::Path, &[EncodedPayload]) -> sval::Result,
) -> sval::Result {
    stream.seq_begin(Some(batch.total_scopes()))?;

    for (path, items) in batch.items() {
        stream.seq_value_begin()?;
        stream_item(&mut *stream, path, items)?;
        stream.seq_value_end()?;
    }

    stream.seq_end()
}

pub(crate) struct Proto;

pub(crate) struct BinaryTraceId(emit::span::TraceId);

impl From<emit::span::TraceId> for BinaryTraceId {
    fn from(id: emit::span::TraceId) -> BinaryTraceId {
        BinaryTraceId(id)
    }
}

impl sval::Value for BinaryTraceId {
    fn stream<'sval, S: sval::Stream<'sval> + ?Sized>(&'sval self, stream: &mut S) -> sval::Result {
        stream.value_computed(&sval::BinaryArray::new(&self.0.to_u128().to_be_bytes()))
    }
}

pub(crate) struct BinarySpanId(emit::span::SpanId);

impl From<emit::span::SpanId> for BinarySpanId {
    fn from(id: emit::span::SpanId) -> BinarySpanId {
        BinarySpanId(id)
    }
}

impl sval::Value for BinarySpanId {
    fn stream<'sval, S: sval::Stream<'sval> + ?Sized>(&'sval self, stream: &mut S) -> sval::Result {
        stream.value_computed(&sval::BinaryArray::new(&self.0.to_u64().to_be_bytes()))
    }
}

impl RawEncoder for Proto {
    type TraceId = BinaryTraceId;
    type SpanId = BinarySpanId;

    fn encode<V: sval::Value>(value: V) -> EncodedPayload {
        EncodedPayload::Proto(sval_protobuf::stream_to_protobuf(value))
    }
}

pub(crate) struct Json;

pub(crate) struct TextTraceId(emit::span::TraceId);

impl From<emit::span::TraceId> for TextTraceId {
    fn from(id: emit::span::TraceId) -> TextTraceId {
        TextTraceId(id)
    }
}

impl sval::Value for TextTraceId {
    fn stream<'sval, S: sval::Stream<'sval> + ?Sized>(&'sval self, stream: &mut S) -> sval::Result {
        stream.value_computed(&sval::Display::new(&self.0))
    }
}

pub(crate) struct TextSpanId(emit::span::SpanId);

impl From<emit::span::SpanId> for TextSpanId {
    fn from(id: emit::span::SpanId) -> TextSpanId {
        TextSpanId(id)
    }
}

impl sval::Value for TextSpanId {
    fn stream<'sval, S: sval::Stream<'sval> + ?Sized>(&'sval self, stream: &mut S) -> sval::Result {
        stream.value_computed(&sval::Display::new(&self.0))
    }
}

impl RawEncoder for Json {
    type TraceId = TextTraceId;
    type SpanId = TextSpanId;

    fn encode<V: sval::Value>(value: V) -> EncodedPayload {
        EncodedPayload::Json(JsonStr::boxed(
            sval_json::stream_to_string(value).expect("failed to stream"),
        ))
    }
}

/**
An encoded buffer for a specific protocol.

The buffer may contain a protobuf or a JSON payload.
*/
#[derive(Value)]
#[sval(dynamic)]
pub(crate) enum EncodedPayload {
    Proto(ProtoBuf),
    Json(Box<JsonStr>),
}

impl Clone for EncodedPayload {
    fn clone(&self) -> Self {
        match self {
            Self::Proto(buf) => Self::Proto(buf.clone()),
            Self::Json(buf) => Self::Json(JsonStr::boxed(buf.as_str())),
        }
    }
}

impl EncodedPayload {
    pub fn into_cursor(self) -> PreEncodedCursor {
        match self {
            EncodedPayload::Proto(buf) => PreEncodedCursor::Proto(buf.into_cursor()),
            EncodedPayload::Json(buf) => PreEncodedCursor::Json(JsonCursor { buf, idx: 0 }),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            EncodedPayload::Proto(buf) => buf.len(),
            EncodedPayload::Json(buf) => buf.as_str().len(),
        }
    }
}

/**
A readable cursor for an [`EncodedPayload`].
*/
pub(crate) enum PreEncodedCursor {
    Proto(ProtoBufCursor),
    Json(JsonCursor),
}

pub(crate) struct JsonCursor {
    buf: Box<JsonStr>,
    idx: usize,
}

impl Buf for PreEncodedCursor {
    fn remaining(&self) -> usize {
        match self {
            PreEncodedCursor::Proto(cursor) => cursor.remaining(),
            PreEncodedCursor::Json(cursor) => cursor.buf.as_str().len() - cursor.idx,
        }
    }

    fn chunk(&self) -> &[u8] {
        match self {
            PreEncodedCursor::Proto(cursor) => cursor.chunk(),
            PreEncodedCursor::Json(cursor) => &cursor.buf.as_str().as_bytes()[cursor.idx..],
        }
    }

    fn advance(&mut self, cnt: usize) {
        match self {
            PreEncodedCursor::Proto(cursor) => cursor.advance(cnt),
            PreEncodedCursor::Json(cursor) => {
                let new_idx = cursor.idx + cnt;

                if new_idx > cursor.buf.as_str().len() {
                    panic!("attempt to advance out of bounds");
                }

                cursor.idx = new_idx;
            }
        }
    }
}

pub(crate) fn stream_field<'sval, S: sval::Stream<'sval> + ?Sized>(
    stream: &mut S,
    label: &sval::Label,
    index: &sval::Index,
    field: impl FnOnce(&mut S) -> sval::Result,
) -> sval::Result {
    stream.record_tuple_value_begin(None, label, index)?;
    field(&mut *stream)?;
    stream.record_tuple_value_end(None, label, index)
}

pub(crate) fn stream_attributes<'sval>(
    stream: &mut (impl sval::Stream<'sval> + ?Sized),
    props: &'sval impl emit::props::Props,
    mut for_each: impl FnMut(
        emit::str::Str<'sval>,
        emit::value::Value<'sval>,
    ) -> Option<(emit::str::Str<'sval>, emit::value::Value<'sval>)>,
) -> sval::Result {
    stream.seq_begin(None)?;

    props.dedup().for_each(|k, v| {
        if let Some((k, v)) = for_each(k, v) {
            stream
                .seq_value_begin()
                .map(|_| ControlFlow::Continue(()))
                .unwrap_or(ControlFlow::Break(()))?;

            sval_ref::stream_ref(
                &mut *stream,
                KeyValue {
                    key: k,
                    value: EmitValue(v),
                },
            )
            .map(|_| ControlFlow::Continue(()))
            .unwrap_or(ControlFlow::Break(()))?;

            stream
                .seq_value_end()
                .map(|_| ControlFlow::Continue(()))
                .unwrap_or(ControlFlow::Break(()))?;
        }

        ControlFlow::Continue(())
    });

    stream.seq_end()
}

pub(crate) type MessageFormatter = dyn Fn(&emit::event::Event<&dyn emit::props::ErasedProps>, &mut fmt::Formatter) -> fmt::Result
    + Send
    + Sync;

pub(crate) struct MessageRenderer<'a, P> {
    pub fmt: &'a MessageFormatter,
    pub evt: &'a emit::event::Event<'a, P>,
}

impl<'a, P: emit::props::Props> fmt::Display for MessageRenderer<'a, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self.fmt)(&self.evt.erase(), f)
    }
}

#[cfg(test)]
pub(crate) mod util {
    use super::*;

    pub(crate) fn encode_event<E: EventEncoder + Default>(
        evt: emit::Event<impl emit::Props>,
        proto: impl FnOnce(PreEncodedCursor),
    ) {
        // Ensure the JSON representation is valid JSON
        let _: serde_json::Value = serde_json::from_reader(
            E::default()
                .encode_event::<Json>(&evt)
                .unwrap()
                .payload
                .into_cursor()
                .reader(),
        )
        .unwrap();

        proto(
            E::default()
                .encode_event::<Proto>(&evt)
                .unwrap()
                .payload
                .into_cursor(),
        );
    }
}
