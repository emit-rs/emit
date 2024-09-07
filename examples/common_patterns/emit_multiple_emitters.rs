use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Use `emit_to` to set an emitter
    // Use `and_emit_to` to append another emitter
    let rt = emit::setup()
        .emit_to(emit_term::stdout())
        .and_emit_to(emit_file::set("./target/logs/emit_multiple_emitters.log").spawn())
        .init();

    emit::info!("Hello, {user}", user: "Rust");

    rt.blocking_flush(Duration::from_secs(5));

    Ok(())
}
