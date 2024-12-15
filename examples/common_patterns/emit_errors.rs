/*!
This examples demonstrates how to emit an event with an error.
*/

use std::{io, time::Duration};

fn example() {
    // Define an error type that implements `std::error::Error`
    #[derive(Debug, thiserror::Error)]
    #[error("task failure")]
    pub struct Error {
        #[source]
        source: io::Error,
    }

    let err = Error {
        source: io::Error::new(io::ErrorKind::Other, "Some IO error"),
    };

    // If a property uses the key `err` then it's captured using its `std::error::Error` implementation by default.
    // Emitters use the well-known `err` property to find the error on events and may handle them differently.
    emit::warn!("Failed to perform some task due to {err}");
}

fn main() {
    let rt = emit::setup().emit_to(emit_term::stdout()).init();

    example();

    rt.blocking_flush(Duration::from_secs(5));
}
