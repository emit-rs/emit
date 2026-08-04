/*!
Infrastructure for emitting diagnostic events in the background.

This library implements a channel that can be used to spawn background workers on a dedicated thread or `tokio` runtime. The channel implements:

- **Batching:** Events written to the channel are processed by the worker in batches rather than one-at-a-time.
- **Retries with backoff:** If the worker fails or panics then the batch can be retried up to some number of times, with backoff applied between retries. The worker can decide how much of a batch needs to be retried.
- **Maximum size management:** If the worker can't keep up then the channel truncates to avoid runaway memory use. The alternative would be to apply backpressure, but that would affect system availability so isn't suitable for diagnostics.
- **Flushing:** Callers can ask the worker to signal when all diagnostic events in the channel at the point they called are processed. This can be used for auditing and flushing on shutdown.

# WebAssembly

This library can be used on the `wasm32-unknown-unknown` target by enabling the `web` Cargo feature.
Instead of spawning background threads to run the batching receiver, it will instead spawn a fire-and-forget promise.
Blocking functions like `blocking_send` and `blocking_flush` are not available, but asynchronous `send` and `flush` variants are.
*/

#![doc(html_logo_url = "https://raw.githubusercontent.com/emit-rs/emit/main/asset/logo.svg")]
#![deny(missing_docs)]

use crate::internal_metrics::InternalMetrics;
use std::{
    any::Any,
    cmp, error, fmt,
    future::Future,
    mem,
    panic::{self, AssertUnwindSafe, UnwindSafe},
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
    time::Duration,
};

mod internal_metrics;

/**
A channel between a shared [`Sender`] and exclusive [`Receiver`].

The sender pushes items onto the channel. At some point, the receiver swaps the channel out for a fresh one and processes it.
*/
pub trait Channel {
    /**
    The kind of item stored in this channel.
    */
    type Item;

    /**
    Create a new, empty channel.

    This method shouldn't allocate.
    */
    fn new() -> Self;

    /**
    Create a channel with the given capacity hint.

    The hint is to avoid potentially re-allocating the channel and should be respected, but is safe to ignore.
    */
    fn with_capacity(capacity_hint: usize) -> Self
    where
        Self: Sized,
    {
        let _ = capacity_hint;

        Self::new()
    }

    /**
    Push an item onto the end of the channel.
    */
    fn push(&mut self, item: Self::Item);

    /**
    The number of items in the channel.
    */
    fn len(&self) -> usize;

    /**
    Whether the channel has any items in it.
    */
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /**
    Clear everything out of the channel.

    After this call, [`Channel::len`] must return `0`.
    */
    fn clear(&mut self);
}

impl<T> Channel for Vec<T> {
    type Item = T;

    fn new() -> Self {
        Vec::new()
    }

    fn with_capacity(capacity: usize) -> Self {
        Vec::with_capacity(capacity)
    }

    fn push<'a>(&mut self, item: Self::Item) {
        self.push(item);
    }

    fn len(&self) -> usize {
        self.len()
    }

    fn is_empty(&self) -> bool {
        self.is_empty()
    }

    fn clear(&mut self) {
        self.clear()
    }
}

/**
Create a [`Sender`] and [`Receiver`] pair with the given [`Channel`] type, `T`.

If the channel exceeds `max_capacity` then it will be cleared.

Use [`Sender::send`] to push items onto the channel.

Use [`tokio::spawn`] or [`sync::spawn`] to run the receiver-side of the channel.
*/
pub fn bounded<T: Channel>(max_capacity: usize) -> (Sender<T>, Receiver<T>) {
    let shared = Arc::new(Shared {
        metrics: Default::default(),
        receiver_notifier: ReceiverNotifier::new(),
        state: Mutex::new(State {
            next_batch: Batch::new(),
            is_open: true,
            is_in_batch: false,
        }),
    });

    (
        Sender {
            max_capacity,
            shared: shared.clone(),
        },
        // NOTE: These should be made configurable via a `Builder`
        // The defaults chosen here are not going to be optimal for all cases
        // These defaults give a batch ~30 seconds to get through before it'll be dropped
        Receiver {
            // The time the receiver will wait before checking for a batch of events to emit
            idle_delay: Delay::new(Duration::from_millis(1), Duration::from_millis(500)),
            // The maximum number of times a retryable batch will be retried
            retry: Retry::new(10),
            // The backoff applied to retries
            retry_delay: Delay::new(Duration::from_millis(700), Duration::from_secs(10)),
            capacity: Capacity::new(),
            shared,
            #[cfg(all(not(target_arch = "wasm32"), test))]
            test_barriers: TestBarriers::default(),
        },
    )
}

