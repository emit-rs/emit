/*!
The [`Props`] type.

Properties, also called attributes in some systems, are the structured data associated with an [`crate::event::Event`]. They are the dimensions an event can be categorized and queried on. Each property is a pair of [`Str`] and [`Value`] that can be inspected or serialized.

[`Props`] allow duplicate keys, but can be de-duplicated by taking the first value seen for a given key. This lets consumers searching for a key short-circuit once they see it instead of needing to scan to the end in case a duplicate is found.

[`Props`] can be fed to a [`crate::template::Template`] to render it into a user-facing message.

Well-known properties described in [`crate::well_known`] are used to extend `emit`'s event model with different kinds of diagnostic data.
*/

use core::{borrow::Borrow, fmt, ops::ControlFlow};

use crate::{
    and::And,
    empty::Empty,
    str::{Str, ToStr},
    value::{FromValue, ToValue, Value},
};

/**
A collection of [`Str`] and [`Value`] pairs.

The [`Props::for_each`] method can be used to enumerate properties.

# Uniqueness

Properties may be duplicated in a set of `Props`. When a property is duplicated, the _first_ for a given key is the one to use.

# Typed and untyped properties

The [`Props::get`] method will return a property as an untyped [`Value`] that can be formatted or serialized. If you're looking for a specific type, you can use [`Props::pull`] instead.
*/
pub trait Props {
    /**
    Enumerate the [`Str`] and [`Value`] pairs.

    The function `for_each` will be called for each property until all properties are visited, or it returns `ControlFlow::Break`.

    Properties may be repeated, but can be de-duplicated by taking the first seen for a given key.
    */
    fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        for_each: F,
    ) -> ControlFlow<()>;

    /**
    Get the value for a given key, if it's present.

    If the key is present then this method will return `Some`. Otherwise this method will return `None`.

    If the key appears multiple times, the first value seen should be returned.

    Implementors are encouraged to override this method with a more efficient implementation.
    */
    fn get<'v, K: ToStr>(&'v self, key: K) -> Option<Value<'v>> {
        let key = key.to_str();
        let mut value = None;

        let _ = self.for_each(|k, v| {
            if k == key {
                value = Some(v);

                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        });

        value
    }

    /**
    Get the value for a given key, if it's present as an instance of `V`.

    If the key is present, and the raw value can be converted into `V` through [`Value::cast`] then this method will return `Some`. Otherwise this method will return `None`.

    If the key appears multiple times, the first value seen should be returned.
    */
    fn pull<'kv, V: FromValue<'kv>, K: ToStr>(&'kv self, key: K) -> Option<V> {
        self.get(key).and_then(|v| v.cast())
    }

    /**
    Whether the collection is known not to contain any duplicate keys.

    If there's any possibility a key may be duplicated, this method should return `false`.
    */
    fn is_unique(&self) -> bool {
        false
    }

    /**
    Concatenate `other` to the end of `self`.
    */
    fn and_props<U: Props>(self, other: U) -> And<Self, U>
    where
        Self: Sized,
    {
        And::new(self, other)
    }

    /**
    Get an adapter that will serialize properties as a map.
    */
    fn as_map(&self) -> &AsMap<Self>
    where
        Self: Sized,
    {
        AsMap::new(self)
    }

    /**
    Lazily de-duplicate properties in the collection.

    Properties are de-duplicated by taking the first value for a given key.
    */
    #[cfg(feature = "alloc")]
    fn dedup(&self) -> &Dedup<Self>
    where
        Self: Sized,
    {
        Dedup::new(self)
    }
}

impl<'a, P: Props + ?Sized> Props for &'a P {
    fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        for_each: F,
    ) -> ControlFlow<()> {
        (**self).for_each(for_each)
    }

    fn get<'v, K: ToStr>(&'v self, key: K) -> Option<Value<'v>> {
        (**self).get(key)
    }

    fn pull<'kv, V: FromValue<'kv>, K: ToStr>(&'kv self, key: K) -> Option<V> {
        (**self).pull(key)
    }

    fn is_unique(&self) -> bool {
        (**self).is_unique()
    }
}

