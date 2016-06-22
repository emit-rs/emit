pub mod json;
pub mod text;
pub mod raw;

use std::error::Error;
use events::Event;
use std::io::Write;

pub trait TextFormatter {
    fn format(&self, event: &Event<'static>, to: &mut Write) -> Result<(), Box<Error>>;
}
