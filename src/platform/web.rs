/*!
Components provided by the JavaScript web platform.
*/

use core::time::Duration;

use js_sys::Date;

use emit_core::{
    clock::Clock,
    rng::Rng,
    runtime::{InternalClock, InternalRng},
    timestamp::Timestamp,
};

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

/**
An RNG based on the [Crypto API](https://developer.mozilla.org/en-US/docs/Web/API/Crypto).
*/
#[derive(Default, Debug, Clone, Copy)]
pub struct CryptoRng {}

impl CryptoRng {
    /**
    Create a new instance of the crypto RNG.
    */
    pub const fn new() -> Self {
        CryptoRng {}
    }
}

impl Rng for CryptoRng {
    fn fill<A: AsMut<[u8]>>(&self, mut arr: A) -> Option<A> {
        crypto_fill(arr.as_mut());

        Some(arr)
    }
}

impl InternalRng for CryptoRng {}

fn crypto_fill(buf: &mut [u8]) {
    crypto::get_random_values(buf);
}

mod crypto {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = crypto, js_name = getRandomValues)]
        pub fn get_random_values(buf: &mut [u8]);
    }
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

    #[wasm_bindgen_test]
    #[test]
    fn crypto_rng_produces_random_data() {
        let mut buf = [0; 32];

        CryptoRng::new().fill(&mut buf).unwrap();

        assert_ne!([0; 32], buf);
    }
}
