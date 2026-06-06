fn main() {
    emit_fuzz::main(de);
}

pub fn de(input: &[u8]) {
    let Ok(input) = std::str::from_utf8(input) else {
        return;
    };

    let _ = baggage::parse(input);
}

mod error {
    use std::{error, fmt};

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
}

use error::*;

#[path = "../../emitter/otlp/src/baggage.rs"]
mod baggage;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial() {
        emit_fuzz::initial_cases("otel_baggage", de);
    }

    #[test]
    fn repro() {
        emit_fuzz::repro_cases("otel_baggage", de);
    }
}
