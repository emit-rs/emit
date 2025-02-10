/*!
Components provided by the underlying platform.

This module defines implementations of [`crate::runtime::Runtime`] components that use capabilities of the host platform.
*/

#[cfg(feature = "std")]
pub mod system_clock;

#[cfg(feature = "std")]
pub mod thread_local_ctxt;

#[cfg(feature = "rand")]
pub mod rand_rng;

/**
The default [`crate::Rng`].
*/
#[cfg(not(feature = "rand"))]
pub type DefaultRng = crate::Empty;
/**
The default [`crate::Rng`].
*/
#[cfg(feature = "rand")]
pub type DefaultRng = rand_rng::RandRng;

#[cfg(feature = "std")]
mod std_support {
    use super::*;

    use emit_core::{
        clock::{Clock, ErasedClock},
        rng::{ErasedRng, Rng},
        runtime::AssertInternal,
    };

    /**
    The default [`crate::Clock`].
    */
    #[cfg(not(all(
        target_arch = "wasm32",
        target_vendor = "unknown",
        target_os = "unknown"
    )))]
    pub type DefaultClock = system_clock::SystemClock;

    /**
    The default [`crate::Clock`].
    */
    #[cfg(all(
        target_arch = "wasm32",
        target_vendor = "unknown",
        target_os = "unknown"
    ))]
    pub type DefaultClock = crate::Empty;

    /**
    The default [`crate::Ctxt`] to use in [`crate::setup()`].
    */
    pub type DefaultCtxt = thread_local_ctxt::ThreadLocalCtxt;

    /**
    A type-erased container for system services used when intiailizing runtimes.
    */
    pub(crate) struct Platform {
        pub(crate) clock: AssertInternal<Box<dyn ErasedClock + Send + Sync>>,
        pub(crate) rng: AssertInternal<Box<dyn ErasedRng + Send + Sync>>,
    }

    impl Default for Platform {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Platform {
        pub fn new() -> Self {
            Platform {
                clock: AssertInternal(Box::new(DefaultClock::default())),
                rng: AssertInternal(Box::new(DefaultRng::default())),
            }
        }

        pub fn with_clock(&mut self, clock: impl Clock + Send + Sync + 'static) {
            self.clock = AssertInternal(Box::new(clock));
        }

        pub fn with_rng(&mut self, rng: impl Rng + Send + Sync + 'static) {
            self.rng = AssertInternal(Box::new(rng));
        }
    }
}

#[cfg(feature = "std")]
pub use self::std_support::*;

#[cfg(not(feature = "std"))]
mod no_std_support {
    /**
    The default [`crate::Clock`].
    */
    #[cfg(not(feature = "std"))]
    pub type DefaultClock = crate::Empty;

    /**
    The default [`crate::Ctxt`]..
    */
    #[cfg(not(feature = "std"))]
    pub type DefaultCtxt = crate::Empty;
}

#[cfg(not(feature = "std"))]
pub use self::no_std_support::*;
