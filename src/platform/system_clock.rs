/*!
The [`SystemClock`] type.
*/

use emit_core::{clock::Clock, runtime::InternalClock, timestamp::Timestamp};

/**
A [`Clock`] based on the standard library's [`std::time::SystemTime`].
*/
#[derive(Default, Debug, Clone, Copy)]
pub struct SystemClock {}

impl SystemClock {
    /**
    Create a new clock.
    */
    pub const fn new() -> Self {
        SystemClock {}
    }
}

impl Clock for SystemClock {
    fn now(&self) -> Option<Timestamp> {
        Timestamp::from_unix(std::time::UNIX_EPOCH.elapsed().unwrap_or_default())
    }
}

impl InternalClock for SystemClock {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn now() {
        todo!()
    }
}
