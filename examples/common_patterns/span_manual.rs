use std::time::Duration;

fn example(i: i32) {
    // Start a timer to track the runtime of the span
    let timer = emit::Timer::start(emit::clock());

    // Generate trace and span ids
    let ctxt = emit::SpanCtxt::current(emit::ctxt()).new_child(emit::rng());

    // Push into the ambient context, so emitted events see the trace and span ids
    ctxt.push(emit::ctxt(), emit::props! {}).call(|| {
        let r = i + 1;

        if r == 4 {
            // Emit a span event on completion
            emit::error!(
                event: emit::Span::new(
                    emit::module!(),
                    timer,
                    "example",
                    emit::props! {},
                ),
                "Running an example failed with {r}",
            );
        } else {
            // Emit a span event on completion
            emit::info!(
                event: emit::Span::new(
                    emit::module!(),
                    timer,
                    "example",
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
