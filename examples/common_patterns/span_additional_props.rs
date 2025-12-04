/*!
This example demonstrates how to attach additional properties to a span on completion.
*/

use std::{cell::RefCell, collections::HashMap, rc::Rc, time::Duration};

#[emit::span(guard: span, "Running an example", i)]
fn example(i: i32) {
    // This example uses a `HashMap` to store additional properties to include.
    //
    // To make sure additional properties are always included regardless of early returns,
    // we use a local clonable wrapper for them. If you don't need to worry about early
    // returns you can use a raw `HashMap` or other structure instead.
    let additional_props = Rc::new(RefCell::new(HashMap::new()));

    let _span = span.with_completion(emit::span::completion::from_fn({
        let additional_props = additional_props.clone();

        move |evt| {
            use emit::Props as _;

            // Add the additional properties to the outgoing event
            let additional_props = &*additional_props.borrow();
            let evt = evt.map_props(|props| additional_props.and_props(props));

            // Emit the outgoing event
            emit::info!(evt, "Running an example");
        }
    }));

    if i > 4 {
        additional_props.borrow_mut().insert("is_big", true);
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
