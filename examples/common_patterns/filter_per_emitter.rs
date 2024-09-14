/*!
This example demonstrates how to use a wrapping to filter events to a specific emitter.
*/

use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rt = emit::setup()
        // The `emit::emitter::wrap` method wraps an emitter in middleware that can transform
        // and filter events before they're passed through to it.
        //
        // The `emit::wrapping::from_filter` function is a wrapping that applies a filter.
        // Only events that match the filter are passed through to the underlying emitter.
        //
        // This is different from the global filter you set in `setup.emit_when`,
        // which is applied to all emitted events, but can be bypassed. A filter you set using
        // a wrapping cannot be bypassed.
        .emit_to(emit::emitter::wrap(
            emit_term::stdout(),
            emit::emitter::wrapping::from_filter(emit::level::min_filter(emit::Level::Warn)),
        ))
        .and_emit_to(emit_file::set("./target/logs/filter_per_emitter.log").spawn())
        .init();

    emit::info!("Hello, {user}", user: "Rust");

    rt.blocking_flush(Duration::from_secs(5));

    Ok(())
}
