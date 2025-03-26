/*!
The [`Setup`] type.

All functionality in `emit` is based on a [`crate::runtime::Runtime`]. When you call [`Setup::init`], it initializes the [`crate::runtime::shared`] runtime for you, which is also what macros use by default.

You can implement your own runtime, providing your own implementations of the ambient clock, randomness, and global context. First, disable the default features of `emit` in your `Cargo.toml`:

```toml
[dependencies.emit]
version = "1.4.1"
default-features = false
features = ["std"]
```

This will ensure the `rt` control parameter is always passed to macros so that your custom runtime will always be used.

You can define your runtime as a [`crate::runtime::AmbientSlot`] in a static and initialize it through [`Setup::init_slot`]:

```
// Define a static runtime to use
// In this example, we use the default implementations of most things,
// but you can also bring-your-own
static RUNTIME: emit::runtime::AmbientSlot = emit::runtime::AmbientSlot::new();

let rt = emit::setup()
    .emit_to(emit::emitter::from_fn(|evt| println!("{}", evt.msg())))
    .init_slot(&RUNTIME);

// Use your runtime with the `rt` control parameter
emit::emit!(rt: RUNTIME.get(), "emitted through a custom runtime");

rt.blocking_flush(std::time::Duration::from_secs(5));
```

```text
emitted through a custom runtime
```

The [`crate::runtime::AmbientSlot`] is type-erased, but you can also define your own fully concrete runtimes too:

```
// Define a static runtime to use
// In this example, we use the default implementations of most things,
// but you can also bring-your-own
static RUNTIME: emit::runtime::Runtime<
    MyEmitter,
    emit::Empty,
    emit::platform::thread_local_ctxt::ThreadLocalCtxt,
    emit::platform::system_clock::SystemClock,
    emit::platform::rand_rng::RandRng,
> = emit::runtime::Runtime::build(
    MyEmitter,
    emit::Empty,
    emit::platform::thread_local_ctxt::ThreadLocalCtxt::shared(),
    emit::platform::system_clock::SystemClock::new(),
    emit::platform::rand_rng::RandRng::new(),
);

struct MyEmitter;

impl emit::Emitter for MyEmitter {
    fn emit<E: emit::event::ToEvent>(&self, evt: E) {
        println!("{}", evt.to_event().msg());
    }

    fn blocking_flush(&self, _: std::time::Duration) -> bool {
        // Nothing to flush
        true
    }
}

// Use your runtime with the `rt` control parameter
emit::emit!(rt: &RUNTIME, "emitted through a custom runtime");
```

```text
emitted through a custom runtime
```
*/

use core::time::Duration;

use emit_core::{
    and::And,
    clock::Clock,
    ctxt::Ctxt,
    emitter::Emitter,
    filter::Filter,
    rng::Rng,
    runtime::{
        AmbientRuntime, AmbientSlot, InternalClock, InternalCtxt, InternalEmitter, InternalFilter,
        InternalRng,
    },
};

/**
Configure `emit` with [`Emitter`]s, [`Filter`]s, and [`Ctxt`].

This function should be called as early in your application as possible. It returns a [`Setup`] builder that, once configured, can be initialized with a call to [`Setup::init`].
*/
pub fn setup() -> Setup {
    Setup::default()
}

pub use crate::platform::{DefaultClock, DefaultCtxt, DefaultEmitter, DefaultFilter, DefaultRng};

/**
A configuration builder for an `emit` runtime.
*/
#[must_use = "call `.init()` to finish setup"]
pub struct Setup<
    TEmitter = DefaultEmitter,
    TFilter = DefaultFilter,
    TCtxt = DefaultCtxt,
    TClock = DefaultClock,
    TRng = DefaultRng,
> {
    emitter: TEmitter,
    filter: TFilter,
    ctxt: TCtxt,
    clock: TClock,
    rng: TRng,
}

impl Default for Setup {
    fn default() -> Self {
        Self::new()
    }
}

impl Setup {
    /**
    Create a new builder with the default [`Emitter`], [`Filter`], and [`Ctxt`].
    */
    pub fn new() -> Self {
        Setup {
            emitter: Default::default(),
            filter: Default::default(),
            ctxt: Default::default(),
            clock: Default::default(),
            rng: Default::default(),
        }
    }
}

