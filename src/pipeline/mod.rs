//! An asynchronous/buffered log event pipeline from producers to a single dispatching consumer.
//! Currently based on `std::sync::mpsc`, but highly likely this will change.

pub mod ambient;
mod async;
pub mod builder;
pub mod chain;
pub mod reference;
