use std::time::Duration;

fn example() {
    // The `emit::key` attribute can be used to give a property a
    // name that isn't a valid Rust identifier
    emit::info!(
        "Hello, {user}",
        #[emit::key("user.name")]
        user: "Rust",
    );
}

fn main() {
    let rt = emit::setup()
        .emit_to(emit::emitter::from_fn(|evt| println!("{evt:?}")))
        .init();

    example();

    rt.blocking_flush(Duration::from_secs(5));
}
