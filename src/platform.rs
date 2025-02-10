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
    The default [`crate::Ctxt`].
    */
    #[cfg(not(feature = "std"))]
    pub type DefaultCtxt = crate::Empty;
}

#[cfg(not(feature = "std"))]
pub use self::no_std_support::*;
