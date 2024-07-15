/*!
The [`Props`] type.

Properties, also called attributes in some systems, are the structured data associated with an [`crate::event::Event`]. They are the dimensions an event can be categorized and queried on. Each property is a pair of [`Str`] and [`Value`] that can be inspected or serialized.

[`Props`] allow duplicate keys, but can be de-duplicated by taking the first value seen for a given key. This lets consumers searching for a key short-circuit once they see it instead of needing to scan to the end in case a duplicate is found.

[`Props`] can be fed to a [`crate::template::Template`] to render it into a user-facing message.

Well-known properties described in [`crate::well_known`] are used to extend `emit`'s event model with different kinds of diagnostic data.
*/

use core::{borrow::Borrow, ops::ControlFlow};

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

The [`Props::get`] method will return a property as an untyped [`Value`] that can be formatted or serialized. IF you're looking for a specific type, you can use [`Props::pull`] instead.
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

        self.for_each(|k, v| {
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

#[cfg(feature = "alloc")]
impl<K, V> Props for alloc::collections::BTreeMap<K, V>
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

#[cfg(feature = "std")]
impl<K, V> Props for std::collections::HashMap<K, V>
where
    K: Eq + std::hash::Hash + ToStr + Borrow<str>,
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

            self.0.for_each(|key, value| {
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

    #[cfg(test)]
    mod tests {
        use super::*;

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

            deduped.for_each(|k, v| {
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
