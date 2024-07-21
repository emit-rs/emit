/*!
Infrastructure for emitting diagnostic events in the background.

This library implements a channel that can be used to spawn background workers on a dedicated thread or `tokio` runtime. The channel implements:

- **Batching:** Events written to the channel are processed by the worker in batches rather than one-at-a-time.
- **Retries with backoff:** If the worker fails or panics then the batch can be retried up to some number of times, with backoff applied between retries. The worker can decide how much of a batch needs to be retried.
- **Maximum size management:** If the worker can't keep up then the channel truncates to avoid runaway memory use. The alternative would be to apply backpressure, but that would affect system availability so isn't suitable for diagnostics.
- **Flushing:** Callers can ask the worker to signal when all diagnostic events in the channel at the point they called are processed. This can be used for auditing and flushing on shutdown.

# Status

This library is still experimental, so its API may change.
*/

#![doc(html_logo_url = "https://raw.githubusercontent.com/emit-rs/emit/main/asset/logo.svg")]
#![deny(missing_docs)]

use crate::internal_metrics::InternalMetrics;
use std::{
    any::Any,
    cmp,
    future::{self, Future},
    mem,
    panic::{self, AssertUnwindSafe, UnwindSafe},
    pin::{pin, Pin},
    sync::{Arc, Mutex, OnceLock},
    task,
    task::{Context, Poll},
    thread,
    time::Duration,
};

mod internal_metrics;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

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

Pass the receiver to [`tokio::spawn`] to spawn a background task that processes batches of items. You can also create a thread manually and call [`Receiver::blocking_exec`] on it.
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
            self.shared.metrics.queue_overflow.increment();
        }

        // If the channel is closed then return without adding the message
        if !state.is_open {
            return;
        }

        state.next_batch.channel.push(msg);
    }

    /**
    Set a callback to fire when all items in the active batch are processed by the [`Receiver`].
    */
    pub fn on_next_flush(&self, watcher: impl FnOnce() + Send + 'static) {
        let watcher = Box::new(watcher);

        let mut state = self.shared.state.lock().unwrap();

        // If:
        // - We're not in a batch and
        //   - the next batch is empty (there's no data) or
        //   - the state is closed
        // Then:
        // - Call the watcher without scheduling it; there's nothing to wait for
        if !state.is_in_batch && (state.next_batch.channel.is_empty() || !state.is_open) {
            // Drop the lock before signalling the watcher
            drop(state);

            watcher();
        }
        // If there's active data to flush then schedule the watcher
        else {
            state.next_batch.watchers.push(watcher);
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

Use [`crate::tokio::spawn`], or [`Receiver::exec`] or [`Receiver::blocking_exec`] in a dedicated thread to run the receiver as a background worker.
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

        // TODO: Trigger callback for the current batch?
    }
}

impl<T: Channel> Receiver<T> {
    /**
    Run the receiver synchronously.

    This method should be called on a dedicated thread. It will return once the [`Sender`] is dropped.
    */
    pub fn blocking_exec(
        self,
        mut on_batch: impl FnMut(T) -> Result<(), BatchError<T>>,
    ) -> Result<(), Error> {
        static WAKER: OnceLock<Arc<NeverWake>> = OnceLock::new();

        // A waker that does nothing; the tasks it runs are fully
        // synchronous so there's never any notifications to issue
        struct NeverWake;

        impl task::Wake for NeverWake {
            fn wake(self: Arc<Self>) {}
        }

        // The future is polled to completion here, so we can pin
        // it directly on the stack
        let mut fut = pin!(self.exec(
            |delay| future::ready(thread::sleep(delay)),
            move |batch| future::ready(on_batch(batch)),
        ));

        // Get a context for our synchronous task
        let waker = WAKER.get_or_init(|| Arc::new(NeverWake)).clone().into();
        let mut cx = task::Context::from_waker(&waker);

        // Drive the task to completion; it should complete in one go,
        // but may eagerly return as soon as it hits an await point, so
        // just to be sure we continuously poll it
        loop {
            match fut.as_mut().poll(&mut cx) {
                task::Poll::Ready(r) => return r,
                task::Poll::Pending => continue,
            }
        }
    }

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
    ) -> Result<(), Error> {
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
                current_batch.watchers.notify();
            }
            // If the batch was empty then notify any watchers (there was nothing to flush)
            // and wait before checking again
            else {
                current_batch.watchers.notify();

                // If the channel is closed then exit the loop and return; this will
                // drop the receiver
                if !is_open {
                    return Ok(());
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
                emit::Path::new_unchecked(env!("CARGO_PKG_NAME")),
                emit::empty::Empty,
                "queue_length",
                emit::well_known::METRIC_AGG_LAST,
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

struct Watchers(Vec<Watcher>);

type Watcher = Box<dyn FnOnce() + Send>;

impl Default for Watchers {
    fn default() -> Self {
        Watchers::new()
    }
}

impl Watchers {
    fn new() -> Self {
        Watchers(Vec::new())
    }

    fn push(&mut self, watcher: Watcher) {
        self.0.push(watcher);
    }

    fn notify(self) {
        for watcher in self.0 {
            let _ = panic::catch_unwind(AssertUnwindSafe(watcher));
        }
    }
}

pub mod sync;

#[cfg(feature = "tokio")]
pub mod tokio;
