/*!
This example demonstrates how to include `anyhow::Error` on a fallible span.
*/

use std::time::Duration;

// The `err` control parameter can be used on functions returning a `Result`
// to change the level based on the result returned. `err` is a function
// that maps the error into a `std::error::Error`.
//
// `err` can be combined with `ok_lvl` and `err_lvl`. Also see the
// `span_fallible_completion` example.
#[emit::span(
    err: emit::err::as_ref,
    "Running an example",
    i,
)]
fn example(i: i32) -> Result<(), anyhow::Error> {
    let r = i + 1;

    if r == 4 {
        Err(anyhow::Error::msg(format!("{r} is 4")))
    } else {
        Ok(())
    }
}

fn main() {
    let rt = emit::setup().emit_to(emit_term::stdout()).init();

    let _ = example(1);
    let _ = example(3);

    rt.blocking_flush(Duration::from_secs(5));
}
