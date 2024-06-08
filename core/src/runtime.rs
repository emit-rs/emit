/*!
The [`Runtime`] type.

Runtimes combine components into a fully encapsulated diagnostic pipeline. Each runtime includes:

- An [`Emitter`] to receive diagnostic events.
- A [`Filter`] to limit the volume of diagnostic events.
- A [`Ctxt`] to capture and attach ambient state to events.
- A [`Clock`] to timestamp events.
- A [`Rng`] to generate correlation ids for events.

Runtimes are fully isolated and may be short-lived. A [`Runtime`] can be treated generically, or erased behind an [`AmbientSlot`] for global sharing. This module defines two global runtimes; the [`shared()`] runtime, and the [`internal()`] runtime. Applications should emit their events through the [`shared()`] runtime. Code running within a runtime itself, such as an implementation of [`Emitter`] should emit their events through the [`internal()`] runtime.

The [`internal()`] runtime can only be initialized with components that also satisfy internal versions of their regular traits. These marker traits require a component not produce any diagnostics of its own, and so are safe to use by another runtime. If components in the [`internal()`] runtime could produce their own diagnostics then it could cause loops and stack overflows.

If an application is initializing both the [`shared()`] and [`internal()`] runtimes, then it should initialize the [`internal()`] runtime _first_.
*/

use crate::{
    clock::Clock, ctxt::Ctxt, emitter::Emitter, empty::Empty, event::ToEvent, extent::ToExtent,
    filter::Filter, props::Props, rng::Rng, timestamp::Timestamp,
};

#[cfg(feature = "implicit_rt")]
static SHARED: AmbientSlot = AmbientSlot::new();
#[cfg(feature = "implicit_internal_rt")]
static INTERNAL: AmbientInternalSlot = AmbientInternalSlot::new();

/**
The global shared runtime for applications to use.

This runtime needs to be initialized through its [`shared_slot()`], otherwise it will use [`Empty`] implementations of its components.
*/
#[cfg(feature = "implicit_rt")]
pub fn shared() -> &'static AmbientRuntime<'static> {
    SHARED.get()
}

/**
The initialization slot for the [`shared()`] runtime.
*/
#[cfg(feature = "implicit_rt")]
pub fn shared_slot() -> &'static AmbientSlot {
    &SHARED
}

/**
The internal runtime for other runtime components to use.

Applications should use the [`shared()`] runtime instead of this one.

This runtime can be initialized through its [`internal_slot()`] to enable diagnostics on the regular diagnostics runtime itself.
*/
#[cfg(feature = "implicit_internal_rt")]
pub fn internal() -> &'static AmbientRuntime<'static> {
    INTERNAL.get()
}

/**
The initialization slot for the [`internal()`] runtime.

This slot should be initialized _before_ the [`shared_slot()`] if it's in use.
*/
#[cfg(feature = "implicit_internal_rt")]
pub fn internal_slot() -> &'static AmbientInternalSlot {
    &INTERNAL
}

/**
A diagnostic pipeline.

Each runtime includes the following components:

- An [`Emitter`] to receive diagnostic events.
- A [`Filter`] to limit the volume of diagnostic events.
- A [`Ctxt`] to capture and attach ambient state to events.
- A [`Clock`] to timestamp events.
- A [`Rng`] to generate correlation ids for events.

The components of a runtime can be accessed directly through methods. A runtime can be treated like a builder to set its components, or initialized with them all directly.

In statics, you can also use the [`AmbientSlot`] type to hold a type-erased runtime. It's also reasonable to store a fully generic runtime in a static too.
*/
#[derive(Debug, Clone, Copy)]
pub struct Runtime<TEmitter = Empty, TFilter = Empty, TCtxt = Empty, TClock = Empty, TRng = Empty> {
    pub(crate) emitter: TEmitter,
    pub(crate) filter: TFilter,
    pub(crate) ctxt: TCtxt,
    pub(crate) clock: TClock,
    pub(crate) rng: TRng,
}

impl Default for Runtime {
    fn default() -> Self {
        Runtime::new()
    }
}

