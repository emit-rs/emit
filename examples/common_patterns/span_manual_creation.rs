/*!
This example demonstrates how to construct a span manually, without using `#[span]`.

It can be useful in applications that have more complex control flow or organization that makes picking a function to annotate difficult.

This example differs from `span_manual_full` in still using the same `SpanGuard` type that the `#[span]` attribute generates for you.
*/

use std::time::Duration;

fn example(i: i32) {
    let (mut span, frame) = emit::new_span!("example");

    // Execute our code within the context of the frame
    // If this function was async, then you would use `frame.in_future(..).await`
    frame.call(move || {
        // Call `start` on the span to begin it
        // This *must* be done in the body of `frame.call` or `frame.in_future`
        span.start();

        let r = i + 1;

        if r == 4 {
            // Emit a span event on completion
            span.complete_with(emit::span::completion::from_fn(|evt| {
                emit::error!(evt, "Running an example failed with {r}")
            }));
        }
    })
}

fn main() {
    let rt = emit::setup().emit_to(emit_term::stdout()).init();

    example(1);
    example(3);

    rt.blocking_flush(Duration::from_secs(5));
}