impl<P: Props> Props for Option<P> {
    fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        for_each: F,
    ) -> ControlFlow<()> {
        match self {
            Some(props) => props.for_each(for_each),
            None => ControlFlow::Continue(()),
        }
    }
}

#[cfg(feature = "alloc")]
impl<'a, P: Props + ?Sized + 'a> Props for alloc::boxed::Box<P> {
    fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        for_each: F,
    ) -> ControlFlow<()> {
        (**self).for_each(for_each)
    }
}

#[cfg(feature = "alloc")]
impl<'a, P: Props + ?Sized + 'a> Props for alloc::sync::Arc<P> {
    fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        for_each: F,
    ) -> ControlFlow<()> {
        (**self).for_each(for_each)
    }
}

impl<K: ToStr, V: ToValue> Props for (K, V) {
    fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        mut for_each: F,
    ) -> ControlFlow<()> {
        for_each(self.0.to_str(), self.1.to_value())
    }

    fn is_unique(&self) -> bool {
        true
    }
}

impl<P: Props> Props for [P] {
    fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        mut for_each: F,
    ) -> ControlFlow<()> {
        for p in self {
            p.for_each(&mut for_each)?;
        }

        ControlFlow::Continue(())
    }
}

impl<T, const N: usize> Props for [T; N]
where
    [T]: Props,
{
    fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        for_each: F,
    ) -> ControlFlow<()> {
        (self as &[_]).for_each(for_each)
    }
}

impl Props for Empty {
    fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        _: F,
    ) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }

    fn get<'v, K: ToStr>(&'v self, _: K) -> Option<Value<'v>> {
        None
    }

    fn is_unique(&self) -> bool {
        true
    }
}

impl<A: Props, B: Props> Props for And<A, B> {
    fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        mut for_each: F,
    ) -> ControlFlow<()> {
        self.left().for_each(&mut for_each)?;
        self.right().for_each(for_each)
    }

    fn get<'v, K: ToStr>(&'v self, key: K) -> Option<Value<'v>> {
        let key = key.borrow();

        self.left().get(key).or_else(|| self.right().get(key))
    }
}

#[cfg(feature = "alloc")]
mod alloc_support {
    use super::*;

    use alloc::collections::BTreeMap;

    /**
    The result of calling [`Props::dedup`].

    Properties are de-duplicated by taking the first value for a given key.
    */
    #[repr(transparent)]
    pub struct Dedup<P: ?Sized>(P);

