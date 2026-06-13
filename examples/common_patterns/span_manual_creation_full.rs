/*!
This example is a variant of `span_manual_creation` that doesn't use the `emit::span_guard!` macro.

It instead constructs a `SpanGuard` directly, which is what the `emit::span_guard!` macro de-sugars to.
*/

use std::time::Duration;

fn example(i: i32) {
    let (mut span, frame) = emit::span::SpanGuard::from_runtime(
        emit::runtime::shared(),
        emit::props! {
            // Properties that will be shared by all events emitted within this span
        },
        emit::mdl!(),
        emit::props! {
            // Properties that will appear just on this span event
        },
    );

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
