/*!
The [`Clock`] type.

A clock is a service that returns a [`Timestamp`] representing the current point in time. Clock readings are not guaranteed to be monotonic. They may move forwards or backwards arbitrarily, but for diagnostics to be useful, a clock should strive for accuracy.
*/

use crate::{empty::Empty, timestamp::Timestamp};

/**
A service to measure the current time.
*/
pub trait Clock {
    /**
    Read the current time.

    This method may return `None` if the clock couldn't be read for any reason. That may involve the clock not actually supporting reading now, time moving backwards, or any other reason that could result in an inaccurate reading.
    */
    fn now(&self) -> Option<Timestamp>;
}

impl<'a, T: Clock + ?Sized> Clock for &'a T {
    fn now(&self) -> Option<Timestamp> {
        (**self).now()
    }
}

impl<'a, T: Clock> Clock for Option<T> {
    fn now(&self) -> Option<Timestamp> {
        if let Some(time) = self {
            time.now()
        } else {
            Empty.now()
        }
    }
}

#[cfg(feature = "alloc")]
impl<'a, T: Clock + ?Sized + 'a> Clock for alloc::boxed::Box<T> {
    fn now(&self) -> Option<Timestamp> {
        (**self).now()
    }
}

#[cfg(feature = "alloc")]
impl<'a, T: Clock + ?Sized + 'a> Clock for alloc::sync::Arc<T> {
    fn now(&self) -> Option<Timestamp> {
        (**self).now()
    }
}

impl Clock for Empty {
    fn now(&self) -> Option<Timestamp> {
        None
    }
}

mod internal {
    use super::Timestamp;

    pub trait DispatchClock {
        fn dispatch_now(&self) -> Option<Timestamp>;
    }

    pub trait SealedClock {
        fn erase_clock(&self) -> crate::internal::Erased<&dyn DispatchClock>;
    }
}

/**
An object-safe [`Clock`].

A `dyn ErasedClock` can be treated as `impl Clock`.
*/
pub trait ErasedClock: internal::SealedClock {}

impl<T: Clock> ErasedClock for T {}

impl<T: Clock> internal::SealedClock for T {
    fn erase_clock(&self) -> crate::internal::Erased<&dyn internal::DispatchClock> {
        crate::internal::Erased(self)
    }
}

impl<T: Clock> internal::DispatchClock for T {
    fn dispatch_now(&self) -> Option<Timestamp> {
        self.now()
    }
}

impl<'a> Clock for dyn ErasedClock + 'a {
    fn now(&self) -> Option<Timestamp> {
        self.erase_clock().0.dispatch_now()
    }
}

impl<'a> Clock for dyn ErasedClock + Send + Sync + 'a {
    fn now(&self) -> Option<Timestamp> {
        (self as &(dyn ErasedClock + 'a)).now()
    }
}

#[cfg(test)]
mod tests {
    use core::{cell::Cell, time::Duration};

    use super::*;

    #[test]
    fn erased_clock() {
        struct SomeClock {
            now: Cell<usize>,
        }

        impl Clock for SomeClock {
            fn now(&self) -> Option<Timestamp> {
                self.now.set(self.now.get() + 1);

                Some(Timestamp::from_unix(Duration::from_secs(97)).unwrap())
            }
        }

        let clock = SomeClock {
            now: Cell::new(0),
        };

        let _ = (&clock as &dyn ErasedClock).now();

        assert_eq!(1, clock.now.get());
    }
}
