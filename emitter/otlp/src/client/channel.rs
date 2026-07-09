/*!
Channel infrastructure for the OTLP emitter.

Events are sent from the caller thread through a [`Channel`] to a background
[`SignalWorker`], which serializes and ships them via HTTP.
*/

use std::{
    future::Future,
    hash::{BuildHasher, Hash},
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
    mut send_batch: impl FnMut(&EncodedScopeItems) -> F,
) -> Result<(), BatchError<Channel>>
where
    F: Future<Output = Result<(), BatchError<()>>>,
{
    let mut batch = EncodedScopeItems::new();

    let mut scope_index = channel.cursor.scope_index;
    let mut event_index = channel.cursor.event_index;
    let mut remaining_items = channel.cursor.remaining_items;

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
                match send_batch(&batch).await {
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
        match send_batch(&batch).await {
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

pub(crate) struct Channel {
    scopes: Vec<(emit::Path<'static>, Vec<ChannelEvent>)>,
    scopes_by_key: HashTable<usize>,
    cursor: ChannelCursor,
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

pub(crate) struct ChannelItem {
    pub(crate) event: ChannelEvent,
}

pub(crate) type ChannelEvent = emit::Event<'static, emit::props::OwnedProps>;

impl emit_batcher::Channel for Channel {
    type Item = ChannelItem;

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

        let scope = item.event.mdl();

        match self.scopes_by_key.entry(
            hash(&scope),
            |idx| self.scopes[*idx].0 == scope,
            |idx| hash(&self.scopes[*idx].0),
        ) {
            hash_table::Entry::Occupied(entry) => {
                self.scopes[*entry.get()].1.push(item.event);
            }
            hash_table::Entry::Vacant(entry) => {
                let idx = self.scopes.len();

                self.scopes.push((scope.to_owned(), vec![item.event]));
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
