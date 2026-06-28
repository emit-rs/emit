/*!
Channel infrastructure for the OTLP emitter.

Events are sent from the caller thread through a [`Channel`] to a background
[`SignalWorker`], which serializes and ships them via HTTP.
*/

use std::sync::Arc;

use emit_batcher::BatchError;

use super::OtlpTransport;
use crate::data::{EncodedEvent, EncodedScopeItems};

/**
Execute the batching loop over a channel.

Iterates events from the current cursor position, encodes them via
`encode_event`, accumulates into size-bounded batches, and ships each
batch through `send_batch`. On send failure the cursor is reset to the
start of the failed batch so the caller can retry from that point.

# Arguments

* `channel` — the channel to process (cursor is updated in place)
* `max_request_size` — maximum bytes per batch before flushing
* `encode_event` — closure that encodes a raw event; returns `None` to skip
* `send_batch` — async closure that ships an encoded batch; returns `Err` on failure

# Returns

`Ok(())` when all events (from the cursor onward) are encoded and sent.
On failure the channel cursor is reset to the start of the failed batch.
*/
pub(crate) async fn execute<F, S>(
    mut channel: Channel,
    max_request_size: usize,
    mut encode_event: impl FnMut(&OwnedEvent) -> Option<EncodedEvent>,
    mut send_batch: F,
) -> Result<(), BatchError<Channel>>
where
    F: FnMut(&EncodedScopeItems) -> S,
    S: std::future::Future<Output = Result<(), BatchError<()>>>,
{
    let mut batch = EncodedScopeItems::new();
    let mut current_batch_size: usize = 0;
    // Cursor position of the first event in the current batch.
    // On send failure we reset to here so the batch is re-sent.
    let mut batch_start = channel.cursor;

    let mut scope_idx = channel.cursor.scope_index();
    let mut event_idx = channel.cursor.event_index();

    while scope_idx < channel.scopes.len() {
        let events = &channel.scopes[scope_idx].1;
        while event_idx < events.len() {
            let event = &events[event_idx];
            event_idx += 1;

            let Some(encoded) = encode_event(event) else {
                continue;
            };

            let event_size = encoded.payload.len();

            if current_batch_size > 0 && current_batch_size + event_size > max_request_size {
                match send_batch(&batch).await {
                    Ok(()) => {
                        batch = EncodedScopeItems::new();
                        current_batch_size = 0;
                        batch_start = Cursor::at(scope_idx, event_idx);
                    }
                    Err(e) => {
                        channel.cursor = batch_start;
                        return Err(e.map_retryable(|r| r.map(|_| channel)));
                    }
                }
            }

            // Mark batch start on the first event added
            if batch.total_items() == 0 {
                batch_start = Cursor::at(scope_idx, event_idx - 1);
            }

            batch.push(encoded);
            current_batch_size += event_size;
        }

        event_idx = 0;
        scope_idx += 1;
    }

    if batch.total_items() > 0 {
        match send_batch(&batch).await {
            Ok(()) => {}
            Err(e) => {
                channel.cursor = batch_start;
                return Err(e.map_retryable(|r| r.map(|_| channel)));
            }
        }
    }

    Ok(())
}

/** An owned event ready to be sent through a [`Channel`]. */
pub(crate) type OwnedEvent = emit::event::Event<'static, emit::props::OwnedProps>;

/**
Cursor position within a [`Channel`].

Tracks `(scope_index, event_index)` — the next event to process during
resumable batch sending. On send failure the cursor is reset to the start
of the failed batch; on success it advances past the sent events.
*/
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Cursor {
    scope_index: usize,
    event_index: usize,
}

impl Cursor {
    pub(crate) const fn new() -> Self {
        Cursor {
            scope_index: 0,
            event_index: 0,
        }
    }

    pub(crate) const fn at(scope_index: usize, event_index: usize) -> Self {
        Cursor {
            scope_index,
            event_index,
        }
    }

    pub(crate) const fn scope_index(&self) -> usize {
        self.scope_index
    }

    pub(crate) const fn event_index(&self) -> usize {
        self.event_index
    }
}

/**
A channel that groups raw events by scope path.

Events pushed into the channel are grouped by their module path so the
background worker can iterate scope-by-scope during serialization. A
[`Cursor`] tracks how far the worker has processed for resumable retry.
*/
pub(crate) struct Channel {
    /** Events grouped by scope. A `Vec` so iteration order is deterministic
    and the worker can index into it for resumable processing on retry. */
    pub(crate) scopes: Vec<(emit::Path<'static>, Vec<OwnedEvent>)>,
    pub(crate) total_items: usize,
    /** Cursor tracking how far the worker has successfully processed this
    channel. Reset on `clear()` or when a batch is fully sent. */
    pub(crate) cursor: Cursor,
}

impl Default for Channel {
    fn default() -> Self {
        Channel {
            scopes: Vec::new(),
            total_items: 0,
            cursor: Cursor::new(),
        }
    }
}

/** Item pushed into a [`Channel`] from the caller thread. */
pub(crate) struct ChannelItem {
    pub(crate) event: OwnedEvent,
}

impl emit_batcher::Channel for Channel {
    type Item = ChannelItem;

