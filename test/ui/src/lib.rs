/*!
Integration tests for `emit`'s macros.

Compile-pass tests live in top-level modules here. Compile-fail tests live under the `compile_fail` module.
*/

#![cfg(test)]

#[macro_use]
#[cfg(feature = "serde")]
extern crate serde_derive;

#[macro_use]
#[cfg(feature = "sval")]
extern crate sval_derive;

mod util;

mod emit;
mod event;
mod props;
mod span;
mod tpl;

#[cfg(feature = "std")]
mod format;

#[test]
#[cfg(feature = "compile_fail")]
fn compile_fail_std() {
    let t = trybuild::TestCases::new();
    t.compile_fail("src/compile_fail/std/*.rs");
}
