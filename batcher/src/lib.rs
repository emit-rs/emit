/*!
Infrastructure for emitting diagnostic events in the background.

This library implements a channel that can be used to spawn background workers on a dedicated thread or `tokio` runtime. The channel implements:

- **Batching:** Events written to the channel are processed by the worker in batches rather than one-at-a-time.
- **Retries with backoff:** If the worker fails or panics then the batch can be retried up to some number of times, with backoff applied between retries. The worker can decide how much of a batch needs to be retried.
- **Maximum size management:** If the worker can't keep up then the channel truncates to avoid runaway memory use. The alternative would be to apply backpressure, but that would affect system availability so isn't suitable for diagnostics.
- **Flushing:** Callers can ask the worker to signal when all diagnostic events in the channel at the point they called are processed. This can be used for auditing and flushing on shutdown.
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
    time::{Duration, Instant},
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
        Receiver {
            idle_delay: Delay::new(Duration::from_millis(1), Duration::from_millis(500)),
            retry: Retry::new(10),
            retry_delay: Delay::new(Duration::from_millis(50), Duration::from_secs(1)),
            capacity: Capacity::new(),
            shared,
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

            Ok(())
        } else {
            Err(BatchError::retry(TrySendError("the channel is full"), msg))
        }
    }

    async fn send_or_wait<'a, FWait: Future<Output = ()> + 'a>(
        &'a self,
        msg: T::Item,
        timeout: Duration,
        mut wait_until_empty: impl FnMut(&'a Self, Duration) -> FWait,
    ) -> Result<(), BatchError<T::Item>> {
        match self.try_send(msg) {
            // If the message was sent then return
            Ok(()) => Ok(()),
            // If the message wasn't sent then wait until the next batch is taken then try again
            Err(mut err) => {
                self.shared.metrics.queue_full_blocked.increment();

                let now = Instant::now();

                while now.elapsed() < timeout {
                    wait_until_empty(self, timeout.saturating_sub(now.elapsed())).await;

                    // NOTE: Between being triggered and calling, we may have filled up again
                    match self.try_send(err.try_into_retryable()?) {
                        Ok(()) => return Ok(()),
                        Err(retry) => {
                            err = retry;
                            continue;
                        }
                    }
                }

                Err(err)
            }
        }
    }

    /**
    Set a callback to fire when the next batch is taken.

    The watcher is guaranteed to trigger at a point where the current batch is empty.
    */
    pub fn when_empty(&self, f: impl FnOnce() + Send + 'static) {
        let mut state = self.shared.state.lock().unwrap();

        // If:
        // - The next batch is empty
        // Then:
        // - Call the watcher without scheduling it; there's nothing to wait for
        if state.next_batch.channel.is_empty() {
            drop(state);

            f();
        } else {
            state.next_batch.watchers.push_on_take(Box::new(f));
        }
    }

    /**
    Set a callback to fire when all items in the active batch are processed by the [`Receiver`].

    The watcher is guaranteed to trigger at a point where the batch that was processing at the time this call was made has completed.
    */
    pub fn when_flushed(&self, f: impl FnOnce() + Send + 'static) {
        let mut state = self.shared.state.lock().unwrap();

        // If:
        // - We're not in a batch and
        //   - the next batch is empty (there's no data) or
        //   - the state is closed
        // Then:
        // - Call the watcher without scheduling it; there's nothing to flush
        if !state.is_in_batch && (state.next_batch.channel.is_empty() || !state.is_open) {
            // Drop the lock before signalling the watcher
            drop(state);

            f();
        }
        // If there's active data to flush then schedule the watcher
        else {
            state.next_batch.watchers.push_on_flush(Box::new(f));
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
The receiving half of a channel.

Use [`Receiver::exec`], [`crate::tokio::spawn`], or [`crate::sync::spawn`] to run the receiver as a background worker.
*/
pub struct Receiver<T> {
    idle_delay: Delay,
    retry: Retry,
    retry_delay: Delay,
    capacity: Capacity,
    shared: Arc<Shared<T>>,
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
        mut self,
        mut wait: impl FnMut(Duration) -> FWait,
        mut on_batch: impl FnMut(T) -> FBatch,
    ) {
        // This variable holds the "next" batch
        // Under the lock all we do is push onto a pre-allocated vec
        // and replace it with another pre-allocated vec
        let mut next_batch = Batch::new();

        loop {
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
                // If there are no events to emit then mark that we're outside of a batch and take its watchers
                else {
                    state.is_in_batch = false;

                    let watchers = mem::take(&mut state.next_batch.watchers);
                    let open = state.is_open;

                    (
                        Batch {
                            channel: T::new(),
                            watchers,
                        },
                        open,
                    )
                }
            };

            // Run outside of the lock
            current_batch.watchers.notify_on_take();

            if current_batch.channel.len() > 0 {
                self.retry.reset();
                self.retry_delay.reset();
                self.idle_delay.reset();

                // Re-allocate our next buffer outside of the lock
                next_batch = Batch {
                    channel: T::with_capacity(self.capacity.next(current_batch.channel.len())),
                    watchers: Watchers::new(),
                };

                // Emit the batch, taking care not to panic
                loop {
                    match panic::catch_unwind(AssertUnwindSafe(|| on_batch(current_batch.channel)))
                    {
                        Ok(on_batch_future) => {
                            match CatchUnwind(AssertUnwindSafe(on_batch_future)).await {
                                Ok(Ok(())) => {
                                    self.shared.metrics.queue_batch_processed.increment();
                                    break;
                                }
                                Ok(Err(BatchError { retryable })) => {
                                    self.shared.metrics.queue_batch_failed.increment();

                                    if let Some(retryable) = retryable {
                                        if retryable.len() > 0 && self.retry.next() {
                                            // Delay a bit before trying again; this gives the external service
                                            // a chance to get itself together
                                            wait(self.retry_delay.next()).await;

                                            current_batch = Batch {
                                                channel: retryable,
                                                watchers: current_batch.watchers,
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

                // After the batch has been emitted, notify any watchers
                current_batch.watchers.notify_on_flush();
            }
            // If the batch was empty then notify any watchers (there was nothing to flush)
            // and wait before checking again
            else {
                current_batch.watchers.notify_on_flush();

                // If the channel is closed then exit the loop and return; this will
                // drop the receiver
                if !is_open {
                    return;
                }

                // If we didn't see any events, then sleep for a bit
                wait(self.idle_delay.next()).await;
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

const CAPACITY_WINDOW: usize = 16;

struct Capacity([usize; CAPACITY_WINDOW], usize);

impl Capacity {
    fn new() -> Self {
        Capacity([1; CAPACITY_WINDOW], 0)
    }

    fn next(&mut self, last_len: usize) -> usize {
        self.0[self.1 % CAPACITY_WINDOW] = last_len;
        self.0.iter().copied().max().unwrap()
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
    state: Mutex<State<T>>,
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

        let metrics = self
            .shared
            .metrics
            .sample()
            .chain(Some(emit::metric::Metric::new(
                emit::pkg!(),
                "queue_length",
                emit::well_known::METRIC_AGG_LAST,
                emit::empty::Empty,
                queue_length,
                emit::empty::Empty,
            )));

        for metric in metrics {
            sampler.metric(metric);
        }
    }
}

struct State<T> {
    next_batch: Batch<T>,
    is_open: bool,
    is_in_batch: bool,
}

struct Batch<T> {
    channel: T,
    watchers: Watchers,
}

impl<T: Channel> Batch<T> {
    fn new() -> Self {
        Batch {
            channel: T::new(),
            watchers: Watchers::new(),
        }
    }
}

impl<T: Channel> Default for Batch<T> {
    fn default() -> Self {
        Batch::new()
    }
}

struct Watchers {
    on_take: Vec<Watcher>,
    on_flush: Vec<Watcher>,
}

type Watcher = Box<dyn FnOnce() + Send>;

impl Default for Watchers {
    fn default() -> Self {
        Watchers::new()
    }
}

impl Watchers {
    fn new() -> Self {
        Watchers {
            on_take: Vec::new(),
            on_flush: Vec::new(),
        }
    }

    fn push_on_flush(&mut self, watcher: Watcher) {
        self.on_flush.push(watcher);
    }

    fn notify_on_flush(&mut self) {
        for watcher in mem::take(&mut self.on_flush) {
            let _ = panic::catch_unwind(AssertUnwindSafe(watcher));
        }
    }

    fn push_on_take(&mut self, watcher: Watcher) {
        self.on_take.push(watcher);
    }

    fn notify_on_take(&mut self) {
        for watcher in mem::take(&mut self.on_take) {
            let _ = panic::catch_unwind(AssertUnwindSafe(watcher));
        }
    }
}

pub mod sync;

#[cfg(feature = "tokio")]
pub mod tokio;

// Re-export an appropriate implementation of blocking functions based on crate features

#[cfg(feature = "tokio")]
pub use tokio::{blocking_flush, blocking_send};

#[cfg(not(feature = "tokio"))]
pub use sync::{blocking_flush, blocking_send};
