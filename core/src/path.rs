/*!
The [`Path`] type.

A path is a hierarchical identifier with fragments separated by `::`. The following are all valid paths:

- `a`.
- `a::b`.
- `::c`.

The path syntax is a subset of Rust's paths. The following are not valid paths:

- `::`.
- `a::`.
- `a::::b`.
- `a::*`.
- `a::{b, c}`.

Paths are used to represent the module on [`crate::event::Event`]s.
*/

use core::{
    cmp::Ordering,
    fmt,
    hash::{Hash, Hasher},
    str,
};

use crate::{
    str::Str,
    value::{FromValue, ToValue, Value},
};

/**
A hierarchical identifier, such as `a::b::c`.
*/
// TODO: Handle cases of invalid paths like `a::`, `::`, `a::::b`
#[derive(Clone)]
pub struct Path<'a>(Str<'a>);

impl<'a> From<&'a str> for Path<'a> {
    fn from(value: &'a str) -> Self {
        Path(Str::from(value))
    }
}

impl<'a> From<Str<'a>> for Path<'a> {
    fn from(value: Str<'a>) -> Self {
        Path(value)
    }
}

impl<'a, 'b> From<&'a Path<'b>> for Path<'a> {
    fn from(value: &'a Path<'b>) -> Self {
        value.by_ref()
    }
}

impl<'a> ToValue for Path<'a> {
    fn to_value(&self) -> Value {
        self.0.to_value()
    }
}

impl<'a> FromValue<'a> for Path<'a> {
    fn from_value(value: Value<'a>) -> Option<Self> {
        Some(value.cast()?)
    }
}

impl Path<'static> {
    /**
    Create a path from a raw value.
    */
    pub const fn new(path: &'static str) -> Self {
        Path::new_str(Str::new(path))
    }
}

impl<'a> Path<'a> {
    /**
    Create a path from a raw borrowed value.

    The [`Path::new`] method should be preferred where possible.
    */
    pub const fn new_ref(path: &'a str) -> Self {
        Self::new_str(Str::new_ref(path))
    }

    /**
    Create a path from a raw [`Str`] value.
    */
    pub const fn new_str(path: Str<'a>) -> Self {
        Path(path)
    }

    /**
    Get a path, borrowing data from this one.
    */
    pub fn by_ref<'b>(&'b self) -> Path<'b> {
        Path(self.0.by_ref())
    }

    /**
    Iterate over the segments of the path.

    Each segment is the identifier between `::` in the path value.
    */
    pub fn segments(&self) -> Segments {
        Segments {
            inner: self.0.get().split("::"),
        }
    }

    /**
    The root of the path.

    If the path consists of a single segment then this method will return it.
    */
    pub fn root(&self) -> Str {
        self.segments().next().unwrap_or_else(|| self.0.by_ref())
    }

    /**
    The last child segment of the path.

    If the path consists of a single segment then this method will return it.
    */
    pub fn last_child(&self) -> Str {
        self.segments().last().unwrap_or_else(|| self.0.by_ref())
    }

    /**
    Whether this path is a child of `other`.

    The path _a_ is a child of the path _b_ if _b_ is a prefix of _a_ up to a path segment. The path `a::b` is a child of `a`. The path `c::a::b` is not a child of `a`. The path `aa::b` is not a child of `a`.

    This method is reflexive. A path is considered a child of itself.
    */
    pub fn is_child_of<'b>(&self, other: &Path<'b>) -> bool {
        let child = self.0.get();
        let parent = other.0.get();

        if child.len() >= parent.len() && child.is_char_boundary(parent.len()) {
            let (child_prefix, child_suffix) = child.split_at(parent.len());

            child_prefix == parent && (child_suffix.is_empty() || child_suffix.starts_with("::"))
        } else {
            false
        }
    }
}

/**
The result of [`Path::segments`].

This type is an iterator over the `::` separated fragments in a [`Path`].
*/
pub struct Segments<'a> {
    inner: str::Split<'a, &'static str>,
}

impl<'a> Iterator for Segments<'a> {
    type Item = Str<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(Str::new_ref)
    }
}

impl<'a> Eq for Path<'a> {}

impl<'a, 'b> PartialEq<Path<'b>> for Path<'a> {
    fn eq(&self, other: &Path<'b>) -> bool {
        self.0 == other.0
    }
}

impl<'a> Hash for Path<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state)
    }
}

impl<'a, 'b> PartialEq<Str<'b>> for Path<'a> {
    fn eq(&self, other: &Str<'b>) -> bool {
        self.0 == *other
    }
}

impl<'a, 'b> PartialEq<Path<'b>> for Str<'a> {
    fn eq(&self, other: &Path<'b>) -> bool {
        *self == other.0
    }
}

impl<'a> PartialEq<str> for Path<'a> {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl<'a> PartialEq<Path<'a>> for str {
    fn eq(&self, other: &Path<'a>) -> bool {
        self == other.0
    }
}

impl<'a, 'b> PartialEq<&'b str> for Path<'a> {
    fn eq(&self, other: &&'b str) -> bool {
        self.0 == *other
    }
}

impl<'a> PartialOrd for Path<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl<'a> Ord for Path<'a> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl<'a> fmt::Debug for Path<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl<'a> fmt::Display for Path<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

#[cfg(feature = "sval")]
impl<'a> sval::Value for Path<'a> {
    fn stream<'sval, S: sval::Stream<'sval> + ?Sized>(&'sval self, stream: &mut S) -> sval::Result {
        use sval_ref::ValueRef as _;

        self.0.stream_ref(stream)
    }
}

#[cfg(feature = "sval")]
impl<'a> sval_ref::ValueRef<'a> for Path<'a> {
    fn stream_ref<S: sval::Stream<'a> + ?Sized>(&self, stream: &mut S) -> sval::Result {
        self.0.stream_ref(stream)
    }
}

#[cfg(feature = "serde")]
impl<'a> serde::Serialize for Path<'a> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

#[cfg(feature = "alloc")]
mod alloc_support {
    use alloc::{borrow::Cow, boxed::Box};

    use super::*;

    impl Path<'static> {
        /**
        Create a path from an owned raw value.
        */
        pub fn new_owned(path: impl Into<Box<str>>) -> Self {
            Path(Str::new_owned(path))
        }
    }

    impl<'a> Path<'a> {
        /**
        Create a path from a potentially owned raw value.

        If the value is `Cow::Borrowed` then this method will defer to [`Path::new_ref`]. If the value is `Cow::Owned` then this method will defer to [`Path::new_owned`].
        */
        pub fn new_cow_ref(path: Cow<'a, str>) -> Self {
            Path(Str::new_cow_ref(path))
        }

        /**
        Get a new path, taking an owned copy of the data in this one.
        */
        pub fn to_owned(&self) -> Path<'static> {
            Path(self.0.to_owned())
        }

        /**
        Get the underlying value as a potentially owned string.

        If the string contains a contiguous `'static` value then this method will return `Cow::Borrowed`. Otherwise it will return `Cow::Owned`.
        */
        pub fn to_cow(&self) -> Cow<'static, str> {
            self.0.to_cow()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_child_of() {
        let a = Path::new("a");
        let aa = Path::new("aa");
        let b = Path::new("b");
        let a_b = Path::new("a::b");

        assert!(!aa.is_child_of(&a));
        assert!(!b.is_child_of(&a));
        assert!(!a.is_child_of(&a_b));

        assert!(a.is_child_of(&a));
        assert!(a_b.is_child_of(&a));
    }
}
