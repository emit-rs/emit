/*!
The [`RandRng`] type.
*/

use emit_core::{rng::Rng, runtime::InternalRng};
use rand::{Rng as _, RngExt as _};

/**
An [`Rng`] based on the [`rand`] library.
*/
#[derive(Default, Debug, Clone, Copy)]
pub struct RandRng {}

impl RandRng {
    /**
    Create a new source of randomness.
    */
    pub const fn new() -> Self {
        RandRng {}
    }
}

impl Rng for RandRng {
    fn fill<A: AsMut<[u8]>>(&self, mut arr: A) -> Option<A> {
        rand::rng().fill_bytes(arr.as_mut());

        Some(arr)
    }

    fn gen_u64(&self) -> Option<u64> {
        Some(rand::rng().random())
    }

    fn gen_u128(&self) -> Option<u128> {
        Some(rand::rng().random())
    }
}

impl InternalRng for RandRng {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gen() {
        assert_ne!(RandRng::new().gen_u128(), RandRng::new().gen_u128());
        assert_ne!(RandRng::new().gen_u64(), RandRng::new().gen_u64());
        assert_ne!(RandRng::new().fill([0; 32]), RandRng::new().fill([0; 32]));
    }
}
