/*!
The [`Value`] type.

Values are an anonymous bag of data that can be formatted or serialized. The basic data model of values includes:

- Null.
- Booleans.
- Integers.
- Binary floating point numbers.
- Strings.
- Errors.
- Sequences.

The data model is enhanced through serialization frameworks like `serde` or `sval` to also support:

- Maps.
- Structs.
- Tuples.
- Enums.

Values are captured using a trait they implement, such as [`fmt::Display`], or [`serde::Serialize`]. They can then be serialized using any trait supported by [`Value`]. The choice depends on the needs of the consumer, but they don't need to use the same trait as the producer did.
*/

use core::{fmt, str::FromStr};

/**
An anonymous captured value that can be serialized or formatted.
*/
#[derive(Clone)]
pub struct Value<'v>(value_bag::ValueBag<'v>);

impl<'v> Value<'v> {
    /**
    Capture a displayable value.
    */
    #[track_caller]
    pub fn capture_display(value: &'v (impl fmt::Display + 'static)) -> Self {
        Value(value_bag::ValueBag::capture_display(value))
    }

    /**
    Get a displayable value.

    This method can be used instead of [`Value::capture_display`] when the value can't satisfy the `'static` bound.
    */
    #[track_caller]
    pub fn from_display(value: &'v impl fmt::Display) -> Self {
        Value(value_bag::ValueBag::from_display(value))
    }

    /**
    Capture a debuggable value.
    */
    #[track_caller]
    pub fn capture_debug(value: &'v (impl fmt::Debug + 'static)) -> Self {
        Value(value_bag::ValueBag::capture_debug(value))
    }

    /**
    Get a debuggable value.

    This method can be used instead of [`Value::capture_debug`] when the value can't satisfy the `'static` bound.
    */
    #[track_caller]
    pub fn from_debug(value: &'v impl fmt::Debug) -> Self {
        Value(value_bag::ValueBag::from_debug(value))
    }

    /**
    Capture a serializable value.
    */
    #[cfg(feature = "serde")]
    #[track_caller]
    pub fn capture_serde(value: &'v (impl serde::Serialize + 'static)) -> Self {
        Value(value_bag::ValueBag::capture_serde1(value))
    }

    /**
    Get a serializable value.

    This method can be used instead of [`Value::capture_serde`] when the value can't satisfy the `'static` bound.
    */
    #[cfg(feature = "serde")]
    #[track_caller]
    pub fn from_serde(value: &'v impl serde::Serialize) -> Self {
        Value(value_bag::ValueBag::from_serde1(value))
    }

    /**
    Capture a serializable value.
    */
    #[cfg(feature = "sval")]
    #[track_caller]
    pub fn capture_sval(value: &'v (impl sval::Value + 'static)) -> Self {
        Value(value_bag::ValueBag::capture_sval2(value))
    }

    /**
    Get a serializable value.

    This method can be used instead of [`Value::capture_sval`] when the value can't satisfy the `'static` bound.
    */
    #[cfg(feature = "sval")]
    #[track_caller]
    pub fn from_sval(value: &'v impl sval::Value) -> Self {
        Value(value_bag::ValueBag::from_sval2(value))
    }

    /**
    Capture an error.
    */
    #[cfg(feature = "std")]
    #[track_caller]
    pub fn capture_error(value: &'v (impl std::error::Error + 'static)) -> Self {
        Value(value_bag::ValueBag::capture_error(value))
    }

    /**
    Capture a convertible type.
    */
    #[track_caller]
    pub fn from_any(value: &'v impl ToValue) -> Self {
        value.to_value()
    }

    /**
    The absence of any meaningful value.
    */
    #[track_caller]
    pub const fn null() -> Self {
        Value(value_bag::ValueBag::empty())
    }

    /**
    Whether the value is null.
    */
    pub fn is_null(&self) -> bool {
        self.0.is_empty()
    }

    /**
    Get a new value, borrowing data from this one.
    */
    pub fn by_ref<'b>(&'b self) -> Value<'b> {
        Value(self.0.by_ref())
    }

    /**
    Attempt to convert this value into an owned instance of `T`.

    This may involve downcasting, serializing, or parsing depending on the implementation of [`FromValue`].
    */
    pub fn cast<'a, T: FromValue<'v>>(self) -> Option<T> {
        T::from_value(self)
    }

    /**
    Attempt to downcast this value into a borrowed instance of `T`.

    This method should be used as a potential optimization, but can't be relied upon to always return `Some`. If any internal buffering happens between owned and borrowed value conversions then the internal captured type will change.
    */
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.0.downcast_ref()
    }

    /**
    Attempt to parse an instance of `T` from this value.

    If the value is an internally captured string then it will be parsed directly. If the value is not a string then it will be formatted into one and then parsed.
    */
    pub fn parse<T: FromStr>(&self) -> Option<T> {
        struct Extract<T>(Option<T>);

        impl<'v, T: FromStr> value_bag::visit::Visit<'v> for Extract<T> {
            fn visit_any(&mut self, value: value_bag::ValueBag) -> Result<(), value_bag::Error> {
                #[cfg(feature = "alloc")]
                {
                    use alloc::string::ToString;

                    self.0 = value.to_string().parse().ok();

                    Ok(())
                }
                #[cfg(not(feature = "alloc"))]
                {
                    let _ = value;

                    Ok(())
                }
            }

            fn visit_str(&mut self, value: &str) -> Result<(), value_bag::Error> {
                self.0 = value.parse().ok();

                Ok(())
            }
        }

        let mut visitor = Extract(None);
        let _ = self.0.visit(&mut visitor);
        visitor.0
    }

    /**
    Try get a borrowed string value.
    */
    pub fn to_borrowed_str(&self) -> Option<&'v str> {
        self.0.to_borrowed_str()
    }

    /**
    Try get a borrowed error value.
    */
    #[cfg(feature = "std")]
    pub fn to_borrowed_error(&self) -> Option<&'v (dyn std::error::Error + 'static)> {
        self.0.to_borrowed_error()
    }

    /**
    Get a lossy binary floating point value.

    If the value is numeric then it will be converted into an `f64` using `as` conversions. If the value is not numeric then this method will attempt to parse an `f64` from it. If the value can't be parsed then [`f64::NAN`] is returned.
    */
    pub fn as_f64(&self) -> f64 {
        let r = self.0.as_f64();

        if r.is_nan() {
            self.parse::<f64>().unwrap_or(f64::NAN)
        } else {
            r
        }
    }
}

