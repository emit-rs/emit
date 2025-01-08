/*!
This example demonstrates how to use the `#[span]` macro to trace a function's execution.
*/

use std::time::Duration;

#[emit::span("Running an example", i)]
fn example(i: i32) {
    let _ = i + 1;
}

fn main() {
    let rt = emit::setup().emit_to(emit_term::stdout()).init();

    example(1);
    example(3);

    rt.blocking_flush(Duration::from_secs(5));
}
