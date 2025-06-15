/*!
The [`CryptoRng`] type.
*/

use emit_core::{rng::Rng, runtime::InternalRng};

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
    fn crypto_rng_produces_random_data() {
        let mut buf = [0; 32];

        CryptoRng::new().fill(&mut buf).unwrap();

        assert_ne!([0; 32], buf);
    }
}