    fn new() -> Self {
        Channel::default()
    }

    fn push(&mut self, item: Self::Item) {
        let scope = item.event.mdl().to_owned();
        if let Some(entry) = self.scopes.iter_mut().find(|(s, _)| *s == scope) {
            entry.1.push(item.event);
        } else {
            self.scopes.push((scope, vec![item.event]));
        }
        self.total_items += 1;
    }

    fn len(&self) -> usize {
        self.total_items
    }

    fn clear(&mut self) {
        self.scopes.clear();
        self.total_items = 0;
        self.cursor = Cursor::new();
    }
}

/**
Bundles the transport and batcher receiver for a single signal.

The transport is shared via [`Arc`] so the batcher callback
can access it without requiring `Clone`. The receiver is
consumed by [`SignalWorker::into_receiver`].
*/
pub(crate) struct SignalWorker<S, E, R> {
    transport: Arc<OtlpTransport<S, E, R>>,
    receiver: emit_batcher::Receiver<Channel>,
}

impl<S, E, R> SignalWorker<S, E, R> {
    pub fn new(
        transport: OtlpTransport<S, E, R>,
        receiver: emit_batcher::Receiver<Channel>,
    ) -> Self {
        SignalWorker {
            transport: Arc::new(transport),
            receiver,
        }
    }

    /** Consume the worker, returning the `Arc` transport (for the callback) and the receiver (for the batcher). */
    pub fn into_receiver(self) -> (Arc<OtlpTransport<S, E, R>>, emit_batcher::Receiver<Channel>) {
        (self.transport, self.receiver)
    }
}

#[cfg(test)]
mod tests {
    use emit_batcher::Channel as _;

    use super::*;

    #[test]
    fn channel_groups_by_scope() {
        let mut channel = Channel::new();

        channel.push(ChannelItem {
            event: emit::event::Event::new(
                emit::path!("app::module_a"),
                emit::template::Template::literal("event 1"),
                emit::empty::Empty,
                emit::empty::Empty,
            )
            .to_owned(),
        });

        channel.push(ChannelItem {
            event: emit::event::Event::new(
                emit::path!("app::module_b"),
                emit::template::Template::literal("event 2"),
                emit::empty::Empty,
                emit::empty::Empty,
            )
            .to_owned(),
        });

        channel.push(ChannelItem {
            event: emit::event::Event::new(
                emit::path!("app::module_a"),
                emit::template::Template::literal("event 3"),
                emit::empty::Empty,
                emit::empty::Empty,
            )
            .to_owned(),
        });

        assert_eq!(3, channel.len());
        assert_eq!(2, channel.scopes.len());
        assert_eq!(2, channel.scopes[0].1.len()); // module_a: 2 events
        assert_eq!(1, channel.scopes[1].1.len()); // module_b: 1 event
        assert_eq!(Cursor::new(), channel.cursor);
    }

    #[test]
    fn channel_clear_resets_cursor() {
        let mut channel = Channel::new();

        channel.push(ChannelItem {
            event: emit::event::Event::new(
                emit::path!("app::module_a"),
                emit::template::Template::literal("event 1"),
                emit::empty::Empty,
                emit::empty::Empty,
            )
            .to_owned(),
        });

        channel.clear();

        assert_eq!(0, channel.len());
        assert_eq!(0, channel.scopes.len());
        assert_eq!(Cursor::new(), channel.cursor);
    }

    #[test]
    fn cursor_construction() {
        let c = Cursor::new();
        assert_eq!(0, c.scope_index());
        assert_eq!(0, c.event_index());

        let c = Cursor::at(3, 7);
        assert_eq!(3, c.scope_index());
        assert_eq!(7, c.event_index());
    }

    // ---- execute tests ----

    use crate::data::EncodedPayload;

    fn make_event(path: emit::Path<'static>) -> OwnedEvent {
        emit::event::Event::new(
            path,
            emit::template::Template::literal("event"),
            emit::empty::Empty,
            emit::empty::Empty,
        )
        .to_owned()
    }

    /** Build a mock encoded event with a payload of the given byte length. */
    fn mock_encoded(scope: emit::Path<'static>, payload_size: usize) -> EncodedEvent {
        EncodedEvent {
            scope,
            payload: EncodedPayload::Json(sval_json::JsonStr::boxed("x".repeat(payload_size))),
        }
    }