/**
The sending half of a channel.
*/
pub struct Sender<T> {
    max_capacity: usize,
    shared: Arc<Shared<T>>,
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        self.shared.state.lock().unwrap().is_open = false;

        // Wake the receiver so it can process any last batch and shut down
        // promptly instead of discovering the closed channel on its next
        // scheduled wakeup
        self.shared.receiver_notifier.notify();
    }
}

impl<T: Channel> Sender<T> {
    /**
    Send an item on the channel.

    The item will be processed at some future point by the [`Receiver`]. If pushing the item would overflow the maximum capacity of the channel it will be cleared first.
    */
    pub fn send<'a>(&self, msg: T::Item) {
        let mut state = self.shared.state.lock().unwrap();

        // If the channel is full then drop it; this prevents OOMing
        // when the destination is unavailable. We don't notify the batch
        // in this case because the clearing is opaque to outside observers
        if state.next_batch.channel.len() >= self.max_capacity {
            state.next_batch.channel.clear();
            self.shared.metrics.queue_full_truncated.increment();
        }

        // If the channel is closed then return without adding the message
        if !state.is_open {
            return;
        }

        state.next_batch.channel.push(msg);

        let len = state.next_batch.channel.len();
        drop(state);

        // If the channel is filling up then wake the receiver so it has a
        // chance to process a batch before the channel overflows and truncates.
        // This only fires once as the length crosses the threshold, so a receiver
        // that can't keep up isn't repeatedly woken
        if len == self.notify_threshold() {
            self.shared.receiver_notifier.notify();
        }
    }

    /**
    Send an item on the channel, returning it if it's currently full.

    The item will be processed at some future point by the [`Receiver`]. If pushing the item would overflow the maximum capacity of the channel then this method will return `Err`.
    */
    pub fn try_send<'a>(&self, msg: T::Item) -> Result<(), BatchError<T::Item>> {
        let mut state = self.shared.state.lock().unwrap();

        if !state.is_open {
            return Err(BatchError::no_retry(TrySendError("the channel is closed")));
        }

        // If the channel is not full then push the message and return
        if state.next_batch.channel.len() < self.max_capacity {
            state.next_batch.channel.push(msg);

            let len = state.next_batch.channel.len();
            drop(state);

            // If the channel is filling up then wake the receiver so it has a
            // chance to process a batch before the channel fills completely
            if len == self.notify_threshold() {
                self.shared.receiver_notifier.notify();
            }

            Ok(())
        } else {
            Err(BatchError::retry(TrySendError("the channel is full"), msg))
        }
    }

    fn notify_threshold(&self) -> usize {
        cmp::max(1, self.max_capacity / 2)
    }

    async fn send_or_wait<'a, FWait: Future<Output = ()> + 'a>(
        &'a self,
        msg: T::Item,
        timeout: Duration,
        elapsed: impl Fn() -> Duration,
        mut wait_until_empty: impl FnMut(&'a Self, Duration) -> FWait,
    ) -> Result<(), BatchError<T::Item>> {
        match self.try_send(msg) {
            // If the message was sent then return
            Ok(()) => Ok(()),
            // If the message wasn't sent then wait until the next batch is taken then try again
            Err(mut err) => {
                self.shared.metrics.queue_full_blocked.increment();

                loop {
                    let elapsed = elapsed();

                    if elapsed >= timeout {
                        return Err(err);
                    }

                    wait_until_empty(self, timeout.saturating_sub(elapsed)).await;

                    // NOTE: Between being triggered and calling, we may have filled up again
                    match self.try_send(err.try_into_retryable()?) {
                        Ok(()) => return Ok(()),
                        Err(retry) => {
                            err = retry;
                            continue;
                        }
                    }
                }
            }
        }
    }

    /**
    Set a callback to fire when the next batch is taken.

    The callback is guaranteed to trigger at a point where the current batch is empty, or when the channel is closed and the batch will never be taken.
    */
    pub fn when_empty(&self, f: impl FnOnce() + Send + 'static) {
        let mut state = self.shared.state.lock().unwrap();

        // If:
        // - The next batch is empty (there's nothing to wait for) or
        // - the channel is closed (the batch will never be taken)
        // Then:
        // - Call the callback without scheduling it
        if state.next_batch.channel.is_empty() || !state.is_open {
            drop(state);

            f();
        } else {
            state.next_batch.notifiers.push_on_take(Box::new(f));
            drop(state);

            // Notify the receiver so the callback triggers as soon as the batch
            // is taken rather than on the receiver's next scheduled wakeup
            self.shared.receiver_notifier.notify();
        }
    }

    /**
    Set a callback to fire when all items in the active batch are processed by the [`Receiver`].

    The callback is guaranteed to trigger at a point where the batch that was processing at the time this call was made has completed, whether or not processing succeeded. To observe the outcome of the flush, use [`sync::blocking_flush`] or one of its asynchronous variants instead.
    */
    pub fn when_flushed(&self, f: impl FnOnce() + Send + 'static) {
        self.when_flushed_inner(move |_| f())
    }

    fn when_flushed_inner(&self, f: impl FnOnce(bool) + Send + 'static) {
        let mut state = self.shared.state.lock().unwrap();

        // If there's no batch being processed and nothing pending then
        // there's nothing to flush; the flush is trivially successful
        if !state.is_in_batch && state.next_batch.channel.is_empty() {
            // Drop the lock before signaling the callback
            drop(state);

            f(true);
        }
        // If the channel is closed then anything pending will never be
        // processed; the flush has failed
        else if !state.is_open {
            // Drop the lock before signaling the callback
            drop(state);

            f(false);
        }
        // If there's active data to flush then schedule the callback
        else {
            // If a batch is currently being processed then the caller's items
            // may be split between it and the next batch, so the outcome needs
            // to cover both
            let requires_in_flight = state.is_in_batch;

            state
                .next_batch
                .notifiers
                .push_on_flush(requires_in_flight, Box::new(f));

            // Drop the lock before signaling the receiver
            drop(state);

            // Wake the receiver so the flush starts immediately rather than
            // on the receiver's next scheduled wakeup
            self.shared.receiver_notifier.notify();
        }
    }

    /**
    Get an [`emit::metric::Source`] for instrumentation produced by the channel.

    These metrics can be used to monitor the running health of your diagnostic pipeline.
    */
    pub fn metric_source(&self) -> ChannelMetrics<T> {
        ChannelMetrics {
            shared: self.shared.clone(),
        }
    }
}

