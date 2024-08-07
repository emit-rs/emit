/*!
The [`Extent`] type.

An extent is the time for which an event is active. It may be either a point for an event that occurred at a particular time, or a span for an event that was active over a particular period.

Extents can be constructed directly, or generically through the [`ToExtent`] trait.
*/

use crate::{empty::Empty, timestamp::Timestamp};
use core::{fmt, ops::Range, time::Duration};

/**
Either a single [`Timestamp`] for a point in time, or a pair of [`Timestamp`]s for a timespan.
*/
#[derive(Clone)]
pub struct Extent {
    range: Range<Timestamp>,
    is_span: bool,
}

impl Extent {
    /**
    Create an extent for a point in time.
    */
    pub fn point(ts: Timestamp) -> Self {
        Extent {
            range: ts..ts,
            is_span: false,
        }
    }

    /**
    Create an extent for a timespan.

    The end of the range should be after the start, but an empty range is still considered a span.
    */
    pub fn span(ts: Range<Timestamp>) -> Self {
        Extent {
            range: ts,
            is_span: true,
        }
    }

    /**
    Get the extent as a range of timestamps.

    For point extents, this will return an empty range with the start and end bounds being equal. For span extents, this will return exactly the range the extent was created from.
    */
    pub fn as_range(&self) -> &Range<Timestamp> {
        &self.range
    }

    /**
    Get the extent as a point in time.

    For point extents, this will return exactly the value the extent was created from. For span extents, this will return the end bound.
    */
    pub fn as_point(&self) -> &Timestamp {
        &self.range.end
    }

    /**
    Try get the extent as a timespan.

    This method will return `Some` if the extent is a span, even if that span is empty. It will return `None` for point extents.
    */
    pub fn as_span(&self) -> Option<&Range<Timestamp>> {
        if self.is_span() {
            Some(&self.range)
        } else {
            None
        }
    }

    /**
    Try get the length of the extent.

    This method will return `Some` if the extent is a span, even if that span is empty. It will return `None` for point extents.
    */
    pub fn len(&self) -> Option<Duration> {
        if self.is_span() {
            self.range.end.duration_since(self.range.start)
        } else {
            None
        }
    }

    /**
    Whether the extent is a point in time.
    */
    pub fn is_point(&self) -> bool {
        !self.is_span()
    }

    /**
    Whether the extent is a timespan.
    */
    pub fn is_span(&self) -> bool {
        self.is_span
    }
}

impl fmt::Debug for Extent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_span() {
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
        if self.is_span() {
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
        Some(Extent::span(self.clone()))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point() {
        let ts = Extent::point(Timestamp::MIN);

        assert!(ts.is_point());
        assert!(!ts.is_span());

        assert_eq!(&Timestamp::MIN, ts.as_point());

        assert_eq!(&(Timestamp::MIN..Timestamp::MIN), ts.as_range());
        assert_eq!(None, ts.as_span());
        assert_eq!(None, ts.len());
    }

    #[test]
    fn span() {
        let ts = Extent::span(Timestamp::MIN..Timestamp::MIN + Duration::from_secs(1));

        assert!(!ts.is_point());
        assert!(ts.is_span());

        assert_eq!(&(Timestamp::MIN + Duration::from_secs(1)), ts.as_point());

        assert_eq!(
            &(Timestamp::MIN..Timestamp::MIN + Duration::from_secs(1)),
            ts.as_range()
        );
        assert_eq!(
            Some(&(Timestamp::MIN..Timestamp::MIN + Duration::from_secs(1))),
            ts.as_span()
        );
        assert_eq!(Some(Duration::from_secs(1)), ts.len());
    }

    #[test]
    fn span_empty() {
        let ts = Extent::span(Timestamp::MIN..Timestamp::MIN);

        assert!(!ts.is_point());
        assert!(ts.is_span());

        assert_eq!(&Timestamp::MIN, ts.as_point());

        assert_eq!(Some(Duration::from_secs(0)), ts.len());
    }

    #[test]
    fn span_backwards() {
        let ts = Extent::span(Timestamp::MAX..Timestamp::MIN);

        assert!(!ts.is_point());
        assert!(ts.is_span());

        assert_eq!(&Timestamp::MIN, ts.as_point());

        assert!(ts.len().is_none());
    }
}
