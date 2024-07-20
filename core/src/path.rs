/*!
The [`Path`] type.

A path is a hierarchical identifier with fragments separated by `::`. Simple Rust paths like `a`, and `a::b::c` are valid [`Path`]s. Complex Rust paths like `a::{b, c}`, and `a::*` are not valid [`Path`]s.

Paths are used to represent the module on [`crate::event::Event`]s.
*/

use core::{
    cmp::Ordering,
    fmt,
    hash::{Hash, Hasher},
    str,
};

use unicode_ident::{is_xid_continue, is_xid_start};

use crate::{
    str::Str,
    value::{FromValue, ToValue, Value},
};

/**
A hierarchical identifier, such as `a::b::c`.

Paths have some logic for determining whether one path is a child of another but don't handle relative/absolute paths or globs.

Paths are validated when they're used rather than when they're constructed. This is to keep conversions between paths and primitive types simple so data is less likely to disappear, even when it's not valid. The [`Path::validate`] method can be used to enforce the validity of a path upfront, so it isn't checked again when calling [`Path::segments`] or [`Path::is_child_of`].
*/
#[derive(Clone)]
pub struct Path<'a>(Str<'a>, bool);

impl<'a> From<&'a str> for Path<'a> {
    fn from(value: &'a str) -> Self {
        Path::new_ref(value)
    }
}

impl<'a> From<Str<'a>> for Path<'a> {
    fn from(value: Str<'a>) -> Self {
        Path::new_str(value)
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

    /**
    Create a path from a raw value without checking its validity.

    This method is not unsafe. There are no memory safety properties tied to the validity of paths.
    */
    pub const fn new_unchecked(path: &'static str) -> Self {
        Path::new_str_unchecked(Str::new(path))
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
    Create a path from a raw borrowed value without checking its validity.

    The [`Path::new_unchecked`] method should be preferred where possible.

    This method is not unsafe. There are no memory safety properties tied to the validity of paths.
    */
    pub const fn new_ref_unchecked(path: &'a str) -> Self {
        Self::new_str_unchecked(Str::new_ref(path))
    }

    /**
    Create a path from a raw [`Str`] value.
    */
    pub const fn new_str(path: Str<'a>) -> Self {
        Path(path, false)
    }

    /**
    Create a path from a raw [`Str`] value without checking its validity.

    This method is not unsafe. There are no memory safety properties tied to the validity of paths.
    */
    pub const fn new_str_unchecked(path: Str<'a>) -> Self {
        Path(path, true)
    }

    /**
    Get a path, borrowing data from this one.
    */
    pub fn by_ref<'b>(&'b self) -> Path<'b> {
        Path(self.0.by_ref(), self.1)
    }

    /**
    Check whether a path is valid.

    After this check is performed, [`Path::segments`] and [`Path::is_child_of`]
    won't need to re-validate this path.
    */
    pub fn validate(self) -> Result<Self, InvalidPathError> {
        if self.is_valid() {
            Ok(Path(self.0, true))
        } else {
            Err(InvalidPathError {})
        }
    }

    /**
    Whether the given path is valid.

    A path is valid if all of the following conditions hold:

    1. The path is non-empty.
    2. The path starts with an identifier.
    3. The path ends with an identifier.
    4. Identifiers are separated by `::`.

    Paths constructed from Rust's [`module_path!`] macro are guaranteed to be valid.
    */
    pub fn is_valid(&self) -> bool {
        // The path is pre-validated
        if self.1 {
            return true;
        }

        let path = self.0.get();

        // Empty paths are not valid
        if path.len() == 0 {
            return false;
        }

        // Paths that start with `:` are not valid
        // We don't need to check whether the path
        // ends with `:` because that's checked below
        if path.starts_with(':') {
            return false;
        }

        let mut separators = 0;

        for c in path.chars() {
            match c {
                // The start of a `::` separator
                ':' if separators == 0 => {
                    separators = 1;
                }
                // The end of a `::` separator
                ':' if separators == 1 => {
                    separators = 2;
                }
                // The start of an identifier
                c if separators % 2 == 0 && is_xid_start(c) => {
                    separators = 0;
                }
                // The middle of an identifier
                c if is_xid_continue(c) => (),
                // An invalid character
                _ => return false,
            }
        }

        // If we ended on a separator (complete or incomplete)
        // then the path is not valid
        if separators != 0 {
            return false;
        }

        true
    }

    /**
    Iterate over the segments of the path.

    The behavior of invalid paths is undefined.
    */
    pub fn segments(&self) -> Result<Segments, InvalidPathError> {
        if !self.is_valid() {
            return Err(InvalidPathError {});
        }

        Ok(Segments {
            inner: match self.0.get_static() {
                Some(inner) => SegmentsInner::Static(inner.split("::")),
                None => SegmentsInner::Borrowed(self.0.get().split("::")),
            },
        })
    }

    /**
    Whether this path is a child of `other`.

    The path _a_ is a child of the path _b_ if _b_ is a prefix of _a_ up to a path segment. The path `a::b` is a child of `a`. The path `c::a::b` is not a child of `a`. The path `aa::b` is not a child of `a`.

    This method is reflexive. A path is considered a child of itself.

    The behavior of invalid paths is undefined.
    */
    pub fn is_child_of<'b>(&self, other: &Path<'b>) -> Result<bool, InvalidPathError> {
        if !self.is_valid() || !other.is_valid() {
            return Err(InvalidPathError {});
        }

        let child = self.0.get();
        let parent = other.0.get();

        let is_child = if child.is_char_boundary(parent.len()) {
            let (child_prefix, child_suffix) = child.split_at(parent.len());

            child_prefix == parent && (child_suffix.is_empty() || child_suffix.starts_with("::"))
        } else {
            false
        };

        Ok(is_child)
    }
}

