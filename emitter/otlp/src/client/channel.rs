/*!
Channel infrastructure for the OTLP emitter.

Events are sent from the caller thread through a [`Channel`] to a background
[`SignalWorker`], which serializes and ships them via HTTP.
*/

use std::{collections::HashMap, future::Future, mem};

use emit_batcher::BatchError;

use crate::data::{EncodedEvent, EncodedScopeItems};

pub(crate) async fn batch<F>(
    mut channel: Channel,
    max_request_size: usize,
    mut encode_event: impl FnMut(&ChannelEvent) -> Option<EncodedEvent>,
    mut send_batch: impl FnMut(&EncodedScopeItems) -> F,
) -> Result<(), BatchError<Channel>>
where
    F: Future<Output = Result<(), BatchError<()>>>,
{
    let state = channel.state.draining_mut();

    emit::debug!("processing channel: {#[emit::as_debug] state}");

    let mut batch = EncodedScopeItems::new();

    let mut scope_index = state.cursor.scope_index;
    let mut event_index = state.cursor.event_index;
    let mut remaining_items = state.cursor.remaining_items;

    while scope_index < state.scopes.len() {
        let events = &state.scopes[scope_index].1;

        while event_index < events.len() {
            let event = &events[event_index];
            event_index += 1;
            remaining_items -= 1;

            let Some(encoded) = encode_event(event) else {
                // TODO: metric
                continue;
            };

            if batch.total_items() > 0
                && batch.total_size_bytes() + encoded.size_bytes() > max_request_size
            {
                // We've reached the maximum size of a single batch; send it then start a new one
                match send_batch(&batch).await {
                    Ok(()) => {
                        batch.clear();
                        state.cursor = ChannelCursor {
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
    state: ChannelState,
}

impl Default for Channel {
    fn default() -> Self {
        Channel {
            state: ChannelState::Filling(ChannelStateFilling {
                scopes: HashMap::new(),
                total_items: 0,
            }),
        }
    }
}

#[derive(Debug)]
enum ChannelState {
    Filling(ChannelStateFilling),
    Draining(ChannelStateDraining),
    Poisoned,
}

#[derive(Debug)]
struct ChannelStateFilling {
    scopes: HashMap<emit::Path<'static>, Vec<ChannelEvent>>,
    total_items: usize,
}

#[derive(Debug)]
struct ChannelStateDraining {
    scopes: Vec<(emit::Path<'static>, Vec<ChannelEvent>)>,
    cursor: ChannelCursor,
}

impl ChannelState {
    fn variant(&self) -> &'static str {
        match self {
            ChannelState::Filling(_) => "filling",
            ChannelState::Draining(_) => "draining",
            ChannelState::Poisoned => "poisoned",
        }
    }

    fn filling_mut(&mut self) -> &mut ChannelStateFilling {
        let ChannelState::Filling(state) = self else {
            panic!("invalid channel state: {:?}", self.variant());
        };

        state
    }

    fn draining_mut(&mut self) -> &mut ChannelStateDraining {
        match mem::replace(self, ChannelState::Poisoned) {
            ChannelState::Filling(state) => {
                // Convert a filling channel into a draining one
                *self = ChannelState::Draining(ChannelStateDraining {
                    scopes: state.scopes.into_iter().collect(),
                    cursor: ChannelCursor {
                        scope_index: 0,
                        event_index: 0,
                        remaining_items: state.total_items,
                    },
                });
            }
            ChannelState::Draining(state) => {
                *self = ChannelState::Draining(state);
            }
            ChannelState::Poisoned => panic!("invalid channel state: {:?}", self.variant()),
        }

        let ChannelState::Draining(state) = self else {
            unreachable!()
        };

        state
    }

    fn remaining(&self) -> usize {
        match self {
            ChannelState::Filling(state) => state.total_items,
            ChannelState::Draining(state) => state.cursor.remaining_items,
            ChannelState::Poisoned => panic!("invalid channel state: {:?}", self.variant()),
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

    fn push(&mut self, item: Self::Item) {
        let state = self.state.filling_mut();

        let scope = item.event.mdl();

        state
            .scopes
            .entry(scope.to_owned())
            .or_insert_with(|| Vec::new())
            .push(item.event);
        state.total_items += 1;
    }

    fn len(&self) -> usize {
        self.state.remaining()
    }

    fn clear(&mut self) {
        *self = Default::default();
    }
}
