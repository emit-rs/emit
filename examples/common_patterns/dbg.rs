/*!
This example demonstrates how to use `emit::dbg!` for quick-and-dirty debugging.
*/

fn main() {
    let rt = emit::setup().emit_to(emit_term::stdout()).init();

    let id = 42;
    let user = "Rust";

    emit::dbg!(user, id);

    rt.blocking_flush(std::time::Duration::from_secs(5));
}
