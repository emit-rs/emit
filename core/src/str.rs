/*!
The [`Str`] type.

This module implements a string type that combines `Cow<'static, str>` with `Cow<'a, str>`. A [`Str`] can hold borrowed, static, owned, or shared data. Internally, it's more efficient than a [`std::borrow::Cow`] to access because it doesn't need to hop through enum variants.

Values can be converted into [`Str`]s either directly using methods like [`Str::new`], or generically through the [`ToStr`] trait.

[`Str`]s are used in place of `str` or `String` as keys in [`crate::props::Props`] and fragments of [`crate::template::Template`]s.
*/

use core::{borrow::Borrow, fmt, hash, marker::PhantomData};

#[cfg(feature = "alloc")]
use alloc::{boxed::Box, sync::Arc};

/**
A string value.

The [`Str::get`] method can be used to operate on the value as if it's a standard [`str`]. Equality, ordering, and hashing all defer to the [`str`] representation.

The value may internally be any one of:

- `&'k str`.
- `&'static str`.
- `Box<str>`.
- `Arc<str>`.
*/
pub struct Str<'k> {
    // This type is an optimized `Cow<str>`
    // It avoids the cost of matching the variant to get the inner value
    value: *const str,
    owner: StrOwner,
    _marker: PhantomData<&'k str>,
}

#[cfg_attr(not(feature = "alloc"), derive(Clone, Copy))]
enum StrOwner {
    None,
    Static(&'static str),
    #[cfg(feature = "alloc")]
    Box(*mut str),
    #[cfg(feature = "alloc")]
    Shared(Arc<str>),
}

impl<'k> fmt::Debug for Str<'k> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.get(), f)
    }
}

impl<'k> fmt::Display for Str<'k> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.get(), f)
    }
}

unsafe impl<'k> Send for Str<'k> {}
unsafe impl<'k> Sync for Str<'k> {}

impl<'k> Clone for Str<'k> {
    fn clone(&self) -> Self {
        #[cfg(feature = "alloc")]
        {
            match self.owner {
                StrOwner::Box(_) => Str::new_owned(unsafe { &*self.value }),
                StrOwner::Shared(ref value) => Str::new_shared(value.clone()),
                StrOwner::Static(owner) => Str {
                    value: self.value,
                    owner: StrOwner::Static(owner),
                    _marker: PhantomData,
                },
                StrOwner::None => Str {
                    value: self.value,
                    owner: StrOwner::None,
                    _marker: PhantomData,
                },
            }
        }
        #[cfg(not(feature = "alloc"))]
        {
            Str {
                value: self.value,
                owner: self.owner,
                _marker: PhantomData,
            }
        }
    }
}

impl<'k> Drop for Str<'k> {
    fn drop(&mut self) {
        #[cfg(feature = "alloc")]
        {
            match self.owner {
                StrOwner::Box(boxed) => {
                    drop(unsafe { Box::from_raw(boxed) });
                }
                // Other cases handled normally
                _ => (),
            }
        }
    }
}

impl Str<'static> {
    /**
    Create a new string from a value borrowed for `'static`.
    */
    pub const fn new(k: &'static str) -> Self {
        Str {
            value: k as *const str,
            owner: StrOwner::Static(k),
            _marker: PhantomData,
        }
    }
}

impl<'k> Str<'k> {
    /**
    Create a new string from a value borrowed for `'k`.

    The [`Str::new`] method should be preferred where possible.
    */
    pub const fn new_ref(k: &'k str) -> Str<'k> {
        Str {
            value: k as *const str,
            owner: StrOwner::None,
            _marker: PhantomData,
        }
    }

    /**
    Get a new string, borrowing data from this one.
    */
    pub const fn by_ref<'b>(&'b self) -> Str<'b> {
        Str {
            value: self.value,
            owner: match self.owner {
                StrOwner::Static(owner) => StrOwner::Static(owner),
                _ => StrOwner::None,
            },
            _marker: PhantomData,
        }
    }

    /**
    Get a reference to the underlying value.
    */
    pub const fn get(&self) -> &str {
        // NOTE: It's important here that the lifetime returned is not `'k`
        // If it was it would be possible to return a `&'static str` from
        // an owned value
        // SAFETY: `self.value` is guaranteed to outlive the borrow of `self`
        unsafe { &(*self.value) }
    }

    /**
    Try get a reference to the underlying static value.

    If the string was created from [`Str::new`] and contains a `'static` value then this method will return `Some`. Otherwise this method will return `None`.
    */
    pub const fn get_static(&self) -> Option<&'static str> {
        if let StrOwner::Static(owner) = self.owner {
            Some(owner)
        } else {
            None
        }
    }
}

