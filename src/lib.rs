/*!
Structured diagnostics for Rust applications.

`emit` is a framework for adding diagnostics to your Rust applications with a simple, powerful data model and an expressive syntax inspired by [Message Templates](https://messagetemplates.org).

These are the technical API docs for `emit`. Also see [the guide](https://emit-rs.io) for a complete introduction.

## Getting started

```toml
[dependencies.emit]
version = "1.8.0"

[dependencies.emit_term]
version = "1.8.0"
```

```rust
# mod emit_term { pub fn stdout() -> impl emit::Emitter { emit::emitter::from_fn(|_| {}) } }
# #[cfg(not(feature = "std"))] fn main() {}
# #[cfg(feature = "std")]
fn main() {
    let rt = emit::setup()
        .emit_to(emit_term::stdout())
        .init();

    // Your app code goes here
    greet("Rust");

    rt.blocking_flush(std::time::Duration::from_secs(5));
}

#[emit::span("Greet {user}")]
fn greet(user: &str) {
    emit::info!("Hello, {user}!");
}
```

The [`setup()`] function configures `emit` with an [`Emitter`] to write [`Event`]s to. The [`macro@emit`] macro emits an event, capturing any ambient state referred to in its template. The [`macro@span`] macro instruments a function, timing its execution and correlating any other events emitted within it together.

## Stable vs nightly toolchains

`emit` works on stable versions of Rust, but can provide more accurate compiler messages on nightly toolchains.

## Crate features

- `std` (default): Enable support for the standard library. Enable capturing properties as errors. Implies `alloc`.
- `alloc`: Enable APIs that require an allocator.
- `implicit_rt` (default): Enable configuring the default shared runtime and calling [`macro@emit`] and [`macro@span`] without needing to specify a runtime manually.
- `implicit_internal_rt` (default): Enable configuring the internal runtime for `emit`'s own diagnostics.
- `sval`: Enable capturing complex properties using `sval`.
- `serde`: Enable capturing complex properties using `serde`.

## Troubleshooting

Emitters write their own diagnostics to an alternative `emit` runtime, which you can configure via [`Setup::init_internal`] to debug them:

```
# mod emit_term { pub fn stdout() -> impl emit::runtime::InternalEmitter { emit::runtime::AssertInternal(emit::emitter::from_fn(|_| {})) } }
# #[cfg(not(feature = "std"))] fn main() {}
# #[cfg(feature = "std")]
fn main() {
    // Configure the internal runtime before your regular setup
    let internal_rt = emit::setup()
        .emit_to(emit_term::stdout())
        .init_internal();

    let rt = emit::setup()
        .emit_to(emit::emitter::from_fn(|evt| println!("{evt:#?}")))
        .init();

    // Your app code goes here

    rt.blocking_flush(std::time::Duration::from_secs(5));

    // Flush the internal runtime after your regular setup
    internal_rt.blocking_flush(std::time::Duration::from_secs(5));
}
```
*/

#![doc(html_logo_url = "https://raw.githubusercontent.com/emit-rs/emit/main/asset/logo.svg")]
#![deny(missing_docs)]
#![cfg_attr(not(any(test, feature = "std")), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;
extern crate core;

/**
Get a [`Path`] of the executing module for use in [`Event::mdl`].

This macro uses the standard `module_path!` macro.
*/
#[macro_export]
macro_rules! mdl {
    () => {
        $crate::Path::new_raw($crate::__private::core::module_path!())
    };
}

/**
Get a [`Path`] of the package name for use in [`Event::mdl`].

This macro uses the build-time `CARGO_PKG_NAME` environment variable.
*/
#[macro_export]
macro_rules! pkg {
    () => {
        $crate::Path::new_raw($crate::__private::core::env!("CARGO_PKG_NAME"))
    };
}

#[doc(inline)]
pub use emit_macros::*;

#[doc(inline)]
pub use emit_core::*;

pub mod frame;
pub mod kind;
pub mod level;
pub mod metric;
pub mod platform;
pub mod span;
pub mod timer;

#[cfg(feature = "std")]
pub mod err;

pub use self::{
    clock::Clock,
    ctxt::Ctxt,
    emitter::Emitter,
    empty::Empty,
    event::Event,
    extent::Extent,
    filter::Filter,
    frame::Frame,
    kind::Kind,
    level::Level,
    metric::Metric,
    path::Path,
    props::Props,
    rng::Rng,
    span::{Span, SpanCtxt, SpanId, TraceId},
    str::Str,
    template::Template,
    timer::Timer,
    timestamp::Timestamp,
    value::Value,
};

mod macro_hooks;

#[cfg(feature = "std")]
pub mod setup;
#[cfg(feature = "std")]
pub use setup::{setup, Setup};

/**
Get the shared emitter.

This method will use the [`Emitter`] from [`runtime::shared()`].

Calling `emitter::emit()` is different from `runtime::shared().emit()`:

1. `emitter::emit()` won't apply the filter.
2. `emitter::emit()` won't add a timestamp to events if they don't have one.
3. `emitter::emit()` won't add ambient properties.
*/
#[cfg(feature = "implicit_rt")]
pub fn emitter() -> runtime::AmbientEmitter<'static> {
    *runtime::shared().emitter()
}

/**
Get the shared filter.

This method will use the [`Filter`] from [`runtime::shared()`].
*/
#[cfg(feature = "implicit_rt")]
pub fn filter() -> runtime::AmbientFilter<'static> {
    *runtime::shared().filter()
}

/**
Get the shared clock.

This method will use the [`Clock`] from [`runtime::shared()`].
*/
#[cfg(feature = "implicit_rt")]
pub fn clock() -> runtime::AmbientClock<'static> {
    *runtime::shared().clock()
}

/**
Get the shared context.

This method will use the [`Ctxt`] from [`runtime::shared()`].

The returned context can be used with [`Frame`]s to manage the ambient state added to diagnostic events.
*/
#[cfg(feature = "implicit_rt")]
pub fn ctxt() -> runtime::AmbientCtxt<'static> {
    *runtime::shared().ctxt()
}

/**
Get the shared random generator.

This method will use the [`Rng`] from [`runtime::shared()`].
*/
#[cfg(feature = "implicit_rt")]
pub fn rng() -> runtime::AmbientRng<'static> {
    *runtime::shared().rng()
}

/**
Flush the runtime, ensuring all diagnostic events are fully processed.

This method will use [`runtime::shared()`].

This method forwards to [`Emitter::blocking_flush`], which has details on how the timeout is handled.
*/
#[cfg(feature = "implicit_rt")]
pub fn blocking_flush(timeout: core::time::Duration) -> bool {
    runtime::shared().blocking_flush(timeout)
}

#[doc(hidden)]
pub mod __private {
    pub extern crate core;
    pub use crate::macro_hooks::*;
}

mod internal {
    pub struct Erased<T>(pub(crate) T);
}
