/*!
This example demonstrates how to customize the span name in the `#[span]` macro.
*/

use std::time::Duration;

// The `name` control parameter sets the span name to "Custom Span Name"
// If unspecified, the template literal ("Running an example" in this case)
#[emit::span(name: "Custom Span Name", "Running an example", i)]
fn example(i: i32) {
    let _ = i + 1;
}

fn main() {
    let rt = emit::setup().emit_to(emit_term::stdout()).init();

    example(1);

    rt.blocking_flush(Duration::from_secs(5));
}
