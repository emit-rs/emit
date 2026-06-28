/*!
Channel infrastructure for the OTLP emitter.

Events are sent from the caller thread through a [`Channel`] to a background
[`SignalWorker`], which serializes and ships them via HTTP.
*/

use std::future::Future;

use emit_batcher::BatchError;

use crate::data::{EncodedEvent, EncodedScopeItems};

pub(crate) async fn execute<F>(
    mut channel: Channel,
    max_request_size: usize,
    mut encode_event: impl FnMut(&ChannelEvent) -> Option<EncodedEvent>,
    mut send_batch: impl FnMut(&EncodedScopeItems) -> F,
) -> Result<(), BatchError<Channel>>
where
    F: Future<Output = Result<(), BatchError<()>>>,
{
    // TODO: Clear out events as they're processed
    // TODO: Review this more thoroughly
    let mut batch = EncodedScopeItems::new();

    let mut scope_index = channel.cursor.scope_index;
    let mut event_index = channel.cursor.event_index;

    while scope_index < channel.scopes.len() {
        let events = &channel.scopes[scope_index].1;

        while event_index < events.len() {
            let event = &events[event_index];
            event_index += 1;

            let Some(encoded) = encode_event(event) else {
                // TODO: metric
                continue;
            };

            if batch.total_items() > 0
                && batch.total_size_bytes() + encoded.size_bytes() > max_request_size
            {
                // We've reached the maximum size of a single batcg; send it then start a new one
                match send_batch(&batch).await {
                    Ok(()) => {
                        batch = EncodedScopeItems::new();
                        channel.cursor = ChannelCursor {
                            scope_index,
                            event_index,
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

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ChannelCursor {
    scope_index: usize,
    event_index: usize,
}

#[derive(Default)]
pub(crate) struct Channel {
    pub(crate) scopes: Vec<(emit::Path<'static>, Vec<ChannelEvent>)>,
    pub(crate) total_items: usize,
    pub(crate) cursor: ChannelCursor,
}

pub(crate) struct ChannelItem {
    pub(crate) event: ChannelEvent,
}

pub(crate) type ChannelEvent = emit::Event<'static, emit::props::OwnedProps>;

impl emit_batcher::Channel for Channel {
    type Item = ChannelItem;

    fn new() -> Self {
        Channel::default()
    }

    fn push(&mut self, item: Self::Item) {
        let scope = item.event.mdl();

        // TODO: Binary search or secondary hashmap
        if let Some(entry) = self.scopes.iter_mut().find(|(s, _)| *s == scope) {
            entry.1.push(item.event);
        } else {
            self.scopes.push((scope.to_owned(), vec![item.event]));
        }

        self.total_items += 1;
    }

    fn len(&self) -> usize {
        self.total_items
    }

    fn clear(&mut self) {
        let Channel {
            scopes,
            total_items,
            cursor,
        } = self;

        scopes.clear();
        *total_items = 0;
        *cursor = ChannelCursor::default();
    }
}

#[cfg(test)]
mod tests {
    use emit_batcher::Channel as _;

    use super::*;

    #[test]
    fn it_works() {
        todo!()
    }
}
