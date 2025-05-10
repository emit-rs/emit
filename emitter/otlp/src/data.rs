/*!
Conversion of `emit` events into OTLP payloads.

Events are converted into individual payloads on-thread by an [`EventEncoder`]. The background receiver then stitches all of these encoded events into a single request using a [`RequestEncoder`].

Each signal (logs, traces, metrics) implements `EventEncoder` and `ReceiverEncoder`.

Each protocol (protobuf and JSON) implements [`RawEncoder`]. This manages the difference between trace/span id encoding between them.
*/

use std::{cell::RefCell, collections::HashMap, fmt, ops::ControlFlow};

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
    type TraceId: From<emit::TraceId> + sval::Value;
    type SpanId: From<emit::SpanId> + sval::Value;

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

pub(crate) struct BinaryTraceId(emit::TraceId);

impl From<emit::TraceId> for BinaryTraceId {
    fn from(id: emit::TraceId) -> BinaryTraceId {
        BinaryTraceId(id)
    }
}

impl sval::Value for BinaryTraceId {
    fn stream<'sval, S: sval::Stream<'sval> + ?Sized>(&'sval self, stream: &mut S) -> sval::Result {
        stream.value_computed(&sval::BinaryArray::new(&self.0.to_u128().to_be_bytes()))
    }
}

pub(crate) struct BinarySpanId(emit::SpanId);

impl From<emit::SpanId> for BinarySpanId {
    fn from(id: emit::SpanId) -> BinarySpanId {
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
        // Where possible, we want to pre-allocate buffers for protobuf encoding
        // Log data tends to be fairly normalized, so we can get a reasonable idea
        // of what to pre-allocate by watching the sizes of events as they run through

        const WINDOW_SIZE: usize = 8;

        struct LocalCapacity {
            window: [sval_protobuf::Capacity; WINDOW_SIZE],
            counter: usize,
            reuse: Option<sval_protobuf::ProtoBufStreamReusable>,
        }

        thread_local! {
            static LOCAL_CAPACITY: RefCell<LocalCapacity> = RefCell::new(LocalCapacity {
                window: [sval_protobuf::Capacity::new(); WINDOW_SIZE],
                counter: 0,
                reuse: None,
            })
        };

        let payload = LOCAL_CAPACITY.with(|lc| {
            // Get re-usable allocations, if there are any
            let reuse = {
                let mut lc = lc.borrow_mut();

                let reuse = lc.reuse.take().unwrap_or_default();
                reuse.with_capacity(sval_protobuf::Capacity::next(&lc.window))
            };

            // NOTE: protobuf encoding is infallible
            let mut stream = sval_protobuf::ProtoBufStream::new_reuse(reuse);
            value.stream(&mut stream).unwrap();
            let (payload, reuse) = stream.freeze_reuse();

            // Restore re-usable allocations
            {
                let mut lc = lc.borrow_mut();
                let lc = &mut *lc;

                lc.counter += 1;
                lc.window[lc.counter % lc.window.len()] = reuse.capacity();
                lc.reuse = Some(reuse);
            }

            payload
        });

        EncodedPayload::Proto(payload)
    }
}

pub(crate) struct Json;

pub(crate) struct TextTraceId(emit::TraceId);

impl From<emit::TraceId> for TextTraceId {
    fn from(id: emit::TraceId) -> TextTraceId {
        TextTraceId(id)
    }
}

impl sval::Value for TextTraceId {
    fn stream<'sval, S: sval::Stream<'sval> + ?Sized>(&'sval self, stream: &mut S) -> sval::Result {
        stream.value_computed(&sval::Display::new(&self.0))
    }
}

pub(crate) struct TextSpanId(emit::SpanId);

impl From<emit::SpanId> for TextSpanId {
    fn from(id: emit::SpanId) -> TextSpanId {
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

pub(crate) fn stream_attributes<'sval, S: sval::Stream<'sval> + ?Sized>(
    stream: &mut S,
    props: &'sval impl emit::props::Props,
    mut for_each: impl FnMut(
        AttributeStream<'_, S>,
        emit::str::Str<'sval>,
        emit::value::Value<'sval>,
    ) -> sval::Result,
) -> sval::Result {
    stream.seq_begin(None)?;

    let _ = props.dedup().for_each(|k, v| {
        for_each(AttributeStream(&mut *stream), k, v)
            .map(|_| ControlFlow::Continue(()))
            .unwrap_or(ControlFlow::Break(()))?;

        ControlFlow::Continue(())
    });

    stream.seq_end()
}

pub(crate) struct AttributeStream<'a, S: ?Sized>(&'a mut S);

impl<'a, 'sval, S: sval::Stream<'sval> + ?Sized> AttributeStream<'a, S> {
    pub(crate) fn stream_attribute(
        &mut self,
        key: emit::str::Str<'sval>,
        value: emit::value::Value<'sval>,
    ) -> sval::Result {
        self.0.seq_value_begin()?;

        sval_ref::stream_ref(
            &mut *self.0,
            KeyValue {
                key,
                value: EmitValue(value),
            },
        )?;

        self.0.seq_value_end()?;

        Ok(())
    }

    pub(crate) fn stream_custom_attribute_computed(
        &mut self,
        key: emit::str::Str<'_>,
        value: impl sval::Value,
    ) -> sval::Result {
        self.0.seq_value_begin()?;

        self.0.value_computed(&KeyValue { key, value })?;

        self.0.seq_value_end()?;

        Ok(())
    }
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
        encode_event_with(E::default(), evt, proto)
    }

    pub(crate) fn encode_event_with(
        encoder: impl EventEncoder,
        evt: emit::Event<impl emit::Props>,
        proto: impl FnOnce(PreEncodedCursor),
    ) {
        // Ensure the JSON representation is valid JSON
        let _: serde_json::Value = serde_json::from_reader(
            encoder
                .encode_event::<Json>(&evt)
                .unwrap()
                .payload
                .into_cursor()
                .reader(),
        )
        .unwrap();

        proto(
            encoder
                .encode_event::<Proto>(&evt)
                .unwrap()
                .payload
                .into_cursor(),
        );
    }
}
