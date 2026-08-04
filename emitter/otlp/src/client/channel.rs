/*!
Channel infrastructure for the OTLP emitter.

Events are sent from the caller thread through a [`Channel`] to a background
[`SignalWorker`], which serializes and ships them via HTTP.
*/

use std::{
    collections::HashMap,
    fmt,
    future::Future,
    hash::{BuildHasher, Hash},
    ops::ControlFlow,
};

use fnv::FnvBuildHasher;
use hashbrown::{HashTable, hash_table};

use emit_batcher::BatchError;

use crate::{
    InternalMetrics,
    data::{EncodedEvent, EncodedScopeItems},
};

pub(crate) async fn batch<F>(
    mut channel: Channel,
    max_request_size: usize,
    metrics: &InternalMetrics,
    mut encode_event: impl FnMut(&ChannelEvent) -> Option<EncodedEvent>,
    mut send_batch: impl FnMut(EncodedScopeItems) -> F,
) -> Result<(), BatchError<Channel>>
where
    F: Future<Output = (EncodedScopeItems, Result<(), BatchError<()>>)>,
{
    // The batch is moved into each send future and returned by it, so its
    // allocations are re-used from one request to the next
    let mut batch = EncodedScopeItems::new();

    let mut scope_index = channel.cursor.scope_index;
    let mut event_index = channel.cursor.event_index;
    let mut remaining_items = channel.cursor.remaining_items;

    // Split our channel into batches roughly by request size
    // OTLP requires we collect events under the same scope together so we work a scope at a time
    while scope_index < channel.scopes.len() {
        let events = &channel.scopes[scope_index].1;

        while event_index < events.len() {
            let event = &events[event_index];

            event_index += 1;
            remaining_items -= 1;

            let Some(encoded) = encode_event(event) else {
                metrics.event_encoding_failed.increment();
                continue;
            };

            if batch.total_items() > 0
                && batch.total_size_bytes() + encoded.size_bytes() > max_request_size
            {
                // We've reached the maximum size of a single batch; send it then start a new one
                let (sent, result) = send_batch(batch).await;
                batch = sent;

                match result {
                    Ok(()) => {
                        batch.clear();

                        channel.cursor = ChannelCursor {
                            scope_index,
                            event_index,
                            remaining_items,
                        };
                    }
                    Err(e) => return Err(e.map_retryable(|r| r.map(|_| channel))),
                }
            }

            batch.push(encoded);
        }

        event_index = 0;
        scope_index += 1;
    }

    // Send the final batch
    if batch.total_items() > 0 {
        match send_batch(batch).await.1 {
            Ok(()) => Ok(()),
            Err(e) => Err(e.map_retryable(|r| r.map(|_| channel))),
        }
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ChannelCursor {
    scope_index: usize,
    event_index: usize,
    remaining_items: usize,
}

#[derive(Clone)]
pub(crate) struct Channel {
    scopes: Vec<(emit::Path<'static>, Vec<ChannelEvent>)>,
    scopes_by_key: HashTable<usize>,
    cursor: ChannelCursor,
}

impl fmt::Debug for Channel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Channel")
            .field("cursor", &self.cursor)
            .finish_non_exhaustive()
    }
}

impl Default for Channel {
    fn default() -> Self {
        Channel {
            scopes: Default::default(),
            scopes_by_key: Default::default(),
            cursor: ChannelCursor {
                scope_index: 0,
                event_index: 0,
                remaining_items: 0,
            },
        }
    }
}

#[derive(Clone)]
pub(crate) struct ChannelEvent(emit::Event<'static, ChannelProps>);

#[derive(Clone, Default)]
pub(crate) struct ChannelProps {
    evt_kind: Option<emit::Kind>,
    lvl: Option<emit::Level>,
    trace_id: Option<emit::TraceId>,
    span_parent: Option<emit::SpanId>,
    span_id: Option<emit::SpanId>,
    span_kind: Option<emit::SpanKind>,
    rest: HashMap<emit::Str<'static>, emit::value::OwnedValue>,
}

impl ChannelEvent {
    pub(crate) fn from_evt(evt: emit::Event<impl emit::Props>) -> Self {
        ChannelEvent(emit::Event::new(
            evt.mdl().to_owned(),
            evt.tpl().to_owned(),
            evt.extent().cloned(),
            evt.props().collect(),
        ))
    }

    pub(crate) fn get<'a>(&'a self) -> &'a emit::Event<'a, impl emit::Props> {
        &self.0
    }
}

impl<'kv> emit::props::FromProps<'kv> for ChannelProps {
    fn from_props<P: emit::Props + ?Sized>(props: &'kv P) -> Self {
        let mut owned = ChannelProps::default();

        let _ = props.for_each(|k, v| {
            match k.get() {
                // Well-known props
                emit::well_known::KEY_LVL => {
                    if let Some(v) = v.by_ref().cast() {
                        owned.lvl = Some(v);

                        return ControlFlow::Continue(());
                    }
                }
                emit::well_known::KEY_TRACE_ID => {
                    if let Some(v) = v.by_ref().cast() {
                        owned.trace_id = Some(v);

                        return ControlFlow::Continue(());
                    }
                }
                emit::well_known::KEY_SPAN_ID => {
                    if let Some(v) = v.by_ref().cast() {
                        owned.span_id = Some(v);

                        return ControlFlow::Continue(());
                    }
                }
                emit::well_known::KEY_SPAN_PARENT => {
                    if let Some(v) = v.by_ref().cast() {
                        owned.span_parent = Some(v);

                        return ControlFlow::Continue(());
                    }
                }
                emit::well_known::KEY_SPAN_KIND => {
                    if let Some(v) = v.by_ref().cast() {
                        owned.span_kind = Some(v);

                        return ControlFlow::Continue(());
                    }
                }
                emit::well_known::KEY_EVT_KIND => {
                    if let Some(v) = v.by_ref().cast() {
                        owned.evt_kind = Some(v);

                        return ControlFlow::Continue(());
                    }
                }
                _ => (),
            }

            // Insert other values
            owned.rest.insert(k.to_owned(), v.to_owned());

            ControlFlow::Continue(())
        });

        owned
    }
}

impl emit::Props for ChannelProps {
    fn for_each<'a, F: FnMut(emit::Str<'a>, emit::Value<'a>) -> ControlFlow<()>>(
        &'a self,
        mut for_each: F,
    ) -> ControlFlow<()> {
        use emit::{str::ToStr as _, value::ToValue as _};

        let ChannelProps {
            evt_kind,
            lvl,
            trace_id,
            span_parent,
            span_id,
            span_kind,
            rest,
        } = self;

        if let Some(evt_kind) = evt_kind {
            for_each(emit::well_known::KEY_EVT_KIND.to_str(), evt_kind.to_value())?;
        }
        if let Some(lvl) = lvl {
            for_each(emit::well_known::KEY_LVL.to_str(), lvl.to_value())?;
        }
        if let Some(trace_id) = trace_id {
            for_each(emit::well_known::KEY_TRACE_ID.to_str(), trace_id.to_value())?;
        }
        if let Some(span_parent) = span_parent {
            for_each(
                emit::well_known::KEY_SPAN_PARENT.to_str(),
                span_parent.to_value(),
            )?;
        }
        if let Some(span_id) = span_id {
            for_each(emit::well_known::KEY_SPAN_ID.to_str(), span_id.to_value())?;
        }
        if let Some(span_kind) = span_kind {
            for_each(
                emit::well_known::KEY_SPAN_KIND.to_str(),
                span_kind.to_value(),
            )?;
        }

        emit::Props::for_each(rest, for_each)
    }

    fn get<'v, K: emit::str::ToStr>(&'v self, key: K) -> Option<emit::Value<'v>> {
        use emit::value::ToValue as _;

        let ChannelProps {
            evt_kind,
            lvl,
            trace_id,
            span_parent,
            span_id,
            span_kind,
            rest,
        } = self;

        match key.to_str().get() {
            emit::well_known::KEY_EVT_KIND => evt_kind.as_ref().map(|v| v.to_value()),
            emit::well_known::KEY_LVL => lvl.as_ref().map(|v| v.to_value()),
            emit::well_known::KEY_TRACE_ID => trace_id.as_ref().map(|v| v.to_value()),
            emit::well_known::KEY_SPAN_ID => span_id.as_ref().map(|v| v.to_value()),
            emit::well_known::KEY_SPAN_PARENT => span_parent.as_ref().map(|v| v.to_value()),
            emit::well_known::KEY_SPAN_KIND => span_kind.as_ref().map(|v| v.to_value()),
            key => emit::Props::get(rest, key),
        }
    }

    fn is_unique(&self) -> bool {
        true
    }
}