impl<'v> fmt::Debug for Value<'v> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl<'v> fmt::Display for Value<'v> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[cfg(feature = "std")]
        {
            /*
            If the value is an error then display it along with its root cause.

            Rust errors are typically a chain of fragments, becoming more specific as they get closer
            to the original failure. We display both the top and the bottom of the chain here so
            the result is more likely to be useful.
            */

            use std::iter;

            if let Some(err) = self.to_borrowed_error() {
                return if let Some(root) =
                    iter::successors(err.source(), |ref err| err.source()).last()
                {
                    write!(f, "{err} ({root})")
                } else {
                    fmt::Display::fmt(err, f)
                };
            }
        }

        fmt::Display::fmt(&self.0, f)
    }
}

#[cfg(feature = "sval")]
impl<'v> sval::Value for Value<'v> {
    fn stream<'sval, S: sval::Stream<'sval> + ?Sized>(&'sval self, stream: &mut S) -> sval::Result {
        self.0.stream(stream)
    }
}

#[cfg(feature = "sval")]
impl<'v> sval_ref::ValueRef<'v> for Value<'v> {
    fn stream_ref<S: sval::Stream<'v> + ?Sized>(&self, stream: &mut S) -> sval::Result {
        self.0.stream_ref(stream)
    }
}

#[cfg(feature = "serde")]
impl<'v> serde::Serialize for Value<'v> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