    impl<P: ?Sized> Dedup<P> {
        pub(super) fn new<'a>(props: &'a P) -> &'a Dedup<P> {
            // SAFETY: `Dedup<P>` and `P` have the same ABI
            unsafe { &*(props as *const P as *const Dedup<P>) }
        }
    }

    impl<P: Props + ?Sized> Props for Dedup<P> {
        fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
            &'kv self,
            mut for_each: F,
        ) -> ControlFlow<()> {
            // Optimization for props that are already unique
            if self.0.is_unique() {
                return self.0.for_each(for_each);
            }

            let mut seen = alloc::collections::BTreeMap::new();

            // Ignore any break from this iteration
            let _ = self.0.for_each(|key, value| {
                seen.entry(key).or_insert(value);

                ControlFlow::Continue(())
            });

            for (key, value) in seen {
                for_each(key, value)?;
            }

            ControlFlow::Continue(())
        }

        fn get<'v, K: ToStr>(&'v self, key: K) -> Option<Value<'v>> {
            self.0.get(key)
        }

        fn is_unique(&self) -> bool {
            true
        }
    }

    impl<K, V> Props for BTreeMap<K, V>
    where
        K: Ord + ToStr + Borrow<str>,
        V: ToValue,
    {
        fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
            &'kv self,
            mut for_each: F,
        ) -> ControlFlow<()> {
            for (k, v) in self {
                for_each(k.to_str(), v.to_value())?;
            }

            ControlFlow::Continue(())
        }

        fn get<'v, Q: ToStr>(&'v self, key: Q) -> Option<Value<'v>> {
            self.get(key.to_str().as_ref()).map(|v| v.to_value())
        }

        fn is_unique(&self) -> bool {
            true
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn btreemap_props() {
            let props = BTreeMap::from_iter([("a", 1), ("b", 2), ("c", 3)]);

            assert_eq!(1, Props::get(&props, "a").unwrap().cast::<i32>().unwrap());
            assert_eq!(2, Props::get(&props, "b").unwrap().cast::<i32>().unwrap());
            assert_eq!(3, Props::get(&props, "c").unwrap().cast::<i32>().unwrap());

            assert_eq!(1, Props::pull::<i32, _>(&props, "a").unwrap());
            assert_eq!(2, Props::pull::<i32, _>(&props, "b").unwrap());
            assert_eq!(3, Props::pull::<i32, _>(&props, "c").unwrap());

            assert!(props.is_unique());
        }

        #[test]
        fn dedup() {
            let props = [
                ("a", Value::from(1)),
                ("a", Value::from(2)),
                ("b", Value::from(1)),
            ];

            let deduped = props.dedup();

            let mut ac = 0;
            let mut bc = 0;

            let _ = deduped.for_each(|k, v| {
                match k.get() {
                    "a" => {
                        assert_eq!(1, v.cast::<i32>().unwrap());
                        ac += 1;
                    }
                    "b" => {
                        assert_eq!(1, v.cast::<i32>().unwrap());
                        bc += 1;
                    }
                    _ => unreachable!(),
                }

                ControlFlow::Continue(())
            });

            assert_eq!(1, ac);
            assert_eq!(1, bc);
        }
    }
}

#[cfg(feature = "alloc")]
pub use alloc_support::*;

#[cfg(feature = "std")]
mod std_support {
    use super::*;

    use std::{collections::HashMap, hash::Hash};

    impl<K, V> Props for HashMap<K, V>
    where
        K: Eq + Hash + ToStr + Borrow<str>,
        V: ToValue,
    {
        fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
            &'kv self,
            mut for_each: F,
        ) -> ControlFlow<()> {
            for (k, v) in self {
                for_each(k.to_str(), v.to_value())?;
            }

            ControlFlow::Continue(())
        }

        fn get<'v, Q: ToStr>(&'v self, key: Q) -> Option<Value<'v>> {
            self.get(key.to_str().as_ref()).map(|v| v.to_value())
        }

        fn is_unique(&self) -> bool {
            true
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn hashmap_props() {
            let props = HashMap::from_iter([("a", 1), ("b", 2), ("c", 3)]);

            assert_eq!(1, Props::get(&props, "a").unwrap().cast::<i32>().unwrap());
            assert_eq!(2, Props::get(&props, "b").unwrap().cast::<i32>().unwrap());
            assert_eq!(3, Props::get(&props, "c").unwrap().cast::<i32>().unwrap());

            assert_eq!(1, Props::pull::<i32, _>(&props, "a").unwrap());
            assert_eq!(2, Props::pull::<i32, _>(&props, "b").unwrap());
            assert_eq!(3, Props::pull::<i32, _>(&props, "c").unwrap());

            assert!(props.is_unique());
        }
    }
}

/**
The result of calling [`Props::as_map`].

This type implements serialization traits, serializing properties as a map of key-value pairs.
*/
#[repr(transparent)]
pub struct AsMap<P: ?Sized>(P);

impl<P: ?Sized> AsMap<P> {
    fn new<'a>(props: &'a P) -> &'a AsMap<P> {
        // SAFETY: `AsMap<P>` and `P` have the same ABI
        unsafe { &*(props as *const P as *const AsMap<P>) }
    }
}