/**
Deterministic ordering in tests via barriers.
*/
#[cfg(all(not(target_arch = "wasm32"), test))]
#[derive(Default, Clone)]
struct TestBarriers {
    // NOTE: These use `tokio`'s barrier type since we always have `tokio` in tests
    // but could be rewritten to some generic `futures` type
    pre_take: Option<Arc<::tokio::sync::Barrier>>,
    post_take: Option<Arc<::tokio::sync::Barrier>>,
    post_process: Option<Arc<::tokio::sync::Barrier>>,
}

#[cfg(all(not(target_arch = "wasm32"), test))]
impl TestBarriers {
    async fn wait_pre_take(&self) {
        if let Some(ref barrier) = self.pre_take {
            barrier.wait().await;
        }
    }

    async fn wait_post_take(&self) {
        if let Some(ref barrier) = self.post_take {
            barrier.wait().await;
        }
    }

    async fn wait_post_process(&self) {
        if let Some(ref barrier) = self.post_process {
            barrier.wait().await;
        }
    }
}

/**
The receiving half of a channel.

Use [`Receiver::exec`], [`crate::tokio::spawn`], or [`crate::sync::spawn`] to run the receiver as a background worker.
*/
pub struct Receiver<T> {
    idle_delay: Delay,
    retry: Retry,
    retry_delay: Delay,
    capacity: Capacity,
    shared: Arc<Shared<T>>,
    #[cfg(all(not(target_arch = "wasm32"), test))]
    test_barriers: TestBarriers,
}

impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        self.shared.state.lock().unwrap().is_open = false;

        // NOTE: If the sender is waiting for a flush it may time out
        // This is more accurate than triggering it on drop here even if
        // the queue is non-empty
    }
}

impl<T: Channel> Receiver<T> {
    /**
    Run the receiver asynchronously.

    The returned future will resolve once the [`Sender`] is dropped.

    If you're using `tokio`, see [`crate::tokio::spawn`] for a more `tokio`-aware way to run the receiver asynchronously.
    */
    pub async fn exec<
        FBatch: Future<Output = Result<(), BatchError<T>>>,
        FWait: Future<Output = ()>,
    >(
        self,
        mut wait: impl FnMut(Duration) -> FWait,
        on_batch: impl FnMut(T) -> FBatch,
    ) {
        self.exec_inner(move |_, delay| wait(delay), on_batch).await
    }

    pub(crate) async fn exec_inner<
        FBatch: Future<Output = Result<(), BatchError<T>>>,
        FWait: Future<Output = ()>,
    >(
        mut self,
        mut wait: impl FnMut(Wait, Duration) -> FWait,
        mut on_batch: impl FnMut(T) -> FBatch,
    ) {
        // This variable holds the "next" batch
        // Under the lock all we do is push onto a pre-allocated vec
        // and replace it with another pre-allocated vec
        let mut next_batch = Batch::new();

        // Whether the last *non-empty* batch was processed successfully
        let mut last_batch_flushed = true;

        loop {
            // Pre-take barrier: wait here before batch is taken
            #[cfg(all(not(target_arch = "wasm32"), test))]
            self.test_barriers.wait_pre_take().await;

            // Run inside the lock
            let (mut current_batch, is_open) = {
                let mut state = self.shared.state.lock().unwrap();

                // NOTE: We don't check the `is_open` value here because we want a chance to emit
                // any last batch

                // If there are events then mark that we're in a batch and replace it with an empty one
                // The sender will start filling this new batch
                if state.next_batch.channel.len() > 0 {
                    state.is_in_batch = true;

                    (
                        mem::replace(&mut state.next_batch, mem::take(&mut next_batch)),
                        state.is_open,
                    )
                }
                // If there are no events to emit then mark that we're outside of a batch and take its notifiers
                else {
                    state.is_in_batch = false;

                    let notifiers = mem::take(&mut state.next_batch.notifiers);
                    let open = state.is_open;

                    (
                        Batch {
                            channel: T::new(),
                            notifiers,
                        },
                        open,
                    )
                }
            };

            // Run outside of the lock
            current_batch.notifiers.notify_on_take();

            // Post-take barrier: wait here after batch is taken
            #[cfg(all(not(target_arch = "wasm32"), test))]
            self.test_barriers.wait_post_take().await;

            if current_batch.channel.len() > 0 {
                self.retry.reset();
                self.retry_delay.reset();
                self.idle_delay.reset();

                // Re-allocate our next buffer outside of the lock
                next_batch = Batch {
                    channel: T::with_capacity(self.capacity.next(current_batch.channel.len())),
                    notifiers: SenderNotifiers::new(),
                };

                // Track whether the batch completed successfully or was abandoned
                // Assume the batch was abandoned by default
                let mut batch_flushed = false;

                // Emit the batch, taking care not to panic
                loop {
                    match panic::catch_unwind(AssertUnwindSafe(|| on_batch(current_batch.channel)))
                    {
                        Ok(on_batch_future) => {
                            match CatchUnwind(AssertUnwindSafe(on_batch_future)).await {
                                Ok(Ok(())) => {
                                    self.shared.metrics.queue_batch_processed.increment();
                                    batch_flushed = true;
                                    break;
                                }
                                Ok(Err(BatchError { retryable })) => {
                                    self.shared.metrics.queue_batch_failed.increment();

                                    if let Some(retryable) = retryable {
                                        if retryable.len() > 0 && self.retry.next() {
                                            // Delay a bit before trying again; this gives the external service
                                            // a chance to get itself together
                                            wait(Wait::Retry, self.retry_delay.next()).await;

                                            current_batch = Batch {
                                                channel: retryable,
                                                notifiers: current_batch.notifiers,
                                            };

                                            self.shared.metrics.queue_batch_retry.increment();
                                            continue;
                                        }
                                    }

                                    break;
                                }
                                Err(_) => {
                                    self.shared.metrics.queue_batch_panicked.increment();
                                    break;
                                }
                            }
                        }
                        Err(_) => {
                            self.shared.metrics.queue_batch_panicked.increment();
                            break;
                        }
                    }
                }

                // After the batch has been emitted, notify any waiting senders
                current_batch
                    .notifiers
                    .notify_on_flush(batch_flushed, last_batch_flushed);
                last_batch_flushed = batch_flushed;

                // Post-process barrier: wait here after batch is processed
                #[cfg(all(not(target_arch = "wasm32"), test))]
                self.test_barriers.wait_post_process().await;
            }
            // If the batch was empty then notify any waiting senders (there was nothing to flush)
            // and wait before checking again
            else {
                // Notifiers on an empty batch were scheduled while the previous
                // batch was in flight; they get its outcome
                current_batch
                    .notifiers
                    .notify_on_flush(true, last_batch_flushed);

                // Post-process barrier: wait here after empty batch
                #[cfg(all(not(target_arch = "wasm32"), test))]
                self.test_barriers.wait_post_process().await;

                // If the channel is closed then exit the loop and return; this will
                // drop the receiver
                if !is_open {
                    return;
                }

                // If we didn't see any events, then sleep for a bit
                // Idle waits may be cut short by a sender wake (a flush,
                // channel pressure, or the channel closing)
                wait(Wait::Idle, self.idle_delay.next()).await;
            }
        }
    }