impl emit_batcher::Channel for Channel {
    type Item = ChannelEvent;

    fn new() -> Self {
        Default::default()
    }

    fn with_capacity(capacity_hint: usize) -> Self {
        let mut channel = Self::new();

        channel.scopes = Vec::with_capacity(capacity_hint);

        channel
    }

    fn push(&mut self, item: Self::Item) {
        assert_eq!(
            0, self.cursor.scope_index,
            "attempt to push to a channel that's already being drained"
        );
        assert_eq!(
            0, self.cursor.event_index,
            "attempt to push to a channel that's already being drained"
        );

        let scope = item.get().mdl();

        match self.scopes_by_key.entry(
            hash(&scope),
            |idx| self.scopes[*idx].0 == scope,
            |idx| hash(&self.scopes[*idx].0),
        ) {
            hash_table::Entry::Occupied(entry) => {
                self.scopes[*entry.get()].1.push(item);
            }
            hash_table::Entry::Vacant(entry) => {
                let idx = self.scopes.len();

                self.scopes.push((scope.to_owned(), vec![item]));
                entry.insert(idx);
            }
        }

        self.cursor.remaining_items += 1;
    }

    fn len(&self) -> usize {
        self.cursor.remaining_items
    }

    fn clear(&mut self) {
        *self = Default::default();
    }
}

