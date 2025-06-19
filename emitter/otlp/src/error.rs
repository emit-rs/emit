use std::{error, fmt};

/**
An error attempting to configure a [`crate::Otlp`] instance.
*/
#[derive(Debug)]
pub struct Error {
    msg: String,
    cause: Option<Box<dyn error::Error + Send + Sync>>,
}

impl Error {
    pub(crate) fn msg(msg: impl fmt::Display) -> Self {
        Error {
            msg: msg.to_string(),
            cause: None,
        }
    }

    pub(crate) fn new(
        msg: impl fmt::Display,
        e: impl error::Error + Send + Sync + 'static,
    ) -> Self {
        Error {
            msg: msg.to_string(),
            cause: Some(Box::new(e)),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        self.cause
            .as_ref()
            .map(|source| &**source as &(dyn error::Error + 'static))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.msg, f)
    }
}
