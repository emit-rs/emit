/*!
This example demonstrates how to customize the span kind in the `#[span]` macro.
*/

use std::time::Duration;

// The `kind` control parameter sets the span kind to "server"
#[emit::span(kind: "server", "Running an example", i)]
fn example(i: i32) {
    let _ = i + 1;
}

fn main() {
    let rt = emit::setup().emit_to(emit_term::stdout()).init();

    example(1);

    rt.blocking_flush(Duration::from_secs(5));
}