    #[tokio::test]
    async fn execute_sends_all_events_in_single_batch() {
        let mut channel = Channel::new();
        channel.push(ChannelItem {
            event: make_event(emit::path!("mod::a")),
        });
        channel.push(ChannelItem {
            event: make_event(emit::path!("mod::b")),
        });
        channel.push(ChannelItem {
            event: make_event(emit::path!("mod::c")),
        });

        let sent_batches = std::sync::Arc::new(std::sync::Mutex::new(Vec::<usize>::new()));

        let result = execute(
            channel,
            usize::MAX,
            |evt| Some(mock_encoded(evt.mdl().to_owned(), 10)),
            |batch| {
                let items = batch.total_items();
                let sent = sent_batches.clone();
                async move {
                    sent.lock().unwrap().push(items);
                    Ok(())
                }
            },
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(vec![3], *sent_batches.lock().unwrap());
    }

    #[tokio::test]
    async fn execute_chunks_by_size() {
        let mut channel = Channel::new();
        for _ in 0..5 {
            channel.push(ChannelItem {
                event: make_event(emit::path!("mod::e")),
            });
        }

        let sent_batches = std::sync::Arc::new(std::sync::Mutex::new(Vec::<usize>::new()));
        // Each payload is 20 bytes; limit of 45 forces splits: 2+2+1
        let result = execute(
            channel,
            45,
            |evt| Some(mock_encoded(evt.mdl().to_owned(), 20)),
            |batch| {
                let items = batch.total_items();
                let sent = sent_batches.clone();
                async move {
                    sent.lock().unwrap().push(items);
                    Ok(())
                }
            },
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(vec![2, 2, 1], *sent_batches.lock().unwrap());
    }

    #[tokio::test]
    async fn execute_resets_cursor_on_failure() {
        let mut channel = Channel::new();
        for _ in 0..4 {
            channel.push(ChannelItem {
                event: make_event(emit::path!("mod::e")),
            });
        }

        let send_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));

        let result = execute(
            channel,
            45,
            |evt| Some(mock_encoded(evt.mdl().to_owned(), 20)),
            |_batch| {
                let count = send_count.clone();
                async move {
                    let n = count.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                    if n == 1 {
                        Ok(())
                    } else {
                        Err(BatchError::retry(crate::Error::msg("fail"), ()))
                    }
                }
            },
        )
        .await;

        assert!(result.is_err());
        assert_eq!(2, send_count.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[tokio::test]
    async fn execute_skips_encoding_failures() {
        let mut channel = Channel::new();
        channel.push(ChannelItem {
            event: make_event(emit::path!("mod::a")),
        });
        channel.push(ChannelItem {
            event: make_event(emit::path!("mod::b")),
        });
        channel.push(ChannelItem {
            event: make_event(emit::path!("mod::c")),
        });

        let sent_batches = std::sync::Arc::new(std::sync::Mutex::new(Vec::<usize>::new()));
        let encode_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));

        let result = execute(
            channel,
            usize::MAX,
            |evt| {
                let count = encode_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                if count == 2 {
                    None // skip the second event
                } else {
                    Some(mock_encoded(evt.mdl().to_owned(), 10))
                }
            },
            |batch| {
                let items = batch.total_items();
                let sent = sent_batches.clone();
                async move {
                    sent.lock().unwrap().push(items);
                    Ok(())
                }
            },
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(3, encode_count.load(std::sync::atomic::Ordering::SeqCst));
        assert_eq!(vec![2], *sent_batches.lock().unwrap());
    }

    #[tokio::test]
    async fn execute_resumes_from_cursor() {
        let mut channel = Channel::new();
        channel.push(ChannelItem {
            event: make_event(emit::path!("mod::a")),
        });
        channel.push(ChannelItem {
            event: make_event(emit::path!("mod::b")),
        });
        channel.push(ChannelItem {
            event: make_event(emit::path!("mod::c")),
        });

        // Advance cursor to skip first event
        channel.cursor = Cursor::at(0, 1);

        let sent_batches = std::sync::Arc::new(std::sync::Mutex::new(Vec::<usize>::new()));

        let result = execute(
            channel,
            usize::MAX,
            |evt| Some(mock_encoded(evt.mdl().to_owned(), 10)),
            |batch| {
                let items = batch.total_items();
                let sent = sent_batches.clone();
                async move {
                    sent.lock().unwrap().push(items);
                    Ok(())
                }
            },
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(vec![2], *sent_batches.lock().unwrap());
    }

    #[tokio::test]
    async fn execute_empty_channel() {
        let channel = Channel::new();
        let sent = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

        let result = execute(
            channel,
            usize::MAX,
            |_| Some(mock_encoded(emit::path!("mod"), 10)),
            |_batch| {
                let s = sent.clone();
                async move {
                    s.store(true, std::sync::atomic::Ordering::SeqCst);
                    Ok(())
                }
            },
        )
        .await;

        assert!(result.is_ok());
        assert!(!sent.load(std::sync::atomic::Ordering::SeqCst));
    }
}
