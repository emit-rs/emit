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

    use emit_core::{clock::ErasedClock, rng::ErasedRng, runtime::AssertInternal};

    /**
    The default [`crate::Clock`].
    */
    #[cfg(feature = "std")]
    pub type DefaultClock = system_clock::SystemClock;

    /**
    The default [`crate::Ctxt`] to use in [`crate::setup()`].
    */
    #[cfg(feature = "std")]
    pub type DefaultCtxt = thread_local_ctxt::ThreadLocalCtxt;

    /**
    A type-erased container for system services used when intiailizing runtimes.
    */
    pub(crate) struct Platform {
        #[cfg(feature = "std")]
        pub(crate) clock: AssertInternal<Box<dyn ErasedClock + Send + Sync>>,
        #[cfg(feature = "std")]
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
                #[cfg(feature = "std")]
                clock: AssertInternal(Box::new(DefaultClock::default())),
                #[cfg(feature = "std")]
                rng: AssertInternal(Box::new(DefaultRng::default())),
            }
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
