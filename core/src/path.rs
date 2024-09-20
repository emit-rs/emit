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
*/
#[derive(Clone)]
pub struct Path<'a>(Str<'a>);

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
        Path::new_str(value.cast()?).ok()
    }
}

impl Path<'static> {
    /**
    Create a path from a raw value.

    This method will fail if the path is malformed. A valid path consists of one or more identifiers separated by `::`s. like `a::b::c`. See the [`is_valid_path`] function for more details.
    */
    pub fn new(path: &'static str) -> Result<Self, InvalidPathError> {
        Path::new_str(Str::new(path))
    }

    /**
    Create a path from a raw value without checking its validity.

    This method is not unsafe. There are no memory safety properties tied to the validity of paths. Code that uses path segments may panic or produce unexpected results if given an invalid path.
    */
    pub const fn new_unchecked(path: &'static str) -> Self {
        Path::new_str_unchecked(Str::new(path))
    }
}

impl<'a> Path<'a> {
    /**
    Create a path from a raw borrowed value.

    The [`Path::new`] method should be preferred where possible.

    This method will fail if the path is malformed. A valid path consists of one or more identifiers separated by `::`s. like `a::b::c`. See the [`is_valid_path`] function for more details.
    */
    pub fn new_ref(path: &'a str) -> Result<Self, InvalidPathError> {
        Self::new_str(Str::new_ref(path))
    }

    /**
    Create a path from a raw borrowed value without checking its validity.

    The [`Path::new_unchecked`] method should be preferred where possible.

    This method is not unsafe. There are no memory safety properties tied to the validity of paths. Code that uses path segments may panic or produce unexpected results if given an invalid path.
    */
    pub const fn new_ref_unchecked(path: &'a str) -> Self {
        Self::new_str_unchecked(Str::new_ref(path))
    }

    /**
    Create a path from a raw [`Str`] value.

    This method will fail if the path is malformed. A valid path consists of one or more identifiers separated by `::`s. like `a::b::c`. See the [`is_valid_path`] function for more details.
    */
    pub fn new_str(path: Str<'a>) -> Result<Self, InvalidPathError> {
        if is_valid_path(path.get()) {
            Ok(Path(path))
        } else {
            Err(InvalidPathError {})
        }
    }

