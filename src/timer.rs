/*!
The [`Timer`] type.

Timers are a simple mechanism to track the start and end times of some operation. They're based on readings from a [`Clock`], which isn't guaranteed to be monotonic. That means timers can give an approximate timespan based on its readings, but are susceptible to clock drift.

Timers are used by [`crate::Span`]s to produce the [`Extent`] on their events.
*/

use core::time::Duration;

use crate::{extent::ToExtent, Clock, Extent, Timestamp};

/**
A timer that measures the point when an operation started and ended.
*/
#[derive(Clone, Copy)]
pub struct Timer<C> {
    start: Option<Timestamp>,
    clock: C,
}

impl<C: Clock> Timer<C> {
    /**
    Start a timer using [`Clock::now`] as its initial reading.
    */
    pub fn start(clock: C) -> Self {
        Timer {
            start: clock.now(),
            clock,
        }
    }

    /**
    Get the timestamp taken when the timer was started.

    If the underlying [`Clock`] was unable to produce an initial reading then this method will return `None`.
    */
    pub fn start_timestamp(&self) -> Option<Timestamp> {
        self.start
    }

    /**
    Get the value of the timer as an [`Extent`], using [`Clock::now`] as its final reading.

    If the underlying [`Clock`] is unable to produce a reading then this method will return `None`.
    */
    pub fn extent(&self) -> Option<Extent> {
        let end = self.clock.now();

        match (self.start, end) {
            (Some(start), Some(end)) => Some(Extent::range(start..end)),
            _ => None,
        }
    }

    /**
    Get the timespan between the initial reading and [`Clock::now`].

    If the underlying [`Clock`] is unable to produce a reading, or it shifts to before the initial reading, then this method will return `None`.

    This method is not guaranteed to return the actual time elapsed since the timer was started.
    It's based on the difference between two readings of the underlying [`Clock`], which is not guaranteed to be monotonic.
    */
    pub fn elapsed(&self) -> Option<Duration> {
        self.extent().and_then(|extent| extent.len())
    }

    /**
    Get a new timer using the same initial reading, borrowing the [`Clock`] from this one.
    */
    pub fn by_ref(&self) -> Timer<&C> {
        Timer {
            start: self.start,
            clock: &self.clock,
        }
    }
}

impl<C: Clock> ToExtent for Timer<C> {
    fn to_extent(&self) -> Option<Extent> {
        self.extent()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    struct MyClock(Cell<Timestamp>);

    impl Clock for MyClock {
        fn now(&self) -> Option<Timestamp> {
            Some(self.0.get())
        }
    }

    #[test]
    fn timer_forwards() {
        let clock = MyClock(Cell::new(
            Timestamp::from_unix(Duration::from_secs(0)).unwrap(),
        ));

        let timer = Timer::start(&clock);

        assert_eq!(
            Timestamp::from_unix(Duration::from_secs(0)).unwrap(),
            timer.start_timestamp().unwrap()
        );

        assert_eq!(Duration::from_secs(0), timer.elapsed().unwrap());

        assert_eq!(
            Timestamp::from_unix(Duration::from_secs(0)).unwrap()
                ..Timestamp::from_unix(Duration::from_secs(0)).unwrap(),
            timer.extent().unwrap().as_range().unwrap().clone()
        );

        clock
            .0
            .set(Timestamp::from_unix(Duration::from_secs(1)).unwrap());

        assert_eq!(Duration::from_secs(1), timer.elapsed().unwrap());

        assert_eq!(
            Timestamp::from_unix(Duration::from_secs(0)).unwrap()
                ..Timestamp::from_unix(Duration::from_secs(1)).unwrap(),
            timer.extent().unwrap().as_range().unwrap().clone()
        );
    }

    #[test]
    fn timer_backwards() {
        let clock = MyClock(Cell::new(
            Timestamp::from_unix(Duration::from_secs(1)).unwrap(),
        ));

        let timer = Timer::start(&clock);

        clock
            .0
            .set(Timestamp::from_unix(Duration::from_secs(0)).unwrap());

        assert_eq!(None, timer.elapsed());

        assert_eq!(
            Timestamp::from_unix(Duration::from_secs(1)).unwrap()
                ..Timestamp::from_unix(Duration::from_secs(0)).unwrap(),
            timer.extent().unwrap().as_range().unwrap().clone()
        );
    }

    #[test]
    fn timer_empty() {
        let timer = Timer::start(crate::Empty);

        assert!(timer.start_timestamp().is_none());
        assert!(timer.elapsed().is_none());
    }
}