impl<P: Props + ?Sized> Props for AsMap<P> {
    fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        for_each: F,
    ) -> ControlFlow<()> {
        self.0.for_each(for_each)
    }

    fn get<'v, K: ToStr>(&'v self, key: K) -> Option<Value<'v>> {
        self.0.get(key)
    }

    fn pull<'kv, V: FromValue<'kv>, K: ToStr>(&'kv self, key: K) -> Option<V> {
        self.0.pull(key)
    }

    fn is_unique(&self) -> bool {
        self.0.is_unique()
    }
}

#[cfg(feature = "sval")]
impl<P: Props + ?Sized> sval::Value for AsMap<P> {
    fn stream<'sval, S: sval::Stream<'sval> + ?Sized>(&'sval self, stream: &mut S) -> sval::Result {
        stream.map_begin(None)?;

        let mut r = Ok(());
        let _ = self.for_each(|k, v| {
            r = (|| {
                stream.map_key_begin()?;
                sval_ref::stream_ref(&mut *stream, k)?;
                stream.map_key_end()?;

                stream.map_value_begin()?;
                sval_ref::stream_ref(&mut *stream, v)?;
                stream.map_value_end()
            })();

            if r.is_ok() {
                ControlFlow::Continue(())
            } else {
                ControlFlow::Break(())
            }
        });
        r?;

        stream.map_end()
    }
}

#[cfg(feature = "serde")]
impl<P: Props + ?Sized> serde::Serialize for AsMap<P> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap as _;

        let mut err = None;

        let mut map = serializer.serialize_map(None)?;

        let _ = self.for_each(|k, v| match map.serialize_entry(&k, &v) {
            Ok(()) => ControlFlow::Continue(()),
            Err(e) => {
                err = Some(e);
                ControlFlow::Break(())
            }
        });

        if let Some(e) = err {
            return Err(e);
        }

        map.end()
    }
}

impl<P: Props + ?Sized> fmt::Debug for AsMap<P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl<P: Props + ?Sized> fmt::Display for AsMap<P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut map = f.debug_map();

        let _ = self.for_each(|k, v| {
            map.entry(&k, &v);

            ControlFlow::Continue(())
        });

        map.finish()
    }
}

mod internal {
    use core::ops::ControlFlow;

    use crate::{str::Str, value::Value};

    pub trait DispatchProps {
        fn dispatch_for_each<'kv, 'f>(
            &'kv self,
            for_each: &'f mut dyn FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>,
        ) -> ControlFlow<()>;

        fn dispatch_get(&self, key: Str) -> Option<Value>;

        fn dispatch_is_unique(&self) -> bool;
    }

    pub trait SealedProps {
        fn erase_props(&self) -> crate::internal::Erased<&dyn DispatchProps>;
    }
}

/**
An object-safe [`Props`].

A `dyn ErasedProps` can be treated as `impl Props`.
*/
pub trait ErasedProps: internal::SealedProps {}

impl<P: Props> ErasedProps for P {}

impl<P: Props> internal::SealedProps for P {
    fn erase_props(&self) -> crate::internal::Erased<&dyn internal::DispatchProps> {
        crate::internal::Erased(self)
    }
}

impl<P: Props> internal::DispatchProps for P {
    fn dispatch_for_each<'kv, 'f>(
        &'kv self,
        for_each: &'f mut dyn FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>,
    ) -> ControlFlow<()> {
        self.for_each(for_each)
    }

    fn dispatch_get<'v>(&'v self, key: Str) -> Option<Value<'v>> {
        self.get(key)
    }

    fn dispatch_is_unique(&self) -> bool {
        self.is_unique()
    }
}

