# About

> **NOTE:** This guide is an active work in progress. Most of the content is currently stubbed out, but being filled in.

This guide introduces `emit`. You may also want to look at:

- [the source on GitHub](https://github.com/emit-rs/emit).
- [a set of task-oriented examples](https://github.com/emit-rs/emit/tree/main/examples).
- [the API docs](https://docs.rs/emit/0.11.0-alpha.17/emit/index.html).

## What is `emit`?

`emit` is a framework for adding diagnostics to your Rust applications with a simple, powerful data model and an expressive syntax inspired by [Message Templates](https://messagetemplates.org).

Diagnostics in `emit` are represented as _events_ which combine:

- _extent:_ The point in time when the event occurred, or the span of time for which it was active.
- _template:_ A user-facing description of the event that supports property interpolation.
- _properties:_ A bag of structured key-value pairs that capture the context surrounding the event. Properties may be simple primitives like numbers or strings, or arbitrarily complex structures like maps, sequences, or enums.

Using `emit`'s events you can:

- log structured events.
- trace function execution and participate in distributed tracing.
- surface live metrics.
- build anything you can represent as a time-oriented bag of data.

## Who is `emit` for?

`emit` is for Rust applications, it's not intended to be used in public libraries. In general, libraries shouldn't use a diagnostics framework anyway, but `emit`'s opinionated data model, use of dependencies, and procedural macros, will likely make it unappealing for Rust libraries.

## Design goals

`emit`'s guiding design principle is low ceremony, low cognitive-load. Diagnostics are our primary focus, but they're probably not yours. Configuration should be straightforward, operation should be transparent, and visual noise in instrumented code should be low.

These goals result in some tradeoffs that may affect `emit`'s suitability for your needs:

- Simplicity over performance. Keeping the impact of diagnostics small is still important, but not at the expense of usability or simplicity.
- Not an SDK. `emit` has a hackable API you can tailor to your needs but is also a small, complete, and cohesive set of components for you to use out-of-the-box.

## Stable vs nightly toolchains

`emit` works on stable versions of Rust, but can provide more accurate compiler messages on nightly toolchains. If you're using `rustup`, you can [easily install a nightly build of Rust](https://rust-lang.github.io/rustup/concepts/channels.html#working-with-nightly-rust) from it.

## Stability

`emit` follows the regular semver policy of other Rust libraries with the following additional considerations:

- Changes to the interpretation of events, such as the use of new extensions, are considered breaking.
- Breaking changes to `emit_core` are not planned.
- Breaking changes to `emit` itself, its macros, and emitters, may happen infrequently. Major changes to its API are not planned though. We're aware that, as a diagnostics library, you're likely to spread a lot of `emit` code through your application, so even small changes can have a big impact.

As an application developer, you should be able to rely on the stability of `emit` to not to get in the way of your everyday programming.
