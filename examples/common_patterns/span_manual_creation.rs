/*!
This example demonstrates how to construct a span manually, without using `#[span]`.

It can be useful in applications that have more complex control flow or organization that makes picking a function to annotate difficult.

This example differs from `span_manual_creation_full` in still using the same `ActiveSpan` type that the `#[span]` attribute generates for you.
*/

use std::time::Duration;

fn example(i: i32) {
    let (span, frame) = emit::start_span!("example");

    // Execute our code within the context of the frame
    // If this function was async, then you would use `frame.in_future(..).await`
    //
    // NOTE: The `span` guard *must* be moved into this closure, otherwise your
    // span will complete early
    frame.call(move || {
        let r = i + 1;

        if r == 4 {
            // Emit a span event on completion
            span.complete_with(emit::span::completion::from_fn(|evt| {
                emit::error!(evt, "Running an example failed with {r}")
            }));
        } else {
            // The span will complete when it goes out of scope, but it's good
            // to have a call using `span` in the closure so `span` is moved into it
            span.complete();
        }
    })
}

fn main() {
    let rt = emit::setup().emit_to(emit_term::stdout()).init();

    example(1);
    example(3);

    rt.blocking_flush(Duration::from_secs(5));
}