/**
Convert a reference to a [`Value`].

This trait is the basis for the [`Value::from_any`] method.
*/
pub trait ToValue {
    /**
    Perform the conversion.
    */
    fn to_value(&self) -> Value<'_>;
}

/**
Convert from a [`Value`].

This trait is the basis for the [`Value::cast`] method.
*/
pub trait FromValue<'v> {
    /**
    Perform the conversion.
    */
    fn from_value(value: Value<'v>) -> Option<Self>
    where
        Self: Sized;
}

impl<'a, T: ToValue + ?Sized> ToValue for &'a T {
    fn to_value(&self) -> Value<'_> {
        (**self).to_value()
    }
}

impl<T: ToValue> ToValue for Option<T> {
    fn to_value(&self) -> Value<'_> {
        match self {
            Some(v) => v.to_value(),
            None => Value::null(),
        }
    }
}

impl<'v> ToValue for Value<'v> {
    fn to_value(&self) -> Value<'_> {
        self.by_ref()
    }
}

impl<'v> FromValue<'v> for Value<'v> {
    fn from_value(value: Value<'v>) -> Option<Self> {
        Some(value)
    }
}

macro_rules! impl_primitive {
    ($($t:ty,)*) => {
        $(
            impl ToValue for $t {
                fn to_value(&self) -> Value<'_> {
                    Value(self.into())
                }
            }

            impl<const N: usize> ToValue for [$t; N] {
                fn to_value(&self) -> Value<'_> {
                    Value(self.into())
                }
            }

            impl<'v> FromValue<'v> for $t {
                fn from_value(value: Value<'v>) -> Option<Self> {
                    value.0.try_into().ok()
                }
            }

            impl<'v> From<$t> for Value<'v> {
                fn from(value: $t) -> Self {
                    Value(value.into())
                }
            }

            impl<'v> From<Option<$t>> for Value<'v> {
                fn from(value: Option<$t>) -> Self {
                    Value(value_bag::ValueBag::from_option(value))
                }
            }

            impl<'v, const N: usize> From<&'v [$t; N]> for Value<'v> {
                fn from(value: &'v [$t; N]) -> Self {
                    Value(value_bag::ValueBag::from(value))
                }
            }
        )*
    };
}

macro_rules! impl_ref {
    ($(& $l:lifetime $t:ty,)*) => {
        $(
            impl ToValue for $t {
                fn to_value(&self) -> Value<'_> {
                    Value(self.into())
                }
            }

            impl<$l, const N: usize> ToValue for [&$l $t; N] {
                fn to_value(&self) -> Value<'_> {
                    Value(self.into())
                }
            }

            impl<$l> FromValue<$l> for &$l $t {
                fn from_value(value: Value<$l>) -> Option<Self> {
                    value.0.try_into().ok()
                }
            }

            impl<$l> From<&$l $t> for Value<$l> {
                fn from(value: &$l $t) -> Self {
                    Value(value.into())
                }
            }

            impl<$l> From<Option<&$l $t>> for Value<$l> {
                fn from(value: Option<&$l $t>) -> Self {
                    Value(value_bag::ValueBag::from_option(value))
                }
            }

            impl<$l, 'a, const N: usize> From<&'v [&'a $t; N]> for Value<'v> {
                fn from(value: &$l [&'a $t; N]) -> Self {
                    Value(value_bag::ValueBag::from(value))
                }
            }
        )*
    };
}

impl_primitive!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, f64, bool,);

impl_ref!(&'v str,);

impl ToValue for dyn fmt::Debug {
    fn to_value(&self) -> Value<'_> {
        Value(value_bag::ValueBag::from_dyn_debug(self))
    }
}

impl ToValue for dyn fmt::Display {
    fn to_value(&self) -> Value<'_> {
        Value(value_bag::ValueBag::from_dyn_display(self))
    }
}

#[cfg(feature = "std")]
impl ToValue for dyn std::error::Error + 'static {
    fn to_value(&self) -> Value<'_> {
        Value(value_bag::ValueBag::from_dyn_error(self))
    }
}