impl<'a> hash::Hash for Str<'a> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.get().hash(state)
    }
}

impl<'a, 'b> PartialEq<Str<'b>> for Str<'a> {
    fn eq(&self, other: &Str<'b>) -> bool {
        self.get() == other.get()
    }
}

impl<'a> Eq for Str<'a> {}

impl<'a> PartialEq<str> for Str<'a> {
    fn eq(&self, other: &str) -> bool {
        self.get() == other
    }
}

impl<'a> PartialEq<Str<'a>> for str {
    fn eq(&self, other: &Str<'a>) -> bool {
        self == other.get()
    }
}

impl<'a, 'b> PartialEq<&'b str> for Str<'a> {
    fn eq(&self, other: &&'b str) -> bool {
        self.get() == *other
    }
}

impl<'a, 'b> PartialEq<Str<'b>> for &'a str {
    fn eq(&self, other: &Str<'b>) -> bool {
        *self == other.get()
    }
}

impl<'a, 'b> PartialOrd<Str<'b>> for Str<'a> {
    fn partial_cmp(&self, other: &Str<'b>) -> Option<core::cmp::Ordering> {
        self.get().partial_cmp(other.get())
    }
}

impl<'a> Ord for Str<'a> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.get().cmp(other.get())
    }
}

impl<'k> Borrow<str> for Str<'k> {
    fn borrow(&self) -> &str {
        self.get()
    }
}

impl<'k> AsRef<str> for Str<'k> {
    fn as_ref(&self) -> &str {
        self.get()
    }
}

impl<'a> From<&'a str> for Str<'a> {
    fn from(value: &'a str) -> Self {
        Str::new_ref(value)
    }
}

impl<'a, 'b> From<&'a Str<'b>> for Str<'a> {
    fn from(value: &'a Str<'b>) -> Self {
        value.by_ref()
    }
}

impl<'k> ToValue for Str<'k> {
    fn to_value(&self) -> Value<'_> {
        self.get().to_value()
    }
}

impl<'k> FromValue<'k> for Str<'k> {
    fn from_value<'a>(value: Value<'k>) -> Option<Self> {
        #[cfg(feature = "alloc")]
        {
            value.to_cow_str().map(Str::new_cow_ref)
        }
        #[cfg(not(feature = "alloc"))]
        {
            value.to_borrowed_str().map(Str::new_ref)
        }
    }
}

/**
Convert a reference to a [`Str`].
*/
pub trait ToStr {
    /**
    Perform the conversion.
    */
    fn to_str(&self) -> Str;
}

impl<'a, T: ToStr + ?Sized> ToStr for &'a T {
    fn to_str(&self) -> Str<'_> {
        (**self).to_str()
    }
}

impl<'k> ToStr for Str<'k> {
    fn to_str(&self) -> Str<'_> {
        self.by_ref()
    }
}

impl ToStr for str {
    fn to_str(&self) -> Str<'_> {
        Str::new_ref(self)
    }
}

#[cfg(feature = "sval")]
impl<'k> sval::Value for Str<'k> {
    fn stream<'sval, S: sval::Stream<'sval> + ?Sized>(&'sval self, stream: &mut S) -> sval::Result {
        use sval_ref::ValueRef as _;

        self.stream_ref(stream)
    }
}

#[cfg(feature = "sval")]
impl<'k> sval_ref::ValueRef<'k> for Str<'k> {
    fn stream_ref<S: sval::Stream<'k> + ?Sized>(&self, stream: &mut S) -> sval::Result {
        if let Some(k) = self.get_static() {
            stream.value(k)
        } else {
            stream.value_computed(self.get())
        }
    }
}