    /**
    Get an [`emit::metric::Source`] for instrumentation produced by the channel.

    These metrics can be used to monitor the running health of your diagnostic pipeline.
    */
    pub fn metric_source(&self) -> ChannelMetrics<T> {
        ChannelMetrics {
            shared: self.shared.clone(),
        }
    }
}

/**
An error encountered processing a batch.

The error may contain part of the batch to retry.
*/
#[derive(Debug)]
pub struct BatchError<T> {
    retryable: Option<T>,
}

impl<T> BatchError<T> {
    /**
    An error that can't be retried.
    */
    pub fn no_retry(_: impl std::error::Error + Send + Sync + 'static) -> Self {
        BatchError { retryable: None }
    }

    /**
    An error that can be retried.
    */
    pub fn retry(_: impl std::error::Error + Send + Sync + 'static, retryable: T) -> Self {
        BatchError {
            retryable: Some(retryable),
        }
    }

    /**
    Try convert the error into a retryable value.
    */
    pub fn try_into_retryable(self) -> Result<T, BatchError<T>> {
        self.retryable.ok_or_else(|| BatchError { retryable: None })
    }

    /**
    Try get the retryable batch from the error.

    If the error is not retryable then this method will return `None`.
    */
    pub fn into_retryable(self) -> Option<T> {
        self.retryable
    }

    /**
    Map the retryable batch.

    If the batch is already retryable, the input to `f` will be `Some`. The resulting batch is retryable if `f` returns `Some`.
    */
    pub fn map_retryable<U>(self, f: impl FnOnce(Option<T>) -> Option<U>) -> BatchError<U> {
        BatchError {
            retryable: f(self.retryable),
        }
    }
}