impl<TEmitter: Emitter, TFilter: Filter, TCtxt: Ctxt, TClock: Clock, TRng: Rng>
    Setup<TEmitter, TFilter, TCtxt, TClock, TRng>
{
    /**
    Set the [`Emitter`] that will receive diagnostic events.
    */
    pub fn emit_to<UEmitter: Emitter>(
        self,
        emitter: UEmitter,
    ) -> Setup<UEmitter, TFilter, TCtxt, TClock, TRng> {
        Setup {
            emitter,
            filter: self.filter,
            ctxt: self.ctxt,
            clock: self.clock,
            rng: self.rng,
        }
    }

    /**
    Add an [`Emitter`] that will also receive diagnostic events.
    */
    pub fn and_emit_to<UEmitter: Emitter>(
        self,
        emitter: UEmitter,
    ) -> Setup<And<TEmitter, UEmitter>, TFilter, TCtxt, TClock, TRng> {
        Setup {
            emitter: self.emitter.and_to(emitter),
            filter: self.filter,
            ctxt: self.ctxt,
            clock: self.clock,
            rng: self.rng,
        }
    }

    /**
    Map the current [`Emitter`] into a new value.
    */
    pub fn map_emitter<UEmitter: Emitter>(
        self,
        map: impl FnOnce(TEmitter) -> UEmitter,
    ) -> Setup<UEmitter, TFilter, TCtxt, TClock, TRng> {
        Setup {
            emitter: map(self.emitter),
            filter: self.filter,
            ctxt: self.ctxt,
            clock: self.clock,
            rng: self.rng,
        }
    }

    /**
    Set the [`Filter`] that will be applied before diagnostic events are emitted.
    */
    pub fn emit_when<UFilter: Filter>(
        self,
        filter: UFilter,
    ) -> Setup<TEmitter, UFilter, TCtxt, TClock, TRng> {
        Setup {
            emitter: self.emitter,
            filter,
            ctxt: self.ctxt,
            clock: self.clock,
            rng: self.rng,
        }
    }

    /**
    Add a [`Filter`] that will also be applied before diagnostic events are emitted.
    */
    pub fn and_emit_when<UFilter: Filter>(
        self,
        filter: UFilter,
    ) -> Setup<TEmitter, And<TFilter, UFilter>, TCtxt, TClock, TRng> {
        Setup {
            emitter: self.emitter,
            filter: self.filter.and_when(filter),
            ctxt: self.ctxt,
            clock: self.clock,
            rng: self.rng,
        }
    }

    /**
    Set the [`Ctxt`] that will store ambient properties and attach them to diagnostic events.
    */
    pub fn with_ctxt<UCtxt: Ctxt>(
        self,
        ctxt: UCtxt,
    ) -> Setup<TEmitter, TFilter, UCtxt, TClock, TRng> {
        Setup {
            emitter: self.emitter,
            filter: self.filter,
            ctxt,
            clock: self.clock,
            rng: self.rng,
        }
    }

    /**
    Map the current [`Ctxt`] into a new value.
    */
    pub fn map_ctxt<UCtxt: Ctxt>(
        self,
        map: impl FnOnce(TCtxt) -> UCtxt,
    ) -> Setup<TEmitter, TFilter, UCtxt, TClock, TRng> {
        Setup {
            emitter: self.emitter,
            filter: self.filter,
            ctxt: map(self.ctxt),
            clock: self.clock,
            rng: self.rng,
        }
    }

    /**
    Set the [`Clock`] used to assign timestamps and run timers.
    */
    pub fn with_clock<UClock: Clock>(
        self,
        clock: UClock,
    ) -> Setup<TEmitter, TFilter, TCtxt, UClock, TRng> {
        Setup {
            emitter: self.emitter,
            filter: self.filter,
            ctxt: self.ctxt,
            clock,
            rng: self.rng,
        }
    }

    /**
    Set the [`Rng`] used to assign trace and span ids.
    */
    pub fn with_rng<URng: Rng>(self, rng: URng) -> Setup<TEmitter, TFilter, TCtxt, TClock, URng> {
        Setup {
            emitter: self.emitter,
            filter: self.filter,
            ctxt: self.ctxt,
            clock: self.clock,
            rng,
        }
    }
}