/**
An error attempting to use an invalid [`Path`].
*/
#[derive(Debug)]
pub struct InvalidPathError {}

impl fmt::Display for InvalidPathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the path is not valid")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for InvalidPathError {}

/**
The result of [`Path::segments`].

This type is an iterator over the `::` separated fragments in a [`Path`].
*/
pub struct Segments<'a> {
    inner: SegmentsInner<'a>,
}

enum SegmentsInner<'a> {
    Borrowed(str::Split<'a, &'static str>),
    Static(str::Split<'static, &'static str>),
}

impl<'a> Iterator for Segments<'a> {
    type Item = Str<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.inner {
            SegmentsInner::Borrowed(ref mut inner) => inner.next().map(Str::new_ref),
            SegmentsInner::Static(ref mut inner) => inner.next().map(Str::new),
        }
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
    use super::*;
    use alloc::{borrow::Cow, boxed::Box, string::String};

    impl Path<'static> {
        /**
        Create a path from an owned raw value.
        */
        pub fn new_owned(path: impl Into<Box<str>>) -> Self {
            Path(Str::new_owned(path), false)
        }

        /**
        Create a path from an owned raw value without checking its validity.

        This method is not unsafe. There are no memory safety properties tied to the validity of paths.
        */
        pub fn new_owned_unchecked(path: impl Into<Box<str>>) -> Self {
            Path(Str::new_owned(path), true)
        }
    }

    impl<'a> Path<'a> {
        /**
        Create a path from a potentially owned raw value.

        If the value is `Cow::Borrowed` then this method will defer to [`Path::new_ref`]. If the value is `Cow::Owned` then this method will defer to [`Path::new_owned`].
        */
        pub fn new_cow_ref(path: Cow<'a, str>) -> Self {
            Path(Str::new_cow_ref(path), false)
        }

        /**
        Create a path from a potentially owned raw value without checking its validity.

        If the value is `Cow::Borrowed` then this method will defer to [`Path::new_ref_unchecked`]. If the value is `Cow::Owned` then this method will defer to [`Path::new_owned_unchecked`].

        This method is not unsafe. There are no memory safety properties tied to the validity of paths.
        */
        pub fn new_cow_ref_unchecked(path: Cow<'a, str>) -> Self {
            Path(Str::new_cow_ref(path), true)
        }

        /**
        Get a new path, taking an owned copy of the data in this one.
        */
        pub fn to_owned(&self) -> Path<'static> {
            Path(self.0.to_owned(), self.1)
        }

        /**
        Get the underlying value as a potentially owned string.

        If the string contains a contiguous `'static` value then this method will return `Cow::Borrowed`. Otherwise it will return `Cow::Owned`.
        */
        pub fn to_cow(&self) -> Cow<'static, str> {
            self.0.to_cow()
        }
    }

    impl From<String> for Path<'static> {
        fn from(value: String) -> Self {
            Path::new_owned(value)
        }
    }

    impl From<Box<str>> for Path<'static> {
        fn from(value: Box<str>) -> Self {
            Path::new_owned(value)
        }
    }

    impl<'k> From<&'k String> for Path<'k> {
        fn from(value: &'k String) -> Self {
            Path::new_ref(value)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segments() {
        for (case, segments, root, last_child) in [
            ("a", vec!["a"], "a", "a"),
            ("a::b", vec!["a", "b"], "a", "b"),
        ] {
            let path = Path::new(case);

            assert_eq!(
                segments,
                path.segments()
                    .unwrap()
                    .map(|segment| segment.get_static().unwrap())
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                root,
                path.segments()
                    .unwrap()
                    .next()
                    .unwrap()
                    .get_static()
                    .unwrap()
            );
            assert_eq!(
                last_child,
                path.segments()
                    .unwrap()
                    .last()
                    .unwrap()
                    .get_static()
                    .unwrap()
            );
        }
    }

    #[test]
    fn is_child_of() {
        let a = Path::new("a");
        let aa = Path::new("aa");
        let b = Path::new("b");
        let a_b = Path::new("a::b");

        assert!(!aa.is_child_of(&a).unwrap());
        assert!(!b.is_child_of(&a).unwrap());
        assert!(!a.is_child_of(&a_b).unwrap());

        assert!(a.is_child_of(&a).unwrap());
        assert!(a_b.is_child_of(&a).unwrap());
    }

    #[test]
    fn is_valid() {
        for (case, is_valid) in [
            ("a", true),
            ("a::b", true),
            ("", false),
            ("::", false),
            ("::a", false),
            ("a::", false),
            ("a:b", false),
            ("a::::b", false),
            ("a::{b, c}", false),
            ("a::*", false),
        ] {
            assert_eq!(Path::new(case).is_valid(), is_valid);
        }
    }
}
