#![cfg(test)]

#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate sval_derive;

mod util;

mod emit;
mod event;
mod format;
mod props;
mod span;
mod tpl;