impl<
        TEmitter: Emitter + Send + Sync + 'static,
        TFilter: Filter + Send + Sync + 'static,
        TCtxt: Ctxt + Send + Sync + 'static,
        TClock: Clock + Send + Sync + 'static,
        TRng: Rng + Send + Sync + 'static,
    > Setup<TEmitter, TFilter, TCtxt, TClock, TRng>
where
    TCtxt::Frame: Send + 'static,
{
    /**
    Initialize the default runtime used by `emit` macros.

    This method initializes [`crate::runtime::shared`].

    # Panics

    This method will panic if the slot has already been initialized.
    */
    #[must_use = "call `flush_on_drop` or call `blocking_flush` at the end of `main` to ensure events are flushed."]
    #[cfg(feature = "implicit_rt")]
    pub fn init(self) -> Init<'static, TEmitter, TCtxt> {
        self.init_slot(emit_core::runtime::shared_slot())
    }

    /**
    Try initialize the default runtime used by `emit` macros.

    This method initializes [`crate::runtime::shared`].

    If the slot is already initialized, this method will return `None`.
    */
    #[must_use = "call `flush_on_drop` or call `blocking_flush` at the end of `main` to ensure events are flushed."]
    #[cfg(feature = "implicit_rt")]
    pub fn try_init(self) -> Option<Init<'static, TEmitter, TCtxt>> {
        self.try_init_slot(emit_core::runtime::shared_slot())
    }

    /**
    Initialize a runtime in the given static `slot`.

    # Panics

    This method will panic if the slot has already been initialized.
    */
    #[must_use = "call `flush_on_drop` or call `blocking_flush` at the end of `main` to ensure events are flushed."]
    pub fn init_slot<'a>(self, slot: &'a AmbientSlot) -> Init<'a, TEmitter, TCtxt> {
        self.try_init_slot(slot).expect("already initialized")
    }

    /**
    Try initialize a runtime in the given static `slot`.

    If the slot is already initialized, this method will return `None`.
    */
    #[must_use = "call `flush_on_drop` or call `blocking_flush` at the end of `main` to ensure events are flushed."]
    pub fn try_init_slot<'a>(self, slot: &'a AmbientSlot) -> Option<Init<'a, TEmitter, TCtxt>> {
        let ambient = slot.init(
            emit_core::runtime::Runtime::new()
                .with_emitter(self.emitter)
                .with_filter(self.filter)
                .with_ctxt(self.ctxt)
                .with_clock(self.clock)
                .with_rng(self.rng),
        )?;

        Some(Init {
            rt: slot.get(),
            emitter: *ambient.emitter(),
            ctxt: *ambient.ctxt(),
        })
    }
}

impl<
        TEmitter: InternalEmitter + Send + Sync + 'static,
        TFilter: InternalFilter + Send + Sync + 'static,
        TCtxt: InternalCtxt + Send + Sync + 'static,
        TClock: InternalClock + Send + Sync + 'static,
        TRng: InternalRng + Send + Sync + 'static,
    > Setup<TEmitter, TFilter, TCtxt, TClock, TRng>
where
    TCtxt::Frame: Send + 'static,
{
    /**
    Initialize the internal runtime used for diagnosing runtimes themselves.

    This method initializes [`crate::runtime::internal`].

    # Panics

    This method will panic if the slot has already been initialized.
    */
    #[must_use = "call `flush_on_drop` or call `blocking_flush` at the end of `main` (after flushing the main runtime) to ensure events are flushed."]
    #[cfg(feature = "implicit_internal_rt")]
    pub fn init_internal(self) -> Init<'static, TEmitter, TCtxt> {
        self.try_init_internal().expect("already initialized")
    }

    /**
    Initialize the internal runtime used for diagnosing runtimes themselves.

    This method initializes [`crate::runtime::internal`].

    If the slot is already initialized, this method will return `None`.
    */
    #[must_use = "call `flush_on_drop` or call `blocking_flush` at the end of `main` (after flushing the main runtime) to ensure events are flushed."]
    #[cfg(feature = "implicit_internal_rt")]
    pub fn try_init_internal(self) -> Option<Init<'static, TEmitter, TCtxt>> {
        let slot = emit_core::runtime::internal_slot();

        let ambient = slot.init(
            emit_core::runtime::Runtime::new()
                .with_emitter(self.emitter)
                .with_filter(self.filter)
                .with_ctxt(self.ctxt)
                .with_clock(self.clock)
                .with_rng(self.rng),
        )?;

        Some(Init {
            rt: slot.get(),
            emitter: *ambient.emitter(),
            ctxt: *ambient.ctxt(),
        })
    }
}

