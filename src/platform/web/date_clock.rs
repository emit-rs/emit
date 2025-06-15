/*!
The [`DateClock`] type.
*/

use core::time::Duration;

use js_sys::Date;

use emit_core::{clock::Clock, runtime::InternalClock, timestamp::Timestamp};

/**
A clock based on the [Date type](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Date).
*/
#[derive(Default, Debug, Clone, Copy)]
pub struct DateClock {}

impl DateClock {
    /**
    Create a new instance of the date clock.
    */
    pub const fn new() -> Self {
        DateClock {}
    }
}

impl Clock for DateClock {
    fn now(&self) -> Option<Timestamp> {
        Timestamp::from_unix(date_now())
    }
}

impl InternalClock for DateClock {}

fn date_now() -> Duration {
    let timestamp_millis = Date::new_0().get_time();

    let timestamp_nanos = (timestamp_millis * 1_000_000.0) as u128;

    let timestamp_secs = (timestamp_nanos / 1_000_000_000) as u64;
    let timestamp_subsec_nanos = (timestamp_nanos % 1_000_000_000) as u32;

    Duration::new(timestamp_secs, timestamp_subsec_nanos)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    #[test]
    fn date_clock_produces_timestamps() {
        assert_ne!(Timestamp::MIN, DateClock::new().now().unwrap());
    }
}
