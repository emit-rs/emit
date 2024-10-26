/*!
This example demonstrates how to emit complex values using `std::fmt`.
*/

use std::time::Duration;

fn example() {
    #[derive(Debug)]
    pub struct User<'a> {
        id: usize,
        name: &'a str,
    }

    // The `emit::as_debug` attribute captures a property
    // using its `fmt::Debug` implementation
    emit::info!(
        "Hello, {user}",
        #[emit::as_debug]
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
