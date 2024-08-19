fn main() {
    let rt = emit::setup().emit_to(emit_term::stdout()).init();

    let user = "Rust";

    emit::info!("Hello, {user}");

    rt.blocking_flush(std::time::Duration::from_secs(5));
}