impl<'a> Props for dyn ErasedProps + 'a {
    fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        mut for_each: F,
    ) -> ControlFlow<()> {
        self.erase_props().0.dispatch_for_each(&mut for_each)
    }

    fn get<'v, K: ToStr>(&'v self, key: K) -> Option<Value<'v>> {
        self.erase_props().0.dispatch_get(key.to_str())
    }

    fn is_unique(&self) -> bool {
        self.erase_props().0.dispatch_is_unique()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tuple_props() {
        let props = ("a", 1);

        assert_eq!(1, props.get("a").unwrap().cast::<i32>().unwrap());

        assert_eq!(1, props.pull::<i32, _>("a").unwrap());

        assert!(props.is_unique());
    }

    #[test]
    fn array_props() {
        let props = [("a", 1), ("b", 2), ("c", 3)];

        assert_eq!(1, props.get("a").unwrap().cast::<i32>().unwrap());
        assert_eq!(2, props.get("b").unwrap().cast::<i32>().unwrap());
        assert_eq!(3, props.get("c").unwrap().cast::<i32>().unwrap());

        assert_eq!(1, props.pull::<i32, _>("a").unwrap());
        assert_eq!(2, props.pull::<i32, _>("b").unwrap());
        assert_eq!(3, props.pull::<i32, _>("c").unwrap());

        assert!(!props.is_unique());
    }

    #[test]
    fn option_props() {
        for (props, expected) in [(Some(("a", 1)), Some(1)), (None, None)] {
            assert_eq!(expected, props.pull::<i32, _>("a"));
        }
    }

    #[test]
    fn erased_props() {
        let props = ("a", 1);

        let props = &props as &dyn ErasedProps;

        assert_eq!(1, props.get("a").unwrap().cast::<i32>().unwrap());

        assert_eq!(1, props.pull::<i32, _>("a").unwrap());

        assert!(props.is_unique());
    }

    #[test]
    fn get() {
        let props = [("a", 1), ("a", 2)];

        assert_eq!(1, props.get("a").unwrap().cast::<i32>().unwrap());
    }

    #[test]
    fn pull() {
        let props = [("a", 1), ("a", 2)];

        assert_eq!(1, props.pull::<i32, _>("a").unwrap());
    }

    #[test]
    fn and_props() {
        let a = ("a", 1);
        let b = [("b", 2), ("c", 3)];

        let props = a.and_props(b);

        assert_eq!(1, props.get("a").unwrap().cast::<i32>().unwrap());
        assert_eq!(2, props.get("b").unwrap().cast::<i32>().unwrap());
        assert_eq!(3, props.get("c").unwrap().cast::<i32>().unwrap());

        assert_eq!(1, props.pull::<i32, _>("a").unwrap());
        assert_eq!(2, props.pull::<i32, _>("b").unwrap());
        assert_eq!(3, props.pull::<i32, _>("c").unwrap());

        assert!(!props.is_unique());
    }

    #[test]
    fn as_map() {
        let props = [("a", 1), ("b", 2)].as_map();

        assert_eq!("{\"a\": 1, \"b\": 2}", props.to_string());
    }

    #[cfg(feature = "sval")]
    #[test]
    fn as_map_stream() {
        let props = [("a", 1), ("b", 2)].as_map();

        sval_test::assert_tokens(
            &props,
            &[
                sval_test::Token::MapBegin(None),
                sval_test::Token::MapKeyBegin,
                sval_test::Token::TextBegin(Some(1)),
                sval_test::Token::TextFragmentComputed("a".to_owned()),
                sval_test::Token::TextEnd,
                sval_test::Token::MapKeyEnd,
                sval_test::Token::MapValueBegin,
                sval_test::Token::I64(1),
                sval_test::Token::MapValueEnd,
                sval_test::Token::MapKeyBegin,
                sval_test::Token::TextBegin(Some(1)),
                sval_test::Token::TextFragmentComputed("b".to_owned()),
                sval_test::Token::TextEnd,
                sval_test::Token::MapKeyEnd,
                sval_test::Token::MapValueBegin,
                sval_test::Token::I64(2),
                sval_test::Token::MapValueEnd,
                sval_test::Token::MapEnd,
            ],
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn as_map_serialize() {
        let props = [("a", 1), ("b", 2)].as_map();

        serde_test::assert_ser_tokens(
            &props,
            &[
                serde_test::Token::Map { len: None },
                serde_test::Token::Str("a"),
                serde_test::Token::I64(1),
                serde_test::Token::Str("b"),
                serde_test::Token::I64(2),
                serde_test::Token::MapEnd,
            ],
        );
    }
}
