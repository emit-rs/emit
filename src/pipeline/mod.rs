//! An asynchronous/buffered log event AsyncCollectorfrom producers to a single dispatching consumer.
//! Currently based on channels, but highly likely this will change.

pub mod ambient;
mod async;
pub mod builder;
pub mod chain;
pub mod reference;
