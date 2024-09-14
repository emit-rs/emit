/*!
This example demonstrates how to exclude a property that's `None`.
*/

use std::time::Duration;

fn example() {
    // The `emit::optional` attribute can be used to omit
    // capturing a value that's `None`, instead of capturing
    // it as `null`. If you remove the `emit::optional` attribute
    // and run the example, you'll see the `user` property captured
    // as `serde`'s `None` value. When it's present, it's not captured
    // at all
    emit::info!(
        "Hello, {user}",
        #[emit::optional]
        #[emit::as_serde]
        user: None::<&str>,
    );
}

fn main() {
    let rt = emit::setup()
        .emit_to(emit::emitter::from_fn(|evt| println!("{evt:?}")))
        .init();

    example();

    rt.blocking_flush(Duration::from_secs(5));
}
