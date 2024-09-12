/*!
Structured diagnostics for Rust applications.

`emit` is a structured logging framework for manually instrumenting Rust applications with an expressive syntax inspired by [Message Templates](https://messagetemplates.org).

These are the technical docs for `emit`. See [the guide](https://emitrs.io) for a complete introduction to `emit`.

## Getting started

```toml
[dependencies.emit]
version = "0.11.0-alpha.16"

[dependencies.emit_term]
version = "0.11.0-alpha.16"
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
    emit::emit!("Hello, {user}!");
}
```

The [`setup()`] function configures `emit` with an [`Emitter`] to write [`Event`]s to. The [`macro@emit`] macro emits an event, capturing any ambient state referred to in its template. The [`macro@span`] macro instruments a function, timing its execution and correlating any other events emitted within it together.

## Where can I send my diagnostics?

Emitters are defined in external libraries and plugged in to your diagnostic pipeline during [`emit::setup`]. Some emitters include:

- [`emit_term`](https://docs.rs/emit_term/0.11.0-alpha.16/emit_term/index.html) for writing human-readable output to the console.
- [`emit_file`](https://docs.rs/emit_file/0.11.0-alpha.16/emit_file/index.html) for writing JSON or another machine-readable format to rolling files.
- [`emit_otlp`](https://docs.rs/emit_otlp/0.11.0-alpha.16/emit_otlp/index.html) for sending diagnostics to an OpenTelemetry compatible collector.
- [`emit_opentelemetry`](https://docs.rs/emit_opentelemetry/0.11.0-alpha.16/emit_opentelemetry/index.html) for integrating `emit` into an application using the OpenTelemetry SDK for its diagnostics.

## Troubleshooting

Emitters write their own diagnostics to an alternative `emit` runtime, which you can configure to debug them:

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

This defers uses the standard `module_path` macro.
*/
#[macro_export]
macro_rules! mdl {
    () => {
        $crate::Path::new_unchecked($crate::__private::core::module_path!())
    };
}

/**
Get a [`Path`] of the package name for use in [`Event::mdl`].

This macro uses the build-time `CARGO_PKG_NAME` environment variable.
*/
#[macro_export]
macro_rules! pkg {
    () => {
        $crate::Path::new_unchecked($crate::__private::core::env!("CARGO_PKG_NAME"))
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

#[doc(hidden)]
pub mod __private {
    pub extern crate core;
    pub use crate::macro_hooks::*;
}

mod internal {
    pub struct Erased<T>(pub(crate) T);
}