#[cfg(feature = "serde")]
impl<'k> serde::Serialize for Str<'k> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.get().serialize(serializer)
    }
}

#[cfg(feature = "alloc")]
mod alloc_support {
    use super::*;
    use alloc::{
        borrow::{Cow, ToOwned},
        string::String,
    };
    use core::mem;

    impl Str<'static> {
        /**
        Create a string from an owned value.

        Cloning the string will involve cloning the value.
        */
        pub fn new_owned(key: impl Into<Box<str>>) -> Self {
            let value = key.into();

            let raw = Box::into_raw(value);

            Str {
                value: raw as *const str,
                owner: StrOwner::Box(raw),
                _marker: PhantomData,
            }
        }

        /**
        Create a string from a shared value.

        Cloning the string will involve cloning the `Arc`, which may be cheaper than cloning the value itself.
        */
        pub fn new_shared(key: impl Into<Arc<str>>) -> Self {
            let value = key.into();

            Str {
                value: &*value as *const str,
                owner: StrOwner::Shared(value),
                _marker: PhantomData,
            }
        }
    }

    impl<'k> Str<'k> {
        /**
        Create a string from a potentially owned value.

        If the value is `Cow::Borrowed` then this method will defer to [`Str::new_ref`]. If the value is `Cow::Owned` then this method will defer to [`Str::new_owned`].
        */
        pub fn new_cow_ref(key: Cow<'k, str>) -> Self {
            match key {
                Cow::Borrowed(key) => Str::new_ref(key),
                Cow::Owned(key) => Str::new_owned(key),
            }
        }

        /**
        Get the underlying value as a potentially owned string.

        If the string contains a `'static` value then this method will return `Cow::Borrowed`. Otherwise it will return `Cow::Owned`.
        */
        pub fn to_cow(&self) -> Cow<'static, str> {
            match self.owner {
                StrOwner::Static(key) => Cow::Borrowed(key),
                _ => Cow::Owned(self.get().to_owned()),
            }
        }

        /**
        Get a new string, taking an owned copy of the data in this one.

        If the string contains a `'static` or `Arc` value then this method is cheap and doesn't involve cloning. In other cases the underlying value will be passed through [`Str::new_owned`].
        */
        pub fn to_owned(&self) -> Str<'static> {
            match self.owner {
                StrOwner::Static(owner) => Str::new(owner),
                StrOwner::Shared(ref owner) => Str::new_shared(owner.clone()),
                _ => Str::new_owned(self.get()),
            }
        }

        /**
        Convert this string into an owned `String`.

        If the underlying value is already an owned string then this method will return it without allocating.
        */
        pub fn into_string(self) -> String {
            match self.owner {
                StrOwner::Box(boxed) => {
                    // Ensure `Drop` doesn't run over this value
                    // and clean up the box we've just moved out of
                    mem::forget(self);

                    unsafe { Box::from_raw(boxed) }.into()
                }
                _ => self.get().to_owned(),
            }
        }

        /**
        Get a new string, taking an owned copy of the data in this one.

        If the string contains a `'static` or `Arc` value then this method is cheap and doesn't involve cloning. In other cases the underlying value will be passed through [`Str::new_shared`].
        */
        pub fn to_shared(&self) -> Str<'static> {
            match self.owner {
                StrOwner::Static(owner) => Str::new(owner),
                StrOwner::Shared(ref owner) => Str::new_shared(owner.clone()),
                _ => Str::new_shared(self.get()),
            }
        }
    }

    impl ToStr for String {
        fn to_str(&self) -> Str<'_> {
            Str::new_ref(self)
        }
    }

    impl ToStr for Box<str> {
        fn to_str(&self) -> Str<'_> {
            Str::new_ref(self)
        }
    }

    impl ToStr for Arc<str> {
        fn to_str(&self) -> Str<'_> {
            Str::new_shared(self.clone())
        }
    }

    impl From<String> for Str<'static> {
        fn from(value: String) -> Self {
            Str::new_owned(value)
        }
    }

    impl From<Box<str>> for Str<'static> {
        fn from(value: Box<str>) -> Self {
            Str::new_owned(value)
        }
    }

    impl From<Arc<str>> for Str<'static> {
        fn from(value: Arc<str>) -> Self {
            Str::new_shared(value)
        }
    }

    impl<'k> From<&'k String> for Str<'k> {
        fn from(value: &'k String) -> Self {
            Str::new_ref(value)
        }
    }

    impl<'k> From<Str<'k>> for String {
        fn from(value: Str<'k>) -> String {
            value.into_string()
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn str_size() {
            assert_eq!(40, mem::size_of::<Str>());
        }

        #[test]
        fn to_owned() {
            for case in [
                Str::new("string"),
                Str::new_ref("string"),
                Str::new_owned("string"),
                Str::new_shared("string"),
            ] {
                assert_eq!(case, case.to_owned());
            }
        }

        #[test]
        fn to_cow() {
            for (case, expected) in [
                (Str::new("string"), Cow::Borrowed("string")),
                (Str::new_ref("string"), Cow::Owned("string".to_owned())),
                (Str::new_owned("string"), Cow::Owned("string".to_owned())),
                (Str::new_shared("string"), Cow::Owned("string".to_owned())),
            ] {
                assert_eq!(expected, case.to_cow());
            }
        }

        #[test]
        fn to_shared() {
            for case in [
                Str::new("string"),
                Str::new_ref("string"),
                Str::new_owned("string"),
                Str::new_shared("string"),
            ] {
                assert_eq!(case, case.to_shared());
            }
        }

        #[test]
        fn into_string() {
            for case in [
                Str::new("string"),
                Str::new_ref("string"),
                Str::new_owned("string"),
                Str::new_shared("string"),
            ] {
                assert_eq!(case.get().to_owned(), case.into_string());
            }
        }

        #[test]
        fn owned_into_string() {
            let s = Str::new_owned("string");
            let ptr = match s.owner {
                StrOwner::Box(boxed) => boxed as *const u8,
                _ => panic!("expected an owned string"),
            };

            let owned = s.into_string();

            assert_eq!(ptr, owned.as_ptr());
        }

        #[test]
        fn shared_str_clone() {
            let sa = Str::new_shared("string");
            let a = match sa.owner {
                StrOwner::Shared(ref owner) => owner.clone(),
                _ => panic!("expected a shared string"),
            };

            let sb = sa.clone();
            let b = match sb.owner {
                StrOwner::Shared(ref owner) => owner.clone(),
                _ => panic!("expected a shared string"),
            };

            assert!(Arc::ptr_eq(&a, &b));

            drop(sa);
            drop(a);

            assert_eq!("string", sb);
        }
    }
}

