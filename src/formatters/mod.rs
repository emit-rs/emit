pub mod json;
pub mod text;
pub mod raw;

use std::error::Error;
use events::Event;
use std::io::Write;

/// Implementers can write a representation of an event to a binary stream.
pub trait WriteEvent {
    fn write_event(&self, event: &Event<'static>, to: &mut Write) -> Result<(), Box<Error>>;
}
