use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // The `filter::wrap` method takes an `emit::Filter` and an `emit::Emitter`
    // and only emits events that match the filter.
    //
    // This is different from the global filter you set in `setup.emit_when`,
    // which is applied to all emitted events, but can be bypassed.
    let rt = emit::setup()
        .emit_to(emit::filter::wrap(
            emit::level::min_filter(emit::Level::Warn),
            emit_term::stdout(),
        ))
        .and_emit_to(emit_file::set("./target/logs/filter_per_emitter.log").spawn()?)
        .init();

    emit::info!("Hello, {user}", user: "Rust");

    rt.blocking_flush(Duration::from_secs(5));

    Ok(())
}