/**
The result of calling [`Setup::init`].

This type is a handle to an initialized runtime that can be used to ensure it's fully flushed with a call to [`Init::blocking_flush`] before your application exits.
*/
pub struct Init<'a, TEmitter: Emitter + ?Sized = DefaultEmitter, TCtxt: Ctxt + ?Sized = DefaultCtxt>
{
    rt: &'a AmbientRuntime<'a>,
    emitter: &'a TEmitter,
    ctxt: &'a TCtxt,
}

impl<'a, TEmitter: Emitter + ?Sized, TCtxt: Ctxt + ?Sized> Init<'a, TEmitter, TCtxt> {
    /**
    Get a reference to the initialized [`Emitter`].
    */
    pub fn emitter(&self) -> &'a TEmitter {
        self.emitter
    }

    /**
    Get a reference to the initialized [`Ctxt`].
    */
    pub fn ctxt(&self) -> &'a TCtxt {
        self.ctxt
    }

    /**
    Get the underlying runtime that was initialized.
    */
    pub fn get(&self) -> &'a AmbientRuntime<'a> {
        self.rt
    }

    /**
    Flush the runtime, ensuring all diagnostic events are fully processed.

    This method forwards to [`Emitter::blocking_flush`], which has details on how the timeout is handled.
    */
    pub fn blocking_flush(&self, timeout: Duration) -> bool {
        self.emitter.blocking_flush(timeout)
    }

    /**
    Flush the runtime when the returned guard is dropped, ensuring all diagnostic events are fully processed.

    This method forwards to [`Emitter::blocking_flush`], which has details on how the timeout is handled.

    **Important:** Ensure you bind an identifier to this method, otherwise it will be immediately dropped instead of at the end of your `main`:

    ```
    # use std::time::Duration;
    fn main() {
        // Use an ident like `_guard`, not `_`
        let _guard = emit::setup().init().flush_on_drop(Duration::from_secs(5));

        // Your code goes here
    }
    ```
    */
    pub fn flush_on_drop(self, timeout: Duration) -> InitGuard<'a, TEmitter, TCtxt> {
        InitGuard {
            inner: self,
            timeout,
        }
    }
}

/**
The result of calling [`Init::flush_on_drop`].

This type is a guard that will call [`Init::blocking_flush`] when it goes out of scope. It helps ensure diagnostics are emitted, even if a panic unwinds through your `main` function.
*/
pub struct InitGuard<
    'a,
    TEmitter: Emitter + ?Sized = DefaultEmitter,
    TCtxt: Ctxt + ?Sized = DefaultCtxt,
> {
    inner: Init<'a, TEmitter, TCtxt>,
    timeout: Duration,
}

impl<'a, TEmitter: Emitter + ?Sized, TCtxt: Ctxt + ?Sized> InitGuard<'a, TEmitter, TCtxt> {
    /**
    Get the inner [`Init`] value, which can then be used to get the underlying [`AmbientRuntime`].
    */
    pub fn inner(&self) -> &Init<'a, TEmitter, TCtxt> {
        &self.inner
    }
}

impl<'a, TEmitter: Emitter + ?Sized, TCtxt: Ctxt + ?Sized> Drop for InitGuard<'a, TEmitter, TCtxt> {
    fn drop(&mut self) {
        self.inner.blocking_flush(self.timeout);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_init() {
        let slot = AmbientSlot::new();

        assert!(setup().try_init_slot(&slot).is_some());
        assert!(setup().try_init_slot(&slot).is_none());
    }
}