impl Runtime {
    /**
    Create a new, empty runtime.
    */
    pub const fn new() -> Runtime {
        Runtime {
            emitter: Empty,
            filter: Empty,
            ctxt: Empty,
            clock: Empty,
            rng: Empty,
        }
    }
}

impl<TEmitter, TFilter, TCtxt, TClock, TRng> Runtime<TEmitter, TFilter, TCtxt, TClock, TRng> {
    /**
    Create a new runtime with the given components.
    */
    pub const fn build(
        emitter: TEmitter,
        filter: TFilter,
        ctxt: TCtxt,
        clock: TClock,
        rng: TRng,
    ) -> Self {
        Runtime {
            emitter,
            filter,
            ctxt,
            clock,
            rng,
        }
    }

    /**
    Get the [`Emitter`].
    */
    pub const fn emitter(&self) -> &TEmitter {
        &self.emitter
    }

    /**
    Set the [`Emitter`].
    */
    pub fn with_emitter<U>(self, emitter: U) -> Runtime<U, TFilter, TCtxt, TClock, TRng> {
        self.map_emitter(|_| emitter)
    }

    /**
    Map the current [`Emitter`] to a new value.
    */
    pub fn map_emitter<U>(
        self,
        emitter: impl FnOnce(TEmitter) -> U,
    ) -> Runtime<U, TFilter, TCtxt, TClock, TRng> {
        Runtime {
            emitter: emitter(self.emitter),
            filter: self.filter,
            ctxt: self.ctxt,
            clock: self.clock,
            rng: self.rng,
        }
    }

    /**
    Get the [`Filter`].
    */
    pub const fn filter(&self) -> &TFilter {
        &self.filter
    }

    /**
    Set the [`Filter`].
    */
    pub fn with_filter<U>(self, filter: U) -> Runtime<TEmitter, U, TCtxt, TClock, TRng> {
        self.map_filter(|_| filter)
    }

    /**
    Map the current [`Filter`] to a new value.
    */
    pub fn map_filter<U>(
        self,
        filter: impl FnOnce(TFilter) -> U,
    ) -> Runtime<TEmitter, U, TCtxt, TClock, TRng> {
        Runtime {
            emitter: self.emitter,
            filter: filter(self.filter),
            ctxt: self.ctxt,
            clock: self.clock,
            rng: self.rng,
        }
    }

    /**
    Get the [`Ctxt`].
    */
    pub const fn ctxt(&self) -> &TCtxt {
        &self.ctxt
    }

    /**
    Set the [`Ctxt`].
    */
    pub fn with_ctxt<U>(self, ctxt: U) -> Runtime<TEmitter, TFilter, U, TClock, TRng> {
        self.map_ctxt(|_| ctxt)
    }

    /**
    Map the current [`Ctxt`] to a new value.
    */
    pub fn map_ctxt<U>(
        self,
        ctxt: impl FnOnce(TCtxt) -> U,
    ) -> Runtime<TEmitter, TFilter, U, TClock, TRng> {
        Runtime {
            emitter: self.emitter,
            filter: self.filter,
            ctxt: ctxt(self.ctxt),
            clock: self.clock,
            rng: self.rng,
        }
    }

    /**
    Get the [`Clock`].
    */
    pub const fn clock(&self) -> &TClock {
        &self.clock
    }

    /**
    Set the [`Clock`].
    */
    pub fn with_clock<U>(self, clock: U) -> Runtime<TEmitter, TFilter, TCtxt, U, TRng> {
        self.map_clock(|_| clock)
    }

    /**
    Map the current [`Clock`] to a new value.
    */
    pub fn map_clock<U>(
        self,
        clock: impl FnOnce(TClock) -> U,
    ) -> Runtime<TEmitter, TFilter, TCtxt, U, TRng> {
        Runtime {
            emitter: self.emitter,
            filter: self.filter,
            ctxt: self.ctxt,
            clock: clock(self.clock),
            rng: self.rng,
        }
    }

    /**
    Get the [`Rng`].
    */
    pub const fn rng(&self) -> &TRng {
        &self.rng
    }

    /**
    Set the [`Rng`].
    */
    pub fn with_rng<U>(self, id_gen: U) -> Runtime<TEmitter, TFilter, TCtxt, TClock, U> {
        self.map_rng(|_| id_gen)
    }

