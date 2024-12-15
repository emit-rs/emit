/*!
This examples demonstrates how to emit an event with an error from `anyhow`.
*/

use std::time::Duration;

fn example() {
    let err = anyhow::Error::msg("Some failure");

    // You can use the `emit::err::as_ref` function to convert an `anyhow::Error`
    // into a `std::error::Error` so it can be captured by `emit`
    emit::warn!("Failed to perform some task due to {err: emit::err::as_ref(&err)}");
}

fn main() {
    let rt = emit::setup().emit_to(emit_term::stdout()).init();

    example();

    rt.blocking_flush(Duration::from_secs(5));
}
