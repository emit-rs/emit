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
#[cfg(all(feature = "implicit_rt", feature = "std"))]
fn compile_fail_std() {
    let t = trybuild::TestCases::new();
    t.compile_fail("src/compile_fail/std/*.rs");
}