struct TrySendError(&'static str);

impl fmt::Debug for TrySendError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self.0, f)
    }
}

impl fmt::Display for TrySendError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self.0, f)
    }
}

impl error::Error for TrySendError {}

struct CatchUnwind<F>(F);

impl<F: Future + UnwindSafe> Future for CatchUnwind<F> {
    type Output = Result<F::Output, Box<dyn Any + Send>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // SAFETY: `CatchUnwind` uses structural pinning
        let f = unsafe { Pin::map_unchecked_mut(self, |x| &mut x.0) };

        panic::catch_unwind(AssertUnwindSafe(|| f.poll(cx)))?.map(Ok)
    }
}

struct Delay {
    current: Duration,
    step: Duration,
    max: Duration,
}

impl Delay {
    fn new(step: Duration, max: Duration) -> Delay {
        Delay {
            current: Duration::ZERO,
            step,
            max,
        }
    }

    fn reset(&mut self) {
        self.current = Duration::ZERO
    }

    fn next(&mut self) -> Duration {
        self.current = cmp::min(self.current * 2 + self.step, self.max);
        self.current
    }
}

const CAPACITY_WINDOW: usize = 32;

struct Capacity {
    rolling_values: [usize; CAPACITY_WINDOW],
    idx: usize,
}

impl Capacity {
    fn new() -> Self {
        Capacity {
            rolling_values: [0; CAPACITY_WINDOW],
            idx: 0,
        }
    }

    fn next(&mut self, last_len: usize) -> usize {
        self.rolling_values[self.idx % CAPACITY_WINDOW] = last_len;
        self.idx = self.idx.wrapping_add(1);

        let max_len = self.rolling_values.iter().copied().max().unwrap();

        // Add some extra space to accommodate small shifts in size
        // Note that this value is used for initial capacity, but is updated
        // based on the actual length, so adding more space here doesn't mean
        // the `max_len` value will always increase over time
        max_len.saturating_add(cmp::max(1, max_len / 10))
    }
}

struct Retry {
    current: u32,
    max: u32,
}

impl Retry {
    fn new(max: u32) -> Self {
        Retry { current: 0, max }
    }

    fn reset(&mut self) {
        self.current = 0;
    }

    fn next(&mut self) -> bool {
        self.current += 1;
        self.current <= self.max
    }
}

struct Shared<T> {
    metrics: InternalMetrics,
    receiver_notifier: ReceiverNotifier,
    state: Mutex<State<T>>,
}

/**
Whether the receiver is waiting because there's nothing to do, or because it's applying backoff.
*/
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Wait {
    Idle,
    Retry,
}

/**
Metrics produced by a channel.

You can enumerate the metrics using the [`emit::metric::Source`] implementation. See [`emit::metric`] for details.
*/
pub struct ChannelMetrics<T> {
    shared: Arc<Shared<T>>,
}

impl<T: Channel> emit::metric::Source for ChannelMetrics<T> {
    fn sample_metrics<S: emit::metric::sampler::Sampler>(&self, sampler: S) {
        let queue_length = { self.shared.state.lock().unwrap().next_batch.channel.len() };

        self.shared.metrics.sample_metrics(&sampler);

        emit::metric::Metric::new(
            emit::pkg!(),
            emit::Empty,
            emit::props! {
                metric_name: "queue_length",
                metric_agg: "last",
                metric_value: queue_length,
            },
        )
        .sample_metrics(&sampler);
    }
}

struct State<T> {
    next_batch: Batch<T>,
    is_open: bool,
    is_in_batch: bool,
}

struct Batch<T> {
    channel: T,
    notifiers: SenderNotifiers,
}

impl<T: Channel> Batch<T> {
    fn new() -> Self {
        Batch {
            channel: T::new(),
            notifiers: SenderNotifiers::new(),
        }
    }
}

impl<T: Channel> Default for Batch<T> {
    fn default() -> Self {
        Batch::new()
    }
}

/**
A notification channel from the [`Receiver`] to [`Sender`]s.
*/
struct SenderNotifiers {
    on_take: Vec<SenderNotifier>,
    on_flush: Vec<SenderFlushNotifier>,
}

