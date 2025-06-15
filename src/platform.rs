/*!
Components provided by the underlying platform.

This module defines implementations of [`crate::runtime::Runtime`] components that use capabilities of the host platform.

Note that types exported here aren't guartanteed to be available on all platforms. For portability, you should avoid relying on them.
*/

#[cfg(feature = "std")]
#[cfg(not(all(
    target_arch = "wasm32",
    target_vendor = "unknown",
    target_os = "unknown"
)))]
pub mod system_clock;

#[cfg(feature = "std")]
pub mod thread_local_ctxt;

#[cfg(feature = "rand")]
#[cfg(any(
    target_os = "linux",
    target_os = "windows",
    target_os = "macos",
    target_os = "ios",
    target_os = "android",
    target_os = "wasi"
))]
pub mod rand_rng;

#[cfg(feature = "web")]
#[cfg(all(
    target_arch = "wasm32",
    target_vendor = "unknown",
    target_os = "unknown"
))]
pub mod web;

/**
The default [`crate::Emitter`].
*/
pub type DefaultEmitter = crate::Empty;

/**
The default [`crate::Filter`].
*/
pub type DefaultFilter = crate::Empty;

/**
The default [`crate::Ctxt`].
*/
#[cfg(not(feature = "std"))]
pub type DefaultCtxt = crate::Empty;
/**
The default [`crate::Ctxt`] to use in [`crate::setup()`].
*/
#[cfg(feature = "std")]
pub type DefaultCtxt = thread_local_ctxt::ThreadLocalCtxt;

/**
The default [`crate::Clock`].
*/
#[cfg(not(feature = "std"))]
#[cfg(not(all(
    target_arch = "wasm32",
    target_vendor = "unknown",
    target_os = "unknown"
)))]
pub type DefaultClock = crate::Empty;
/**
The default [`crate::Clock`].
*/
#[cfg(feature = "std")]
#[cfg(not(all(
    target_arch = "wasm32",
    target_vendor = "unknown",
    target_os = "unknown"
)))]
pub type DefaultClock = system_clock::SystemClock;

/**
The default [`crate::Clock`].
*/
#[cfg(not(feature = "web"))]
#[cfg(all(
    target_arch = "wasm32",
    target_vendor = "unknown",
    target_os = "unknown"
))]
pub type DefaultClock = crate::Empty;
/**
The default [`crate::Clock`].
*/
#[cfg(feature = "web")]
#[cfg(all(
    target_arch = "wasm32",
    target_vendor = "unknown",
    target_os = "unknown"
))]
pub type DefaultClock = web::date_clock::DateClock;

/**
The default [`crate::Rng`].
*/
#[cfg(not(feature = "rand"))]
#[cfg(any(
    target_os = "linux",
    target_os = "windows",
    target_os = "macos",
    target_os = "ios",
    target_os = "android",
    target_os = "wasi"
))]
pub type DefaultRng = crate::Empty;
/**
The default [`crate::Rng`].
*/
#[cfg(feature = "rand")]
#[cfg(any(
    target_os = "linux",
    target_os = "windows",
    target_os = "macos",
    target_os = "ios",
    target_os = "android",
    target_os = "wasi"
))]
pub type DefaultRng = rand_rng::RandRng;

/**
The default [`crate::Rng`].
*/
#[cfg(not(feature = "web"))]
#[cfg(all(
    target_arch = "wasm32",
    target_vendor = "unknown",
    target_os = "unknown"
))]
pub type DefaultRng = crate::Empty;
/**
The default [`crate::Rng`].
*/
#[cfg(feature = "web")]
#[cfg(all(
    target_arch = "wasm32",
    target_vendor = "unknown",
    target_os = "unknown"
))]
pub type DefaultRng = web::crypto_rng::CryptoRng;
