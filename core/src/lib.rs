/*!
A diagnostic framework for Rust applications.

This library is the core API of `emit`, defining the fundamental abstractions used by the higher-level `emit` crate. This library is home to [`event::Event`], `emit`'s model of diagnostic data through with their [`template::Template`], [`props::Props`], and [`extent::Extent`].

In this library is also the all-encapsulating [`runtime::Runtime`], which collects the platform capabilities and event processing pipeline into a single value that powers the diagnostics for your applications.

If you're looking to explore and understand `emit`'s API, you can start with [`runtime::Runtime`] and [`event::Event`] and follow their encapsulated types.

If you're looking to use `emit` in an application you can use this library directly, but `emit` itself is recommended.
*/

#![doc(html_logo_url = "https://raw.githubusercontent.com/emit-rs/emit/main/asset/logo.svg")]
#![deny(missing_docs)]
#![cfg_attr(not(test), no_std)]

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

extern crate core;

pub mod and;
pub mod clock;
pub mod ctxt;
pub mod emitter;
pub mod empty;
pub mod event;
pub mod extent;
pub mod filter;
pub mod or;
pub mod path;
pub mod props;
pub mod rng;
pub mod runtime;
pub mod str;
pub mod template;
pub mod timestamp;
pub mod value;
pub mod well_known;

mod internal {
    pub struct Erased<T>(pub(crate) T);
}