type SenderNotifier = Box<dyn FnOnce() + Send>;

struct SenderFlushNotifier {
    chain_with_last_batch: bool,
    notify: Box<dyn FnOnce(bool) + Send>,
}

impl Default for SenderNotifiers {
    fn default() -> Self {
        SenderNotifiers::new()
    }
}

impl SenderNotifiers {
    fn new() -> Self {
        SenderNotifiers {
            on_take: Vec::new(),
            on_flush: Vec::new(),
        }
    }

    fn push_on_flush(&mut self, chain_with_last_batch: bool, notify: Box<dyn FnOnce(bool) + Send>) {
        self.on_flush.push(SenderFlushNotifier {
            chain_with_last_batch,
            notify,
        });
    }

    fn notify_on_flush(&mut self, target_batch_flushed: bool, last_batch_flushed: bool) {
        for notifier in mem::take(&mut self.on_flush) {
            let flushed = if notifier.chain_with_last_batch {
                // Success depends on both the batch this notifier was attached to,
                // and the batch that preceded it
                target_batch_flushed && last_batch_flushed
            } else {
                // Success depends only on the batch this notifier was attached to
                target_batch_flushed
            };

            let notify = notifier.notify;

            let _ = panic::catch_unwind(AssertUnwindSafe(move || notify(flushed)));
        }
    }

    fn push_on_take(&mut self, notifier: SenderNotifier) {
        self.on_take.push(notifier);
    }

    fn notify_on_take(&mut self) {
        for notifier in mem::take(&mut self.on_take) {
            let _ = panic::catch_unwind(AssertUnwindSafe(notifier));
        }
    }
}

/**
A notification channel from [`Sender`]s to the [`Receiver`].
*/
struct ReceiverNotifier {
    state: Mutex<ReceiverNotifierState>,
    sync: sync::Trigger,
    #[cfg(feature = "tokio")]
    tokio: tokio::Trigger,
}

struct ReceiverNotifierState {
    notified: bool,
}

impl ReceiverNotifier {
    fn new() -> Self {
        ReceiverNotifier {
            state: Mutex::new(ReceiverNotifierState { notified: false }),
            sync: sync::Trigger::new(),
            #[cfg(feature = "tokio")]
            tokio: tokio::Trigger::new(),
        }
    }

    fn notify(&self) {
        let mut state = self.state.lock().unwrap();

        // If a notification is already pending then any waiter has already been woken
        if state.notified {
            return;
        }

        state.notified = true;

        self.sync.trigger(true);

        #[cfg(feature = "tokio")]
        {
            self.tokio.trigger(true);
        }
    }
}

pub mod sync;

#[cfg(all(
    feature = "tokio",
    not(all(
        target_arch = "wasm32",
        target_vendor = "unknown",
        target_os = "unknown"
    ))
))]
pub mod tokio;

#[cfg(feature = "web")]
#[cfg(all(
    target_arch = "wasm32",
    target_vendor = "unknown",
    target_os = "unknown"
))]
pub mod web;

// Re-export an appropriate implementation of blocking functions based on crate features

#[cfg(all(
    feature = "tokio",
    not(all(
        target_arch = "wasm32",
        target_vendor = "unknown",
        target_os = "unknown"
    ))
))]
pub use tokio::{blocking_flush, blocking_send};

#[cfg(not(all(
    feature = "tokio",
    not(all(
        target_arch = "wasm32",
        target_vendor = "unknown",
        target_os = "unknown"
    ))
)))]
pub use sync::{blocking_flush, blocking_send};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capacity() {
        let mut capacity = Capacity::new();

        let next = capacity.next(10);

        assert_eq!(11, next);

        let next = capacity.next(5);

        assert_eq!(11, next);

        let next = capacity.next(100);

        assert_eq!(110, next);

        for _ in 0..CAPACITY_WINDOW {
            let _ = capacity.next(0);
        }

        let next = capacity.next(0);

        assert_eq!(1, next);
    }
}