    /**
    Create a path from a raw [`Str`] value without checking its validity.

    This method is not unsafe. There are no memory safety properties tied to the validity of paths. Code that uses path segments may panic or produce unexpected results if given an invalid path.
    */
    pub const fn new_str_unchecked(path: Str<'a>) -> Self {
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

    The behavior of this method on invalid paths is undefined.
    */
    pub fn segments(&self) -> Segments {
        Segments {
            inner: match self.0.get_static() {
                Some(inner) => SegmentsInner::Static(inner.split("::")),
                None => SegmentsInner::Borrowed(self.0.get().split("::")),
            },
        }
    }

    /**
    Whether this path is a child of `other`.

    The path _a_ is a child of the path _b_ if _b_ is a prefix of _a_ up to a path segment. The path `a::b` is a child of `a`. The path `c::a::b` is not a child of `a`. The path `aa::b` is not a child of `a`.

    This method is reflexive. A path is considered a child of itself.

    The behavior of this method on invalid paths is undefined.
    */
    pub fn is_child_of<'b>(&self, other: &Path<'b>) -> bool {
        let child = self.0.get();
        let parent = other.0.get();

        if child.is_char_boundary(parent.len()) {
            let (child_prefix, child_suffix) = child.split_at(parent.len());

            child_prefix == parent && (child_suffix.is_empty() || child_suffix.starts_with("::"))
        } else {
            false
        }
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
pub fn is_valid_path(path: &str) -> bool {
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

impl<'a, 'b, 'c> PartialEq<&'c Path<'b>> for Path<'a> {
    fn eq(&self, other: &&'c Path<'b>) -> bool {
        self.0 == other.0
    }
}

impl<'a, 'b, 'c> PartialEq<Path<'c>> for &'b Path<'a> {
    fn eq(&self, other: &Path<'c>) -> bool {
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
    use alloc::{borrow::Cow, boxed::Box};

    impl Path<'static> {
        /**
        Create a path from an owned raw value.

        This method will fail if the path is malformed. A valid path consists of one or more identifiers separated by `::`s. like `a::b::c`. See the [`is_valid_path`] function for more details.
        */
        pub fn new_owned(path: impl Into<Box<str>>) -> Result<Self, InvalidPathError> {
            Path::new_str(Str::new_owned(path))
        }

        /**
        Create a path from an owned raw value without checking its validity.

        This method is not unsafe. There are no memory safety properties tied to the validity of paths. Code that uses path segments may panic or produce unexpected results if given an invalid path.
        */
        pub fn new_owned_unchecked(path: impl Into<Box<str>>) -> Self {
            Path::new_str_unchecked(Str::new_owned(path))
        }
    }

    impl<'a> Path<'a> {
        /**
        Create a path from a potentially owned raw value.

        If the value is `Cow::Borrowed` then this method will defer to [`Path::new_ref`]. If the value is `Cow::Owned` then this method will defer to [`Path::new_owned`].

        This method will fail if the path is malformed. A valid path consists of one or more identifiers separated by `::`s. like `a::b::c`. See the [`is_valid_path`] function for more details.
        */
        pub fn new_cow_ref(path: Cow<'a, str>) -> Result<Self, InvalidPathError> {
            Path::new_str(Str::new_cow_ref(path))
        }

        /**
        Create a path from a potentially owned raw value without checking its validity.

        If the value is `Cow::Borrowed` then this method will defer to [`Path::new_ref_unchecked`]. If the value is `Cow::Owned` then this method will defer to [`Path::new_owned_unchecked`].

        This method is not unsafe. There are no memory safety properties tied to the validity of paths. Code that uses path segments may panic or produce unexpected results if given an invalid path.
        */
        pub fn new_cow_ref_unchecked(path: Cow<'a, str>) -> Self {
            Path::new_str_unchecked(Str::new_cow_ref(path))
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

        /**
        Append `other` to `self` with a separator in-between.
        */
        pub fn append<'b>(self, other: impl Into<Path<'b>>) -> Self {
            let mut base = self.0.into_string();

            base.push_str("::");
            base.push_str(other.into().0.get());
            Path::new_owned_unchecked(base)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn to_owned() {
            let path = Path::new_ref_unchecked("module");

            assert_eq!(path, path.to_owned());
        }

        #[test]
        fn to_cow() {
            for (path, expected) in [
                (Path::new_unchecked("module"), Cow::Borrowed("module")),
                (
                    Path::new_ref_unchecked("module"),
                    Cow::Owned("module".to_owned()),
                ),
            ] {
                assert_eq!(expected, path.to_cow());
            }
        }

        #[test]
        fn append() {
            for (a, b, expected) in [
                ("a", "b", "a::b"),
                ("a::b", "c", "a::b::c"),
                ("a", "b::c", "a::b::c"),
            ] {
                assert_eq!(
                    expected,
                    Path::new_unchecked(a)
                        .append(Path::new_unchecked(&b))
                        .0
                        .get(),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn by_ref() {
        let path = Path::new_unchecked("module");

        assert_eq!(path, path.by_ref());
    }

    #[test]
    fn segments() {
        for (case, segments, root, last_child) in [
            ("a", vec!["a"], "a", "a"),
            ("a::b", vec!["a", "b"], "a", "b"),
        ] {
            let path = Path::new(case).unwrap();

            assert_eq!(
                segments,
                path.segments()
                    .map(|segment| segment.get_static().unwrap())
                    .collect::<Vec<_>>()
            );
            assert_eq!(root, path.segments().next().unwrap().get_static().unwrap());
            assert_eq!(
                last_child,
                path.segments().last().unwrap().get_static().unwrap()
            );
        }
    }

    #[test]
    fn is_child_of() {
        let a = Path::new("a").unwrap();
        let aa = Path::new("aa").unwrap();
        let b = Path::new("b").unwrap();
        let a_b = Path::new("a::b").unwrap();

        assert!(!aa.is_child_of(&a));
        assert!(!b.is_child_of(&a));
        assert!(!a.is_child_of(&a_b));

        assert!(a.is_child_of(&a));
        assert!(a_b.is_child_of(&a));
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
            assert_eq!(is_valid_path(case), is_valid);
        }
    }

    #[test]
    fn to_from_value() {
        let path = Path::new_unchecked("module");

        for value in [Value::from_any(&path), Value::from("module")] {
            assert_eq!(path, value.cast::<Path>().unwrap());
        }
    }
}