    /**
    Map the current [`Rng`] to a new value.
    */
    pub fn map_rng<U>(
        self,
        id_gen: impl FnOnce(TRng) -> U,
    ) -> Runtime<TEmitter, TFilter, TCtxt, TClock, U> {
        Runtime {
            emitter: self.emitter,
            filter: self.filter,
            ctxt: self.ctxt,
            clock: self.clock,
            rng: id_gen(self.rng),
        }
    }
}

impl<TEmitter: Emitter, TFilter: Filter, TCtxt: Ctxt, TClock: Clock, TRng: Rng>
    Runtime<TEmitter, TFilter, TCtxt, TClock, TRng>
{
    /**
    Emit a diagnostic event through the runtime.

    This method uses the components of the runtime to process the event. It will:

    1. Attempt to assign an extent to the event using [`Clock::now`] if the event doesn't already have one.
    2. Add [`Ctxt::Current`] to the event properties.
    3. Ensure the event passes [`Filter::matches`].
    4. Emit the event through [`Emitter::emit`].

    You can bypass any of these steps by emitting the event directly through the runtime's [`Emitter`].
    */
    pub fn emit<E: ToEvent>(&self, evt: E) {
        self.ctxt.with_current(|ctxt| {
            let evt = evt.to_event();

            let extent = evt
                .extent()
                .cloned()
                .or_else(|| self.clock.now().to_extent());

            let evt = evt
                .with_extent(extent)
                .map_props(|props| props.and_props(ctxt));

            if self.filter.matches(&evt) {
                self.emitter.emit(evt);
            }
        });
    }
}

impl<TEmitter: Emitter, TFilter: Filter, TCtxt: Ctxt, TClock: Clock, TRng: Rng> Emitter
    for Runtime<TEmitter, TFilter, TCtxt, TClock, TRng>
{
    fn emit<E: ToEvent>(&self, evt: E) {
        self.emit(evt)
    }

    fn blocking_flush(&self, timeout: core::time::Duration) -> bool {
        self.emitter.blocking_flush(timeout)
    }
}

/**
A marker trait for an [`Emitter`] that does not emit any diagnostics of its own.
*/
pub trait InternalEmitter: Emitter {}

impl<T: Emitter> InternalEmitter for AssertInternal<T> {}

impl InternalEmitter for Empty {}

impl<T: InternalEmitter, U: InternalEmitter> InternalEmitter for crate::and::And<T, U> {}

#[cfg(feature = "alloc")]
impl<'a, T: ?Sized + InternalEmitter> InternalEmitter for alloc::boxed::Box<T> {}

#[cfg(feature = "alloc")]
impl<'a, T: ?Sized + InternalEmitter> InternalEmitter for alloc::sync::Arc<T> {}

/**
A marker trait for a [`Filter`] that does not emit any diagnostics of its own.
*/
pub trait InternalFilter: Filter {}

impl<T: Filter> InternalFilter for AssertInternal<T> {}

impl InternalFilter for Empty {}

impl<T: InternalFilter, U: InternalFilter> InternalFilter for crate::and::And<T, U> {}

impl<T: InternalFilter, U: InternalFilter> InternalFilter for crate::or::Or<T, U> {}

impl<T: InternalFilter, U: InternalEmitter> InternalEmitter
    for crate::filter::FilteredEmitter<T, U>
{
}

#[cfg(feature = "alloc")]
impl<'a, T: ?Sized + InternalFilter> InternalFilter for alloc::boxed::Box<T> {}

#[cfg(feature = "alloc")]
impl<'a, T: ?Sized + InternalFilter> InternalFilter for alloc::sync::Arc<T> {}

/**
A marker trait for a [`Ctxt`] that does not emit any diagnostics of its own.
*/
pub trait InternalCtxt: Ctxt {}

impl<T: Ctxt> InternalCtxt for AssertInternal<T> {}

impl InternalCtxt for Empty {}

#[cfg(feature = "alloc")]
impl<'a, T: ?Sized + InternalCtxt> InternalCtxt for alloc::boxed::Box<T> {}

