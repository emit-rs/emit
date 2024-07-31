/*!
The [`Rng`] type.
*/

use crate::empty::Empty;

/**
A non-cryptographic source of randomness.
*/
pub trait Rng {
    /**
    Fill a buffer with random data.

    This method may return `None` if the buffer couldn't be filled with random data for any reason.

    The buffer is expected to have a consistent implementation of `AsMut` that always returns a reference to the same underlying bytes. This isn't required for safety, but can lead to unexpected results.
    */
    fn fill<A: AsMut<[u8]>>(&self, arr: A) -> Option<A>;

    /**
    Get a random 64bit number.
    */
    fn gen_u64(&self) -> Option<u64> {
        self.fill([0; 8]).map(u64::from_le_bytes)
    }

    /**
    Get a random 128bit number.
    */
    fn gen_u128(&self) -> Option<u128> {
        self.fill([0; 16]).map(u128::from_le_bytes)
    }
}

impl<'a, T: Rng + ?Sized> Rng for &'a T {
    fn fill<A: AsMut<[u8]>>(&self, arr: A) -> Option<A> {
        (**self).fill(arr)
    }

    fn gen_u64(&self) -> Option<u64> {
        (**self).gen_u64()
    }

    fn gen_u128(&self) -> Option<u128> {
        (**self).gen_u128()
    }
}

impl<'a, T: Rng> Rng for Option<T> {
    fn fill<A: AsMut<[u8]>>(&self, arr: A) -> Option<A> {
        self.as_ref().and_then(|id| id.fill(arr))
    }

    fn gen_u64(&self) -> Option<u64> {
        self.as_ref().and_then(|id| id.gen_u64())
    }

    fn gen_u128(&self) -> Option<u128> {
        self.as_ref().and_then(|id| id.gen_u128())
    }
}

#[cfg(feature = "alloc")]
impl<'a, T: Rng + ?Sized + 'a> Rng for alloc::boxed::Box<T> {
    fn fill<A: AsMut<[u8]>>(&self, arr: A) -> Option<A> {
        (**self).fill(arr)
    }

    fn gen_u64(&self) -> Option<u64> {
        (**self).gen_u64()
    }

    fn gen_u128(&self) -> Option<u128> {
        (**self).gen_u128()
    }
}

#[cfg(feature = "alloc")]
impl<'a, T: Rng + ?Sized + 'a> Rng for alloc::sync::Arc<T> {
    fn fill<A: AsMut<[u8]>>(&self, arr: A) -> Option<A> {
        (**self).fill(arr)
    }

    fn gen_u64(&self) -> Option<u64> {
        (**self).gen_u64()
    }

    fn gen_u128(&self) -> Option<u128> {
        (**self).gen_u128()
    }
}

impl Rng for Empty {
    fn fill<A: AsMut<[u8]>>(&self, _: A) -> Option<A> {
        None
    }
}

mod internal {
    pub trait DispatchRng {
        fn dispatch_gen(&self, arr: &mut [u8]) -> bool;
        fn dispatch_gen_u64(&self) -> Option<u64>;
        fn dispatch_gen_u128(&self) -> Option<u128>;
    }

    pub trait SealedRng {
        fn erase_rng(&self) -> crate::internal::Erased<&dyn DispatchRng>;
    }
}

/**
An object-safe [`Rng`].

A `dyn ErasedRng` can be treated as `impl Rng`.
*/
pub trait ErasedRng: internal::SealedRng {}

impl<T: Rng> ErasedRng for T {}

impl<T: Rng> internal::SealedRng for T {
    fn erase_rng(&self) -> crate::internal::Erased<&dyn internal::DispatchRng> {
        crate::internal::Erased(self)
    }
}

impl<T: Rng> internal::DispatchRng for T {
    fn dispatch_gen(&self, arr: &mut [u8]) -> bool {
        self.fill(arr).is_some()
    }

    fn dispatch_gen_u64(&self) -> Option<u64> {
        self.gen_u64()
    }

    fn dispatch_gen_u128(&self) -> Option<u128> {
        self.gen_u128()
    }
}

impl<'a> Rng for dyn ErasedRng + 'a {
    fn fill<A: AsMut<[u8]>>(&self, mut arr: A) -> Option<A> {
        if self.erase_rng().0.dispatch_gen(arr.as_mut()) {
            Some(arr)
        } else {
            None
        }
    }

    fn gen_u64(&self) -> Option<u64> {
        self.erase_rng().0.dispatch_gen_u64()
    }

    fn gen_u128(&self) -> Option<u128> {
        self.erase_rng().0.dispatch_gen_u128()
    }
}

impl<'a> Rng for dyn ErasedRng + Send + Sync + 'a {
    fn fill<A: AsMut<[u8]>>(&self, arr: A) -> Option<A> {
        (self as &(dyn ErasedRng + 'a)).fill(arr)
    }

    fn gen_u64(&self) -> Option<u64> {
        (self as &(dyn ErasedRng + 'a)).gen_u64()
    }

    fn gen_u128(&self) -> Option<u128> {
        (self as &(dyn ErasedRng + 'a)).gen_u128()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MyRng {
        fill: u8,
    }

    impl Rng for MyRng {
        fn fill<A: AsMut<[u8]>>(&self, mut arr: A) -> Option<A> {
            for b in arr.as_mut() {
                *b = self.fill;
            }

            Some(arr)
        }
    }

    #[test]
    fn gen() {
        for (rng, expected_u64, expected_u128) in [
            (MyRng { fill: 0 }, 0u64, 0u128),
            (MyRng { fill: u8::MAX }, u64::MAX, u128::MAX),
        ] {
            assert_eq!(expected_u64, rng.gen_u64().unwrap());
            assert_eq!(expected_u128, rng.gen_u128().unwrap());
        }
    }

    #[test]
    fn option_rng() {
        for (rng, expected) in [(Some(MyRng { fill: 42 }), Some([42u8, 42])), (None, None)] {
            assert_eq!(expected, rng.fill([0, 0]));
        }
    }

    #[test]
    fn erased_rng() {
        let rng = MyRng { fill: 42 };

        let rng = &rng as &dyn ErasedRng;

        assert_eq!([42, 42], rng.fill([0, 0]).unwrap());
    }
}
