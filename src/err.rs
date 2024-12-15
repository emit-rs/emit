/*!
Utilities for working with the `err` well-known property.
*/

use std::error::Error;

/**
Convert an error implementing `AsRef<dyn Error + 'static>` into an [`Error`].

This method can be used to accept types like `anyhow::Error` when using the `err` property.
*/
pub fn as_ref<'a>(err: &'a impl AsRef<dyn Error + 'static>) -> &'a (dyn Error + 'static) {
    err.as_ref()
}
