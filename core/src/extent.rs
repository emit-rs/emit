/*!
The [`Extent`] type.

An extent is the time for which an event is active. It may be either a point for an event that occurred at a particular time, or a range for an event that was active over a particular period.

Extents can be constructed directly, or generically through the [`ToExtent`] trait.
*/

use crate::{
    empty::Empty,
    props::Props,
    str::{Str, ToStr},
    timestamp::Timestamp,
    value::{ToValue, Value},
    well_known::{KEY_TS, KEY_TS_START},
};
use core::{fmt, ops::ControlFlow, ops::Range, time::Duration};

/**
Either a single [`Timestamp`] for a point in time, or a pair of [`Timestamp`]s for a range.
*/
#[derive(Clone)]
pub struct Extent {
    range: Range<Timestamp>,
    is_range: bool,
}

impl Extent {
    /**
    Create an extent for a point in time.
    */
    pub fn point(ts: Timestamp) -> Self {
        Extent {
            range: ts..ts,
            is_range: false,
        }
    }

    /**
    Create an extent for a range.

    The end of the range should be after the start, but an empty range is still considered a range.
    */
    pub fn range(ts: Range<Timestamp>) -> Self {
        Extent {
            range: ts,
            is_range: true,
        }
    }

    /**
    Get the extent as a point in time.

    For point extents, this will return exactly the value the extent was created from. For range extents, this will return the end bound.
    */
    pub fn as_point(&self) -> &Timestamp {
        &self.range.end
    }

    /**
    Try get the extent as a range.

    This method will return `Some` if the extent is a range, even if that range is empty. It will return `None` for point extents.
    */
    pub fn as_range(&self) -> Option<&Range<Timestamp>> {
        if self.is_range() {
            Some(&self.range)
        } else {
            None
        }
    }

    /**
    Try get the length of the extent.

    This method will return `Some` if the extent is a range, even if that range is empty. It will return `None` for point extents.
    */
    pub fn len(&self) -> Option<Duration> {
        if self.is_range() {
            self.range.end.duration_since(self.range.start)
        } else {
            None
        }
    }

    /**
    Whether the extent is a single point in time.
    */
    pub fn is_point(&self) -> bool {
        !self.is_range()
    }

    /**
    Whether the extent is a range.
    */
    pub fn is_range(&self) -> bool {
        self.is_range
    }
}

impl fmt::Debug for Extent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_range() {
            fmt::Debug::fmt(&self.range.start, f)?;
            f.write_str("..")?;
            fmt::Debug::fmt(&self.range.end, f)
        } else {
            fmt::Debug::fmt(&self.range.end, f)
        }
    }
}

impl fmt::Display for Extent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.is_range() {
            fmt::Display::fmt(&self.range.start, f)?;
            f.write_str("..")?;
            fmt::Display::fmt(&self.range.end, f)
        } else {
            fmt::Display::fmt(&self.range.end, f)
        }
    }
}

/**
Try convert a value into an [`Extent`].
*/
pub trait ToExtent {
    /**
    Perform the conversion.
    */
    fn to_extent(&self) -> Option<Extent>;
}

impl<'a, T: ToExtent + ?Sized> ToExtent for &'a T {
    fn to_extent(&self) -> Option<Extent> {
        (**self).to_extent()
    }
}

impl ToExtent for Empty {
    fn to_extent(&self) -> Option<Extent> {
        None
    }
}

impl<T: ToExtent> ToExtent for Option<T> {
    fn to_extent(&self) -> Option<Extent> {
        self.as_ref().and_then(|ts| ts.to_extent())
    }
}

impl ToExtent for Extent {
    fn to_extent(&self) -> Option<Extent> {
        Some(self.clone())
    }
}

impl ToExtent for Timestamp {
    fn to_extent(&self) -> Option<Extent> {
        Some(Extent::point(*self))
    }
}

impl ToExtent for Range<Timestamp> {
    fn to_extent(&self) -> Option<Extent> {
        Some(Extent::range(self.clone()))
    }
}

impl ToExtent for Range<Option<Timestamp>> {
    fn to_extent(&self) -> Option<Extent> {
        match (self.start, self.end) {
            (Some(start), Some(end)) => (start..end).to_extent(),
            (Some(start), None) => start.to_extent(),
            (None, Some(end)) => end.to_extent(),
            (None, None) => None::<Timestamp>.to_extent(),
        }
    }
}

impl Props for Extent {
    fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        mut for_each: F,
    ) -> ControlFlow<()> {
        if let Some(range) = self.as_range() {
            for_each(KEY_TS_START.to_str(), range.start.to_value())?;
            for_each(KEY_TS.to_str(), range.end.to_value())
        } else {
            for_each(KEY_TS.to_str(), self.as_point().to_value())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point() {
        let ts = Extent::point(Timestamp::MIN);

        assert!(ts.is_point());
        assert!(!ts.is_range());

        assert_eq!(&Timestamp::MIN, ts.as_point());

        assert_eq!(None, ts.as_range());
        assert_eq!(None, ts.len());
    }

    #[test]
    fn range() {
        let ts = Extent::range(Timestamp::MIN..Timestamp::MIN + Duration::from_secs(1));

        assert!(!ts.is_point());
        assert!(ts.is_range());

        assert_eq!(&(Timestamp::MIN + Duration::from_secs(1)), ts.as_point());

        assert_eq!(
            &(Timestamp::MIN..Timestamp::MIN + Duration::from_secs(1)),
            ts.as_range().unwrap(),
        );
        assert_eq!(
            Some(&(Timestamp::MIN..Timestamp::MIN + Duration::from_secs(1))),
            ts.as_range()
        );
        assert_eq!(Some(Duration::from_secs(1)), ts.len());
    }

    #[test]
    fn range_empty() {
        let ts = Extent::range(Timestamp::MIN..Timestamp::MIN);

        assert!(!ts.is_point());
        assert!(ts.is_range());

        assert_eq!(&Timestamp::MIN, ts.as_point());

        assert_eq!(Some(Duration::from_secs(0)), ts.len());
    }

    #[test]
    fn range_backwards() {
        let ts = Extent::range(Timestamp::MAX..Timestamp::MIN);

        assert!(!ts.is_point());
        assert!(ts.is_range());

        assert_eq!(&Timestamp::MIN, ts.as_point());

        assert!(ts.len().is_none());
    }

    #[test]
    fn as_props() {
        let ts = Extent::point(Timestamp::MIN);

        assert_eq!(Timestamp::MIN, ts.pull::<Timestamp, _>("ts").unwrap());

        let ts = Extent::range(Timestamp::MIN..Timestamp::MAX);

        assert_eq!(Timestamp::MAX, ts.pull::<Timestamp, _>("ts").unwrap());
        assert_eq!(Timestamp::MIN, ts.pull::<Timestamp, _>("ts_start").unwrap());
    }
}