#[inline]
fn hash(v: &(impl Hash + ?Sized)) -> u64 {
    FnvBuildHasher::default().hash_one(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::{
        io,
        sync::{Arc, Mutex},
    };

    use crate::data::{EventEncoder, Json, logs};

    use emit_batcher::Channel as _;

    #[tokio::test]
    async fn channel_splits_batches_by_size() {
        let mut channel = Channel::default();

        channel.push(ChannelEvent::from_evt(
            emit::evt!(mdl: emit::path!("a"), "Event 1"),
        ));
        channel.push(ChannelEvent::from_evt(
            emit::evt!(mdl: emit::path!("a"), "Event 2"),
        ));
        channel.push(ChannelEvent::from_evt(
            emit::evt!(mdl: emit::path!("b"), "Event 3"),
        ));
        channel.push(ChannelEvent::from_evt(
            emit::evt!(mdl: emit::path!("c"), "Event 4"),
        ));

        assert_eq!(4, channel.len());

        for (case, expected) in [
            (0, 4),
            (
                {
                    logs::LogsEventEncoder::default()
                        .encode_event::<Json>(&emit::evt!(mdl: emit::path!("a"), "Event 1"))
                        .unwrap()
                        .size_bytes()
                        * 2
                },
                2,
            ),
            (usize::MAX, 1),
        ] {
            let channel = channel.clone();

            let calls = Arc::new(Mutex::new(0));

            batch(
                channel,
                case,
                &Default::default(),
                |evt| logs::LogsEventEncoder::default().encode_event::<Json>(evt.get()),
                |batch| async {
                    *calls.lock().unwrap() += 1;

                    (batch, Ok(()))
                },
            )
            .await
            .unwrap();

            assert_eq!(expected, *calls.lock().unwrap(), "max batch size {case}");
        }
    }

    #[tokio::test]
    async fn channel_retry_resumes_from_cursor() {
        let mut channel = Channel::default();

        channel.push(ChannelEvent::from_evt(
            emit::evt!(mdl: emit::path!("a"), "Event 1"),
        ));
        channel.push(ChannelEvent::from_evt(
            emit::evt!(mdl: emit::path!("a"), "Event 2"),
        ));
        channel.push(ChannelEvent::from_evt(
            emit::evt!(mdl: emit::path!("b"), "Event 3"),
        ));
        channel.push(ChannelEvent::from_evt(
            emit::evt!(mdl: emit::path!("c"), "Event 4"),
        ));

        assert_eq!(4, channel.len());

        let calls = Arc::new(Mutex::new(0));

        // This first call will fail
        let channel = batch(
            channel,
            0,
            &Default::default(),
            |evt| logs::LogsEventEncoder::default().encode_event::<Json>(evt.get()),
            |batch| async {
                let mut calls = calls.lock().unwrap();

                *calls += 1;

                if *calls == 2 {
                    return (
                        batch,
                        Err(BatchError::retry(
                            io::Error::new(io::ErrorKind::Other, "explicit failure"),
                            (),
                        )),
                    );
                }

                (batch, Ok(()))
            },
        )
        .await
        .unwrap_err()
        .into_retryable()
        .unwrap();

        assert_eq!(0, channel.cursor.scope_index);
        assert_eq!(2, channel.cursor.event_index);

        assert_eq!(2, channel.len());

        // This second call will succeed
        batch(
            channel,
            0,
            &Default::default(),
            |evt| logs::LogsEventEncoder::default().encode_event::<Json>(evt.get()),
            |batch| async { (batch, Ok(())) },
        )
        .await
        .unwrap();
    }
}