use crate::value::{FromValue, ToValue, Value};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get() {
        for (case, as_ref, as_static) in [
            (Str::new_ref("string"), "string", None::<&'static str>),
            (Str::new("string"), "string", Some("string")),
        ] {
            assert_eq!(as_ref, case.get());
            assert_eq!(as_static, case.get_static());
        }
    }

    #[test]
    fn clone() {
        for case in [
            Str::new("string"),
            Str::new_ref("string"),
            #[cfg(feature = "alloc")]
            Str::new_owned("string"),
            #[cfg(feature = "alloc")]
            Str::new_shared("string"),
        ] {
            assert_eq!(case.get(), case.clone().get());
        }
    }

    #[test]
    fn by_ref() {
        for case in [
            Str::new("string"),
            Str::new_ref("string"),
            #[cfg(feature = "alloc")]
            Str::new_owned("string"),
            #[cfg(feature = "alloc")]
            Str::new_shared("string"),
        ] {
            assert_eq!(case, case.by_ref());
        }
    }

    #[test]
    fn to_from_value() {
        for case in [
            Str::new("string"),
            Str::new_ref("string"),
            #[cfg(feature = "alloc")]
            Str::new_owned("string"),
            #[cfg(feature = "alloc")]
            Str::new_shared("string"),
        ] {
            let value = case.to_value();

            assert_eq!(case, value.cast::<Str>().unwrap());
        }

        let value = Value::from("string");

        assert_eq!(Str::new("string"), value.cast::<Str>().unwrap());
    }
}