#[cfg(feature = "std")]
impl<'v> FromValue<'v> for &'v (dyn std::error::Error + 'static) {
    fn from_value(value: Value<'v>) -> Option<Self> {
        value.to_borrowed_error()
    }
}

#[cfg(feature = "alloc")]
mod alloc_support {
    use super::*;

    use alloc::{borrow::Cow, string::String, vec::Vec};

    impl<'v> Value<'v> {
        /**
        Get a sequence of binary floating points from a captured sequence of values.

        If the value is a sequence then `Some` is returned. Each element will be converted into a `f64` in the same way as [`Value::as_f64`].
        If the value is not a sequence then `None` is returned.

        For more advanced or specific conversion cases, use `serde` or `sval`.
        */
        pub fn to_f64_sequence(&self) -> Option<Vec<f64>> {
            #[derive(Default)]
            struct Seq(Vec<f64>);

            impl Extend<Option<f64>> for Seq {
                fn extend<T: IntoIterator<Item = Option<f64>>>(&mut self, iter: T) {
                    self.0
                        .extend(iter.into_iter().map(|v| v.unwrap_or(f64::NAN)));
                }
            }

            self.0.to_f64_seq::<Seq>().map(|seq| seq.0)
        }
    }

    /**
    An owned [`Value`] that can be cloned and shared.

    Owned values don't expose much API of their own but can be cheaply converted back into a [`Value`] through [`OwnedValue::by_ref`].
    */
    #[derive(Clone)]
    pub struct OwnedValue(value_bag::OwnedValueBag);

    impl<'v> Value<'v> {
        /**
        Get an owned value from this one.
        */
        pub fn to_owned(&self) -> OwnedValue {
            OwnedValue(self.0.to_owned())
        }

        /**
        Get an owned value from this one, internally storing an `Arc` for cheap cloning.
        */
        pub fn to_shared(&self) -> OwnedValue {
            OwnedValue(self.0.to_shared())
        }

        /**
        Try get a string from this value.

        If the value is a captured string then `Some(Cow::Borrowed)` will be returned. If the value is a string that needs to be buffered through a serialization framework then `Some(Cow::Owned)` is returned. In other cases `None` is returned.
        */
        pub fn to_cow_str(&self) -> Option<Cow<'v, str>> {
            self.0.to_str()
        }
    }

    impl OwnedValue {
        /**
        Get a [`Value`], borrowing data from this one.
        */
        pub fn by_ref<'v>(&'v self) -> Value<'v> {
            Value(self.0.by_ref())
        }
    }

    impl fmt::Debug for OwnedValue {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            fmt::Debug::fmt(&self.0, f)
        }
    }

    impl fmt::Display for OwnedValue {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            fmt::Display::fmt(&self.0, f)
        }
    }

    impl<'a> From<&'a OwnedValue> for Value<'a> {
        fn from(value: &'a OwnedValue) -> Self {
            value.by_ref()
        }
    }

    impl ToValue for OwnedValue {
        fn to_value(&self) -> Value<'_> {
            self.by_ref()
        }
    }

    #[cfg(feature = "sval")]
    impl sval::Value for OwnedValue {
        fn stream<'sval, S: sval::Stream<'sval> + ?Sized>(
            &'sval self,
            stream: &mut S,
        ) -> sval::Result {
            self.0.stream(stream)
        }
    }

    #[cfg(feature = "serde")]
    impl serde::Serialize for OwnedValue {
        fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            self.0.serialize(serializer)
        }
    }

    impl ToValue for String {
        fn to_value(&self) -> Value<'_> {
            Value(self.into())
        }
    }

    impl<'v> FromValue<'v> for String {
        fn from_value(value: Value<'v>) -> Option<Self> {
            value.0.try_into().ok()
        }
    }

    impl<'a> From<&'a String> for Value<'a> {
        fn from(value: &'a String) -> Self {
            Value(value.into())
        }
    }

    impl<'a> From<Option<&'a String>> for Value<'a> {
        fn from(value: Option<&'a String>) -> Self {
            Value(value_bag::ValueBag::from_option(value))
        }
    }

    impl<'v> ToValue for Cow<'v, str> {
        fn to_value(&self) -> Value<'_> {
            Value(self.into())
        }
    }

    impl<'v> FromValue<'v> for Cow<'v, str> {
        fn from_value(value: Value<'v>) -> Option<Self> {
            value.0.try_into().ok()
        }
    }

    impl<'a, 'v> From<&'a Cow<'v, str>> for Value<'a> {
        fn from(value: &'a Cow<'v, str>) -> Self {
            Value(value.into())
        }
    }

    impl<'a, 'v> From<Option<&'a Cow<'v, str>>> for Value<'a> {
        fn from(value: Option<&'a Cow<'v, str>>) -> Self {
            Value(value_bag::ValueBag::from_option(value))
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn to_f64_sequence() {
            for (case, expected) in [
                (Value::from(&[0.0, 1.0, 2.0]), Some(vec![0.0, 1.0, 2.0])),
                (Value::from(0.0), None),
                (Value::from(&[0, 1, 2]), Some(vec![0.0, 1.0, 2.0])),
                (Value::from(&[] as &[f64; 0]), Some(vec![])),
                (Value::from(&[true, false]), Some(vec![f64::NAN, f64::NAN])),
            ] {
                let actual = case.to_f64_sequence();

                assert_eq!(expected.is_some(), actual.is_some());

                let (Some(expected), Some(actual)) = (expected, actual) else {
                    continue;
                };

                assert_eq!(expected.len(), actual.len());

                for (expected, actual) in expected.into_iter().zip(actual) {
                    if expected.is_nan() {
                        assert!(actual.is_nan());
                    } else {
                        assert_eq!(expected, actual);
                    }
                }
            }
        }
    }
}

