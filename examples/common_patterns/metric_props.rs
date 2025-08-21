/*!
This example demonstrates how to attach properties to metric samples.
*/

use std::time::Duration;

fn main() {
    let rt = emit::setup().emit_to(emit_term::stdout()).init();

    let metric_a = 42;

    emit::count_sample!(
        value: metric_a,
        // The `props` control parameter accepts any `impl emit::Props`
        props: emit::props! {
            // Additional properties go here
            my_prop: "some value",
        },
    );

    rt.blocking_flush(Duration::from_secs(5));
}
