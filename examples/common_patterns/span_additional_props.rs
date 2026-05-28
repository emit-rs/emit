/*!
This example demonstrates how to attach additional properties to a span on completion.
*/

use std::{collections::HashMap, time::Duration};

// The `guard` control parameter lets us manipulate the span within the body of the function
// The `evt_props` control parameter lets us specify the type for properties on the resulting span event
#[emit::span(guard: span, evt_props: HashMap::new(), "Running an example", i)]
fn example(i: i32) {
    // Your code goes here

    if i > 4 {
        // Add a property to our additional collection
        //
        // `props_mut` returns `None` if the span guard is disabled (filtering rejected it).
        span.props_mut().map(|props| props.insert("is_big", true));
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