#[cfg(feature = "alloc")]
pub use self::alloc_support::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        for (case, expected) in [
            (Value::from("1"), Some(1)),
            (Value::from("x"), None),
            #[cfg(feature = "alloc")]
            (Value::from_display(&"1"), Some(1)),
            #[cfg(feature = "alloc")]
            (Value::from(1), Some(1)),
            #[cfg(feature = "alloc")]
            (Value::from(1.0), Some(1)),
        ] {
            assert_eq!(expected, case.parse::<i32>());
        }
    }

    #[test]
    fn as_f64() {
        for (case, expected) in [
            (Value::from(1.0), 1.0),
            (Value::from(2), 2.0),
            (Value::from(true), f64::NAN),
            (Value::from("1.0"), 1.0),
            (Value::from("x"), f64::NAN),
        ] {
            if expected.is_nan() {
                assert!(case.as_f64().is_nan());
            } else {
                assert_eq!(expected, case.as_f64());
            }
        }
    }

    #[cfg(feature = "std")]
    #[test]
    fn error_display() {
        #[derive(Debug)]
        struct Error {
            msg: String,
            source: Option<Box<Error>>,
        }

        impl fmt::Display for Error {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                fmt::Display::fmt(&self.msg, f)
            }
        }

        impl std::error::Error for Error {
            fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                self.source
                    .as_ref()
                    .map(|err| &**err as &(dyn std::error::Error + 'static))
            }
        }

        assert_eq!(
            "outer",
            Value::capture_error(&Error {
                msg: "outer".into(),
                source: None,
            })
            .to_string(),
        );

        assert_eq!(
            "outer (inner)",
            Value::capture_error(&Error {
                msg: "outer".into(),
                source: Some(Box::new(Error {
                    msg: "inner".into(),
                    source: None,
                })),
            })
            .to_string(),
        );

        assert_eq!(
            "outer (root)",
            Value::capture_error(&Error {
                msg: "outer".into(),
                source: Some(Box::new(Error {
                    msg: "inner".into(),
                    source: Some(Box::new(Error {
                        msg: "root".into(),
                        source: None,
                    })),
                })),
            })
            .to_string(),
        );
    }
}
