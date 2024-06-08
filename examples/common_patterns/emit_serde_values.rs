use std::time::Duration;

fn example() {
    #[derive(serde::Serialize)]
    pub struct User<'a> {
        id: usize,
        name: &'a str,
    }

    // The `emit::serde` attribute captures a property
    // using its `serde::Serialize` implementation. It's
    // good for any complex data types you define or that
    // come from external libraries
    emit::info!(
        "Hello, {user}",
        #[emit::as_serde]
        user: User {
            id: 42,
            name: "Rust",
        },
    );
}

fn main() {
    let rt = emit::setup().emit_to(emit_term::stdout()).init();

    example();

    rt.blocking_flush(Duration::from_secs(5));
}