#[cfg(feature = "alloc")]
impl<'a, T: ?Sized + InternalCtxt> InternalCtxt for alloc::sync::Arc<T> {}

/**
A marker trait for a [`Clock`] that does not emit any diagnostics of its own.
*/
pub trait InternalClock: Clock {}

impl<T: Clock> InternalClock for AssertInternal<T> {}

impl InternalClock for Empty {}

#[cfg(feature = "alloc")]
impl<'a, T: ?Sized + InternalClock> InternalClock for alloc::boxed::Box<T> {}

#[cfg(feature = "alloc")]
impl<'a, T: ?Sized + InternalClock> InternalClock for alloc::sync::Arc<T> {}

/**
A marker trait for an [`Rng`] that does not emit any diagnostics of its own.
*/
pub trait InternalRng: Rng {}

impl<T: Rng> InternalRng for AssertInternal<T> {}

impl InternalRng for Empty {}

#[cfg(feature = "alloc")]
impl<'a, T: ?Sized + InternalRng> InternalRng for alloc::boxed::Box<T> {}

#[cfg(feature = "alloc")]
impl<'a, T: ?Sized + InternalRng> InternalRng for alloc::sync::Arc<T> {}

/**
Assert that a given component does not emit any diagnostics of its own.
*/
pub struct AssertInternal<T>(pub T);

impl<T: Emitter> Emitter for AssertInternal<T> {
    fn emit<E: ToEvent>(&self, evt: E) {
        self.0.emit(evt)
    }

    fn blocking_flush(&self, timeout: core::time::Duration) -> bool {
        self.0.blocking_flush(timeout)
    }
}

impl<T: Filter> Filter for AssertInternal<T> {
    fn matches<E: ToEvent>(&self, evt: E) -> bool {
        self.0.matches(evt)
    }
}

impl<T: Ctxt> Ctxt for AssertInternal<T> {
    type Current = T::Current;
    type Frame = T::Frame;

    fn open_root<P: Props>(&self, props: P) -> Self::Frame {
        self.0.open_root(props)
    }

    fn open_push<P: Props>(&self, props: P) -> Self::Frame {
        self.0.open_push(props)
    }

    fn enter(&self, local: &mut Self::Frame) {
        self.0.enter(local)
    }

    fn with_current<R, F: FnOnce(&Self::Current) -> R>(&self, with: F) -> R {
        self.0.with_current(with)
    }

    fn exit(&self, local: &mut Self::Frame) {
        self.0.exit(local)
    }

    fn close(&self, frame: Self::Frame) {
        self.0.close(frame)
    }
}

impl<T: Clock> Clock for AssertInternal<T> {
    fn now(&self) -> Option<Timestamp> {
        self.0.now()
    }
}

impl<T: Rng> Rng for AssertInternal<T> {
    fn fill<A: AsMut<[u8]>>(&self, arr: A) -> Option<A> {
        self.0.fill(arr)
    }

    fn gen_u64(&self) -> Option<u64> {
        self.0.gen_u64()
    }

    fn gen_u128(&self) -> Option<u128> {
        self.0.gen_u128()
    }
}

#[cfg(feature = "std")]
mod std_support {
    use alloc::boxed::Box;
    use core::any::Any;
    use std::sync::OnceLock;

    use crate::{
        clock::ErasedClock, ctxt::ErasedCtxt, emitter::ErasedEmitter, filter::ErasedFilter,
        rng::ErasedRng,
    };

    use super::*;

    /**
    A type-erased [`Emitter`] for an [`AmbientSlot`].
    */
    pub type AmbientEmitter<'a> = &'a (dyn ErasedEmitter + Send + Sync + 'static);

    trait AnyEmitter: Any + ErasedEmitter + Send + Sync + 'static {
        fn as_any(&self) -> &dyn Any;
        fn as_super(&self) -> &(dyn ErasedEmitter + Send + Sync + 'static);
    }

    impl<T: ErasedEmitter + Send + Sync + 'static> AnyEmitter for T {
        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_super(&self) -> &(dyn ErasedEmitter + Send + Sync + 'static) {
            self
        }
    }

