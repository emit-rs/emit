/*!
The [`Setup`] type.

All functionality in `emit` is based on a [`crate::runtime::Runtime`]. When you call [`Setup::init`], it initializes the [`crate::runtime::shared`] runtime for you, which is also what macros use by default.

You can implement your own runtime, providing your own implementations of the ambient clock, randomness, and global context. First, disable the default features of `emit` in your `Cargo.toml`:

```toml
[dependencies.emit]
version = "0.11.0-alpha.16"
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
    ctxt::Ctxt,
    emitter::Emitter,
    empty::Empty,
    filter::Filter,
    runtime::{AmbientRuntime, AmbientSlot, InternalCtxt, InternalEmitter, InternalFilter},
};

use crate::platform::{self, Platform};

/**
Configure `emit` with [`Emitter`]s, [`Filter`]s, and [`Ctxt`].

This function should be called as early in your application as possible. It returns a [`Setup`] builder that, once configured, can be initialized with a call to [`Setup::init`].
*/
pub fn setup() -> Setup {
    Setup::default()
}

pub use platform::{DefaultClock, DefaultCtxt, DefaultRng};

/**
The default [`Emitter`] to use in [`setup()`].
*/
pub type DefaultEmitter = Empty;

/**
The default [`Filter`] to use in [`setup()`].
*/
pub type DefaultFilter = Empty;

/**
A configuration builder for an `emit` runtime.
*/
#[must_use = "call `.init()` to finish setup"]
pub struct Setup<TEmitter = DefaultEmitter, TFilter = DefaultFilter, TCtxt = DefaultCtxt> {
    emitter: TEmitter,
    filter: TFilter,
    ctxt: TCtxt,
    platform: Platform,
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
            platform: Default::default(),
        }
    }
}

impl<TEmitter: Emitter, TFilter: Filter, TCtxt: Ctxt> Setup<TEmitter, TFilter, TCtxt> {
    /**
    Set the [`Emitter`] that will receive diagnostic events.
    */
    pub fn emit_to<UEmitter: Emitter>(self, emitter: UEmitter) -> Setup<UEmitter, TFilter, TCtxt> {
        Setup {
            emitter,
            filter: self.filter,
            ctxt: self.ctxt,
            platform: self.platform,
        }
    }

    /**
    Add an [`Emitter`] that will also receive diagnostic events.
    */
    pub fn and_emit_to<UEmitter: Emitter>(
        self,
        emitter: UEmitter,
    ) -> Setup<And<TEmitter, UEmitter>, TFilter, TCtxt> {
        Setup {
            emitter: self.emitter.and_to(emitter),
            filter: self.filter,
            ctxt: self.ctxt,
            platform: self.platform,
        }
    }

    /**
    Map the current [`Emitter`] into a new value.
    */
    pub fn map_emitter<UEmitter: Emitter>(
        self,
        map: impl FnOnce(TEmitter) -> UEmitter,
    ) -> Setup<UEmitter, TFilter, TCtxt> {
        Setup {
            emitter: map(self.emitter),
            filter: self.filter,
            ctxt: self.ctxt,
            platform: self.platform,
        }
    }

    /**
    Set the [`Filter`] that will be applied before diagnostic events are emitted.
    */
    pub fn emit_when<UFilter: Filter>(self, filter: UFilter) -> Setup<TEmitter, UFilter, TCtxt> {
        Setup {
            emitter: self.emitter,
            filter,
            ctxt: self.ctxt,
            platform: self.platform,
        }
    }

    /**
    Set the [`Ctxt`] that will store ambient properties and attach them to diagnostic events.
    */
    pub fn with_ctxt<UCtxt: Ctxt>(self, ctxt: UCtxt) -> Setup<TEmitter, TFilter, UCtxt> {
        Setup {
            emitter: self.emitter,
            filter: self.filter,
            ctxt,
            platform: self.platform,
        }
    }

    /**
    Map the current [`Ctxt`] into a new value.
    */
    pub fn map_ctxt<UCtxt: Ctxt>(
        self,
        map: impl FnOnce(TCtxt) -> UCtxt,
    ) -> Setup<TEmitter, TFilter, UCtxt> {
        Setup {
            emitter: self.emitter,
            filter: self.filter,
            ctxt: map(self.ctxt),
            platform: self.platform,
        }
    }
}

impl<
        TEmitter: Emitter + Send + Sync + 'static,
        TFilter: Filter + Send + Sync + 'static,
        TCtxt: Ctxt + Send + Sync + 'static,
    > Setup<TEmitter, TFilter, TCtxt>
where
    TCtxt::Frame: Send + 'static,
{
    /**
    Initialize the default runtime used by `emit` macros.

    This method initializes [`crate::runtime::shared`].
    */
    #[must_use = "call `blocking_flush` at the end of `main` to ensure events are flushed."]
    #[cfg(feature = "implicit_rt")]
    pub fn init(self) -> Init<'static, TEmitter, TCtxt> {
        self.init_slot(emit_core::runtime::shared_slot())
    }

    /**
    Initialize a runtime in the given static `slot`.
    */
    #[must_use = "call `blocking_flush` at the end of `main` to ensure events are flushed."]
    pub fn init_slot<'a>(self, slot: &'a AmbientSlot) -> Init<'a, TEmitter, TCtxt> {
        let ambient = slot
            .init(
                emit_core::runtime::Runtime::new()
                    .with_emitter(self.emitter)
                    .with_filter(self.filter)
                    .with_ctxt(self.ctxt)
                    .with_clock(self.platform.clock)
                    .with_rng(self.platform.rng),
            )
            .expect("already initialized");

        Init {
            rt: slot.get(),
            emitter: *ambient.emitter(),
            ctxt: *ambient.ctxt(),
        }
    }
}

impl<
        TEmitter: InternalEmitter + Send + Sync + 'static,
        TFilter: InternalFilter + Send + Sync + 'static,
        TCtxt: InternalCtxt + Send + Sync + 'static,
    > Setup<TEmitter, TFilter, TCtxt>
where
    TCtxt::Frame: Send + 'static,
{
    /**
    Initialize the internal runtime used for diagnosing runtimes themselves.

    This method initializes [`crate::runtime::internal`].
    */
    #[must_use = "call `blocking_flush` at the end of `main` (after flushing the main runtime) to ensure events are flushed."]
    #[cfg(feature = "implicit_internal_rt")]
    pub fn init_internal(self) -> Init<'static, TEmitter, TCtxt> {
        let slot = emit_core::runtime::internal_slot();

        let ambient = slot
            .init(
                emit_core::runtime::Runtime::new()
                    .with_emitter(self.emitter)
                    .with_filter(self.filter)
                    .with_ctxt(self.ctxt)
                    .with_clock(self.platform.clock)
                    .with_rng(self.platform.rng),
            )
            .expect("already initialized");

        Init {
            rt: slot.get(),
            emitter: *ambient.emitter(),
            ctxt: *ambient.ctxt(),
        }
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
    Flush the runtime, ensuring all diagnostic events are fully processed.

    This method forwards to [`Emitter::blocking_flush`], which has details on how the timeout is handled.
    */
    pub fn blocking_flush(&self, timeout: Duration) -> bool {
        self.emitter.blocking_flush(timeout)
    }

    /**
    Get the underlying runtime that was initialized.
    */
    pub fn get(&self) -> &'a AmbientRuntime<'a> {
        self.rt
    }
}
