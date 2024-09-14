/*!
This example demonstrates how to construct a span manually, without using `#[emit::span]`.

It can be useful in applications that use disconnected middleware that makes it difficult to pick a single point to introduce `#[emit::span]` to.
*/

use std::time::Duration;

fn example(i: i32) {
    // Start a timer to track the runtime of the span
    let timer = emit::Timer::start(emit::clock());

    // Generate trace and span ids
    let ctxt = emit::SpanCtxt::current(emit::ctxt()).new_child(emit::rng());

    // Push into the ambient context, so emitted events see the trace and span ids
    let frame = ctxt.push(emit::ctxt());

    // Execute our code within the context of the frame
    // If this function was async, then you would use `frame.in_future(..).await`
    frame.call(|| {
        let r = i + 1;

        if r == 4 {
            // Emit a span event on completion
            emit::error!(
                evt: emit::Span::new(
                    emit::mdl!(),
                    "example",
                    timer,
                    emit::props! {},
                ),
                "Running an example failed with {r}",
            );
        } else {
            // Emit a span event on completion
            emit::info!(
                evt: emit::Span::new(
                    emit::mdl!(),
                    "example",
                    timer,
                    emit::props! {},
                ),
                "Running an example produced {r}",
            );
        }
    })
}

fn main() {
    let rt = emit::setup().emit_to(emit_term::stdout()).init();

    example(1);
    example(3);

    rt.blocking_flush(Duration::from_secs(5));
}