    /**
    A type-erased [`Filter`] for an [`AmbientSlot`].
    */
    pub type AmbientFilter<'a> = &'a (dyn ErasedFilter + Send + Sync + 'static);

    trait AnyFilter: Any + ErasedFilter + Send + Sync + 'static {
        fn as_any(&self) -> &dyn Any;
        fn as_super(&self) -> &(dyn ErasedFilter + Send + Sync + 'static);
    }

    impl<T: ErasedFilter + Send + Sync + 'static> AnyFilter for T {
        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_super(&self) -> &(dyn ErasedFilter + Send + Sync + 'static) {
            self
        }
    }

    /**
    A type-erased [`Ctxt`] for an [`AmbientSlot`].
    */
    pub type AmbientCtxt<'a> = &'a (dyn ErasedCtxt + Send + Sync + 'static);

    trait AnyCtxt: Any + ErasedCtxt + Send + Sync + 'static {
        fn as_any(&self) -> &dyn Any;
        fn as_super(&self) -> &(dyn ErasedCtxt + Send + Sync + 'static);
    }

    impl<T: ErasedCtxt + Send + Sync + 'static> AnyCtxt for T {
        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_super(&self) -> &(dyn ErasedCtxt + Send + Sync + 'static) {
            self
        }
    }

    /**
    A type-erased [`Clock`] for an [`AmbientSlot`].
    */
    pub type AmbientClock<'a> = &'a (dyn ErasedClock + Send + Sync + 'static);

    trait AnyClock: Any + ErasedClock + Send + Sync + 'static {
        fn as_any(&self) -> &dyn Any;
        fn as_super(&self) -> &(dyn ErasedClock + Send + Sync + 'static);
    }

    impl<T: ErasedClock + Send + Sync + 'static> AnyClock for T {
        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_super(&self) -> &(dyn ErasedClock + Send + Sync + 'static) {
            self
        }
    }

    /**
    A type-erased [`Rng`] for an [`AmbientSlot`].
    */
    pub type AmbientRng<'a> = &'a (dyn ErasedRng + Send + Sync + 'static);

    trait AnyRng: Any + ErasedRng + Send + Sync + 'static {
        fn as_any(&self) -> &dyn Any;
        fn as_super(&self) -> &(dyn ErasedRng + Send + Sync + 'static);
    }

    impl<T: ErasedRng + Send + Sync + 'static> AnyRng for T {
        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_super(&self) -> &(dyn ErasedRng + Send + Sync + 'static) {
            self
        }
    }

    /**
    A type-erased slot for a globally shared [`Runtime`].

    The slot is suitable to store directly in a static; it coordinates its own initialization using a [`OnceLock`].
    */
    pub struct AmbientSlot(OnceLock<AmbientSync>);

    /**
    A type-erased slot for the [`internal()`] runtime.
    */
    #[cfg(feature = "implicit_internal_rt")]
    pub struct AmbientInternalSlot(AmbientSlot);

    struct AmbientSync {
        value: AmbientSyncValue,
        runtime: AmbientSyncRuntime,
    }

    type AmbientSyncValue = Runtime<
        Box<dyn AnyEmitter + Send + Sync>,
        Box<dyn AnyFilter + Send + Sync>,
        Box<dyn AnyCtxt + Send + Sync>,
        Box<dyn AnyClock + Send + Sync>,
        Box<dyn AnyRng + Send + Sync>,
    >;

    type AmbientSyncRuntime = Runtime<
        *const (dyn ErasedEmitter + Send + Sync),
        *const (dyn ErasedFilter + Send + Sync),
        *const (dyn ErasedCtxt + Send + Sync),
        *const (dyn ErasedClock + Send + Sync),
        *const (dyn ErasedRng + Send + Sync),
    >;

    /**
    A type-erased [`Runtime`].
    */
    pub type AmbientRuntime<'a> = Runtime<
        AmbientEmitter<'a>,
        AmbientFilter<'a>,
        AmbientCtxt<'a>,
        AmbientClock<'a>,
        AmbientRng<'a>,
    >;

    unsafe impl Send for AmbientSync where AmbientSyncValue: Send {}
    unsafe impl Sync for AmbientSync where AmbientSyncValue: Sync {}

