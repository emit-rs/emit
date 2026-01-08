/*!
This example demonstrates how to attach additional properties to a span on completion.
*/

use std::{collections::HashMap, time::Duration};

use emit::Props as _;

// The `guard` control parameter lets us manipulate the span within the body of the function
#[emit::span(guard: span, "Running an example", i)]
fn example(i: i32) {
    // This example uses a `HashMap` to store additional properties to include
    let additional_props = HashMap::new();
    let mut span = span.map_props(|props| additional_props.and_props(props));

    // Your code goes here

    if i > 4 {
        span.props_mut()
            .map(|props| props.left_mut().insert("is_big", true));
    }
}

fn main() {
    let rt = emit::setup()
        .emit_to(emit::emitter::from_fn(|evt| println!("{evt:?}")))
        .init();

    example(1);
    example(5);

    rt.blocking_flush(Duration::from_secs(5));
}
