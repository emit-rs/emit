/*!
Integration tests for `emit`'s macros.

Compile-pass tests mostly live in top-level modules here. Compile-fail tests live under the `compile_fail` module.
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
mod metric;
mod new_span;
mod props;
mod sample;
mod span;
mod tpl;

#[cfg(feature = "std")]
mod format;

mod shadow {
    // Shadow core imports
    #![allow(dead_code)]

    pub struct Result;
    pub struct Ok;
    pub struct Err;
    pub struct Some;
    pub struct None;
    pub struct String;
    pub struct Vec;
    pub mod core {}
    pub mod std {}
}

#[rustversion::nightly]
mod compile {
    #[test]
    #[cfg(feature = "compile")]
    fn compile_fail_std() {
        let t = trybuild::TestCases::new();
        t.compile_fail("src/compile_fail/std/*.rs");
    }

    #[test]
    #[cfg(feature = "compile")]
    fn compile_pass_std() {
        let t = trybuild::TestCases::new();
        t.pass("src/compile_pass/std/*.rs");
    }
}