    impl AmbientSlot {
        /**
        Create a new, empty slot.
        */
        pub const fn new() -> Self {
            AmbientSlot(OnceLock::new())
        }

        /**
        Whether the slot has been initialized with a runtime.
        */
        pub fn is_enabled(&self) -> bool {
            self.0.get().is_some()
        }

        /**
        Try initialize the slot with the given components.

        If the slot has not already been initialized then the components will be installed and a reference to the resulting [`Runtime`] will be returned. If the slot has already been initialized by another caller then this method will discard the components and return `None`.
        */
        pub fn init<TEmitter, TFilter, TCtxt, TClock, TRng>(
            &self,
            pipeline: Runtime<TEmitter, TFilter, TCtxt, TClock, TRng>,
        ) -> Option<Runtime<&TEmitter, &TFilter, &TCtxt, &TClock, &TRng>>
        where
            TEmitter: Emitter + Send + Sync + 'static,
            TFilter: Filter + Send + Sync + 'static,
            TCtxt: Ctxt + Send + Sync + 'static,
            TCtxt::Frame: Send + 'static,
            TClock: Clock + Send + Sync + 'static,
            TRng: Rng + Send + Sync + 'static,
        {
            self.0
                .set({
                    let value = pipeline
                        .map_emitter(|emitter| {
                            Box::new(emitter) as Box<dyn AnyEmitter + Send + Sync>
                        })
                        .map_filter(|filter| Box::new(filter) as Box<dyn AnyFilter + Send + Sync>)
                        .map_ctxt(|ctxt| Box::new(ctxt) as Box<dyn AnyCtxt + Send + Sync>)
                        .map_clock(|clock| Box::new(clock) as Box<dyn AnyClock + Send + Sync>)
                        .map_rng(|id_gen| Box::new(id_gen) as Box<dyn AnyRng + Send + Sync>);

                    let runtime = Runtime::build(
                        value.emitter().as_super() as *const _,
                        value.filter().as_super() as *const _,
                        value.ctxt().as_super() as *const _,
                        value.clock().as_super() as *const _,
                        value.rng().as_super() as *const _,
                    );

                    AmbientSync { value, runtime }
                })
                .ok()?;

            let rt = self.0.get()?;

            Some(Runtime::build(
                rt.value.emitter().as_any().downcast_ref()?,
                rt.value.filter().as_any().downcast_ref()?,
                rt.value.ctxt().as_any().downcast_ref()?,
                rt.value.clock().as_any().downcast_ref()?,
                rt.value.rng().as_any().downcast_ref()?,
            ))
        }

        /**
        Get the underlying [`Runtime`], or a [`Runtime::default`] if it hasn't been initialized yet.
        */
        pub fn get(&self) -> &AmbientRuntime {
            const EMPTY_AMBIENT_RUNTIME: AmbientRuntime = Runtime::build(
                &Empty as &(dyn ErasedEmitter + Send + Sync + 'static),
                &Empty as &(dyn ErasedFilter + Send + Sync + 'static),
                &Empty as &(dyn ErasedCtxt + Send + Sync + 'static),
                &Empty as &(dyn ErasedClock + Send + Sync + 'static),
                &Empty as &(dyn ErasedRng + Send + Sync + 'static),
            );

            self.0
                .get()
                .map(|rt|
                    // SAFETY: The borrow of `self` cannot outlive the components
                    // it contains. This block is converting `*const dyn T + Send + Sync`
                    // to `&'_ dyn T + Send + Sync`
                    unsafe {
                        &*(&rt.runtime as *const AmbientSyncRuntime as *const AmbientRuntime)
                    })
                .unwrap_or(&EMPTY_AMBIENT_RUNTIME)
        }
    }

    #[cfg(feature = "implicit_internal_rt")]
    impl AmbientInternalSlot {
        pub(in crate::runtime) const fn new() -> Self {
            AmbientInternalSlot(AmbientSlot(OnceLock::new()))
        }

        /**
        Whether the [`internal()`] runtime has been initialized.

        Components can use this method to decide whether to do work related to diagnostic capturing.
        */
        pub fn is_enabled(&self) -> bool {
            self.0.is_enabled()
        }

        /**
        Initialize the [`internal()`] runtime with the given components.

        The components must satisfy additional trait bounds compared to a regular [`AmbientSlot`]. Each component must also implement a marker trait that promises they don't produce any diagnostics of their own.
        */
        pub fn init<TEmitter, TFilter, TCtxt, TClock, TRng>(
            &self,
            pipeline: Runtime<TEmitter, TFilter, TCtxt, TClock, TRng>,
        ) -> Option<Runtime<&TEmitter, &TFilter, &TCtxt, &TClock, &TRng>>
        where
            TEmitter: InternalEmitter + Send + Sync + 'static,
            TFilter: InternalFilter + Send + Sync + 'static,
            TCtxt: InternalCtxt + Send + Sync + 'static,
            TCtxt::Frame: Send + 'static,
            TClock: InternalClock + Send + Sync + 'static,
            TRng: InternalRng + Send + Sync + 'static,
        {
            self.0.init(pipeline)
        }

        /**
        Get the underlying [`Runtime`], or a [`Runtime::default`] if it hasn't been initialized yet.
        */
        pub fn get(&self) -> &AmbientRuntime {
            self.0.get()
        }
    }
}

#[cfg(feature = "std")]
pub use self::std_support::*;

#[cfg(not(feature = "std"))]
mod no_std_support {
    use super::*;

    /**
    A slot for a shared runtime.

    Without the `std` feature enabled, this slot cannot be initialized.
    */
    pub struct AmbientSlot {}

    /**
    A slot for the internal runtime.

    Without the `std` feature enabled, this slot cannot be initialized.
    */
    #[cfg(feature = "implicit_internal_rt")]
    pub struct AmbientInternalSlot(AmbientSlot);

    impl AmbientSlot {
        /**
        Create a new, empty slot.
        */
        pub const fn new() -> Self {
            AmbientSlot {}
        }

        /**
        When the `std` feature is not enabled this method always returns `false`.
        */
        pub fn is_enabled(&self) -> bool {
            false
        }

        /**
        When the `std` feature is not enabled this method always returns an empty runtime.
        */
        pub fn get(&self) -> &AmbientRuntime {
            const EMPTY_AMBIENT_RUNTIME: AmbientRuntime =
                Runtime::build(&Empty, &Empty, &Empty, &Empty, &Empty);

            &EMPTY_AMBIENT_RUNTIME
        }
    }

    #[cfg(feature = "implicit_internal_rt")]
    impl AmbientInternalSlot {
        pub(in crate::runtime) const fn new() -> Self {
            AmbientInternalSlot(AmbientSlot::new())
        }

        /**
        When the `std` feature is not enabled this method always returns `false`.
        */
        pub fn is_enabled(&self) -> bool {
            false
        }

        /**
        When the `std` feature is not enabled this method always returns an empty runtime.
        */
        pub fn get(&self) -> &AmbientRuntime {
            self.0.get()
        }
    }

    /**
    When the `std` feature is not enabled this is always [`Empty`].
    */
    pub type AmbientEmitter<'a> = &'a Empty;
    /**
    When the `std` feature is not enabled this is always [`Empty`].
    */
    pub type AmbientFilter<'a> = &'a Empty;
    /**
    When the `std` feature is not enabled this is always [`Empty`].
    */
    pub type AmbientCtxt<'a> = &'a Empty;
    /**
    When the `std` feature is not enabled this is always [`Empty`].
    */
    pub type AmbientClock<'a> = &'a Empty;
    /**
    When the `std` feature is not enabled this is always [`Empty`].
    */
    pub type AmbientRng<'a> = &'a Empty;

    /**
    When the `std` feature is not enabled this is always [`Runtime::default`].
    */
    pub type AmbientRuntime<'a> = Runtime<
        AmbientEmitter<'a>,
        AmbientFilter<'a>,
        AmbientCtxt<'a>,
        AmbientClock<'a>,
        AmbientRng<'a>,
    >;
}

#[cfg(not(feature = "std"))]
pub use self::no_std_support::*;
