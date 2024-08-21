use sval_derive::Value;

use super::stream_field;

const ANY_VALUE_STRING_LABEL: sval::Label =
    sval::Label::new("stringValue").with_tag(&sval::tags::VALUE_IDENT);
const ANY_VALUE_BOOL_LABEL: sval::Label =
    sval::Label::new("boolValue").with_tag(&sval::tags::VALUE_IDENT);
const ANY_VALUE_INT_LABEL: sval::Label =
    sval::Label::new("intValue").with_tag(&sval::tags::VALUE_IDENT);
const ANY_VALUE_DOUBLE_LABEL: sval::Label =
    sval::Label::new("doubleValue").with_tag(&sval::tags::VALUE_IDENT);
const ANY_VALUE_ARRAY_LABEL: sval::Label =
    sval::Label::new("arrayValue").with_tag(&sval::tags::VALUE_IDENT);
const ANY_VALUE_KVLIST_LABEL: sval::Label =
    sval::Label::new("kvlistValue").with_tag(&sval::tags::VALUE_IDENT);
const ANY_VALUE_BYTES_LABEL: sval::Label =
    sval::Label::new("bytesValue").with_tag(&sval::tags::VALUE_IDENT);

const ANY_VALUE_STRING_INDEX: sval::Index = sval::Index::new(1);
const ANY_VALUE_BOOL_INDEX: sval::Index = sval::Index::new(2);
const ANY_VALUE_INT_INDEX: sval::Index = sval::Index::new(3);
const ANY_VALUE_DOUBLE_INDEX: sval::Index = sval::Index::new(4);
const ANY_VALUE_ARRAY_INDEX: sval::Index = sval::Index::new(5);
const ANY_VALUE_KVLIST_INDEX: sval::Index = sval::Index::new(6);
const ANY_VALUE_BYTES_INDEX: sval::Index = sval::Index::new(7);

#[derive(Value)]
pub enum AnyValue<'a, SV: ?Sized = str> {
    #[sval(label = ANY_VALUE_STRING_LABEL, index = ANY_VALUE_STRING_INDEX)]
    String(&'a SV),
}

const ARRAY_VALUES_LABEL: sval::Label =
    sval::Label::new("values").with_tag(&sval::tags::VALUE_IDENT);

const ARRAY_VALUES_INDEX: sval::Index = sval::Index::new(1);

#[derive(Value)]
pub struct ArrayValue<'a> {
    #[sval(label = ARRAY_VALUES_LABEL, index = ARRAY_VALUES_INDEX)]
    pub values: &'a [AnyValue<'a>],
}

const KVLIST_VALUES_LABEL: sval::Label =
    sval::Label::new("values").with_tag(&sval::tags::VALUE_IDENT);

const KVLIST_VALUES_INDEX: sval::Index = sval::Index::new(1);

#[derive(Value)]
pub struct KvList<'a> {
    #[sval(label = KVLIST_VALUES_LABEL, index = KVLIST_VALUES_INDEX)]
    pub values: &'a [KeyValue<&'a str, &'a AnyValue<'a>>],
}

const KEY_VALUE_KEY_LABEL: sval::Label = sval::Label::new("key").with_tag(&sval::tags::VALUE_IDENT);
const KEY_VALUE_VALUE_LABEL: sval::Label =
    sval::Label::new("value").with_tag(&sval::tags::VALUE_IDENT);

const KEY_VALUE_KEY_INDEX: sval::Index = sval::Index::new(1);
const KEY_VALUE_VALUE_INDEX: sval::Index = sval::Index::new(2);

#[derive(Value)]
pub struct KeyValue<K, V> {
    #[sval(label = KEY_VALUE_KEY_LABEL, index = KEY_VALUE_KEY_INDEX)]
    pub key: K,
    #[sval(label = KEY_VALUE_VALUE_LABEL, index = KEY_VALUE_VALUE_INDEX)]
    pub value: V,
}

impl<'a, K: sval_ref::ValueRef<'a>, V: sval_ref::ValueRef<'a>> sval_ref::ValueRef<'a>
    for KeyValue<K, V>
{
    fn stream_ref<S: sval::Stream<'a> + ?Sized>(&self, stream: &mut S) -> sval::Result {
        stream.record_tuple_begin(None, None, None, Some(2))?;

        stream_field(
            &mut *stream,
            &KEY_VALUE_KEY_LABEL,
            &KEY_VALUE_KEY_INDEX,
            |stream| sval_ref::stream_ref(&mut *stream, &self.key),
        )?;

        stream_field(
            &mut *stream,
            &KEY_VALUE_VALUE_LABEL,
            &KEY_VALUE_VALUE_INDEX,
            |stream| sval_ref::stream_ref(&mut *stream, &self.value),
        )?;

        stream.record_tuple_end(None, None, None)
    }
}

pub struct EmitValue<'a>(pub emit::value::Value<'a>);

impl<'a> sval::Value for EmitValue<'a> {
    fn stream<'sval, S: sval::Stream<'sval> + ?Sized>(&'sval self, stream: &mut S) -> sval::Result {
        use sval_ref::ValueRef as _;

        self.stream_ref(stream)
    }
}

impl<'a> sval_ref::ValueRef<'a> for EmitValue<'a> {
    fn stream_ref<S: sval::Stream<'a> + ?Sized>(&self, stream: &mut S) -> sval::Result {
        struct AnyStream<S> {
            stream: S,
            in_map_key: bool,
        }

        impl<'sval, S: sval::Stream<'sval>> AnyStream<S> {
            fn any_value_begin(
                &mut self,
                label: &sval::Label,
                index: &sval::Index,
            ) -> sval::Result {
                self.stream.enum_begin(None, None, None)?;
                self.stream.tagged_begin(None, Some(label), Some(index))
            }

            fn any_value_end(&mut self, label: &sval::Label, index: &sval::Index) -> sval::Result {
                self.stream.tagged_end(None, Some(label), Some(index))?;
                self.stream.enum_end(None, None, None)
            }
        }

        impl<'sval, S: sval::Stream<'sval>> sval::Stream<'sval> for AnyStream<S> {
            fn null(&mut self) -> sval::Result {
                self.stream.null()
            }

            fn bool(&mut self, value: bool) -> sval::Result {
                if self.in_map_key {
                    todo!()
                }

                self.any_value_begin(&ANY_VALUE_BOOL_LABEL, &ANY_VALUE_BOOL_INDEX)?;
                self.stream.bool(value)?;
                self.any_value_end(&ANY_VALUE_BOOL_LABEL, &ANY_VALUE_BOOL_INDEX)
            }

            fn text_begin(&mut self, num_bytes: Option<usize>) -> sval::Result {
                if !self.in_map_key {
                    self.any_value_begin(&ANY_VALUE_STRING_LABEL, &ANY_VALUE_STRING_INDEX)?;
                }

                self.stream.text_begin(num_bytes)
            }

            fn text_fragment(&mut self, fragment: &'sval str) -> sval::Result {
                self.stream.text_fragment(fragment)
            }

            fn text_fragment_computed(&mut self, fragment: &str) -> sval::Result {
                self.stream.text_fragment_computed(fragment)
            }

            fn text_end(&mut self) -> sval::Result {
                self.stream.text_end()?;

                if !self.in_map_key {
                    self.any_value_end(&ANY_VALUE_STRING_LABEL, &ANY_VALUE_STRING_INDEX)?;
                }

                Ok(())
            }

            fn i64(&mut self, value: i64) -> sval::Result {
                if self.in_map_key {
                    todo!()
                }

                self.any_value_begin(&ANY_VALUE_INT_LABEL, &ANY_VALUE_INT_INDEX)?;
                self.stream.i64(value)?;
                self.any_value_end(&ANY_VALUE_INT_LABEL, &ANY_VALUE_INT_INDEX)
            }

            fn f64(&mut self, value: f64) -> sval::Result {
                if self.in_map_key {
                    todo!()
                }

                self.any_value_begin(&ANY_VALUE_DOUBLE_LABEL, &ANY_VALUE_DOUBLE_INDEX)?;
                self.stream.f64(value)?;
                self.any_value_end(&ANY_VALUE_DOUBLE_LABEL, &ANY_VALUE_DOUBLE_INDEX)
            }

            fn binary_begin(&mut self, num_bytes: Option<usize>) -> sval::Result {
                if self.in_map_key {
                    todo!()
                }

                self.any_value_begin(&ANY_VALUE_BYTES_LABEL, &ANY_VALUE_BYTES_INDEX)?;
                self.stream.binary_begin(num_bytes)
            }

            fn binary_fragment(&mut self, fragment: &'sval [u8]) -> sval::Result {
                self.stream.binary_fragment(fragment)
            }

            fn binary_fragment_computed(&mut self, fragment: &[u8]) -> sval::Result {
                self.stream.binary_fragment_computed(fragment)
            }

            fn binary_end(&mut self) -> sval::Result {
                self.stream.binary_end()?;
                self.any_value_end(&ANY_VALUE_BYTES_LABEL, &ANY_VALUE_BYTES_INDEX)
            }

            fn seq_begin(&mut self, num_entries: Option<usize>) -> sval::Result {
                if self.in_map_key {
                    todo!()
                }

                self.any_value_begin(&ANY_VALUE_ARRAY_LABEL, &ANY_VALUE_ARRAY_INDEX)?;
                self.stream.record_tuple_begin(None, None, None, Some(1))?;
                self.stream.record_tuple_value_begin(
                    None,
                    &ARRAY_VALUES_LABEL,
                    &ARRAY_VALUES_INDEX,
                )?;
                self.stream.seq_begin(num_entries)
            }

            fn seq_value_begin(&mut self) -> sval::Result {
                self.stream.seq_value_begin()
            }

            fn seq_value_end(&mut self) -> sval::Result {
                self.stream.seq_value_end()
            }

            fn seq_end(&mut self) -> sval::Result {
                self.stream.seq_end()?;
                self.stream.record_tuple_value_end(
                    None,
                    &ARRAY_VALUES_LABEL,
                    &ARRAY_VALUES_INDEX,
                )?;
                self.stream.record_tuple_end(None, None, None)?;
                self.any_value_end(&ANY_VALUE_ARRAY_LABEL, &ANY_VALUE_ARRAY_INDEX)
            }

            fn map_begin(&mut self, num_entries: Option<usize>) -> sval::Result {
                if self.in_map_key {
                    todo!()
                }

                self.any_value_begin(&ANY_VALUE_KVLIST_LABEL, &ANY_VALUE_KVLIST_INDEX)?;
                self.stream.record_tuple_begin(None, None, None, Some(1))?;
                self.stream.record_tuple_value_begin(
                    None,
                    &ARRAY_VALUES_LABEL,
                    &ARRAY_VALUES_INDEX,
                )?;
                self.stream.seq_begin(num_entries)
            }

            fn map_key_begin(&mut self) -> sval::Result {
                self.in_map_key = true;

                self.stream.seq_value_begin()?;
                self.stream.record_tuple_begin(None, None, None, Some(2))?;
                self.stream.record_tuple_value_begin(
                    None,
                    &KEY_VALUE_KEY_LABEL,
                    &KEY_VALUE_KEY_INDEX,
                )
            }

            fn map_key_end(&mut self) -> sval::Result {
                self.in_map_key = false;

                self.stream
                    .record_tuple_value_end(None, &KEY_VALUE_KEY_LABEL, &KEY_VALUE_KEY_INDEX)
            }

            fn map_value_begin(&mut self) -> sval::Result {
                self.stream.record_tuple_value_begin(
                    None,
                    &KEY_VALUE_VALUE_LABEL,
                    &KEY_VALUE_VALUE_INDEX,
                )
            }

            fn map_value_end(&mut self) -> sval::Result {
                self.stream.record_tuple_value_end(
                    None,
                    &KEY_VALUE_VALUE_LABEL,
                    &KEY_VALUE_VALUE_INDEX,
                )?;
                self.stream.record_tuple_end(None, None, None)?;
                self.stream.seq_value_end()
            }

            fn map_end(&mut self) -> sval::Result {
                self.stream.seq_end()?;
                self.stream.record_tuple_value_end(
                    None,
                    &KVLIST_VALUES_LABEL,
                    &KVLIST_VALUES_INDEX,
                )?;
                self.stream.record_tuple_end(None, None, None)?;
                self.any_value_end(&ANY_VALUE_KVLIST_LABEL, &ANY_VALUE_KVLIST_INDEX)
            }
        }

        sval_ref::stream_ref(
            &mut AnyStream {
                stream,
                in_map_key: false,
            },
            &self.0,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use prost::Message;

    use crate::data::generated::{common::v1 as common, util::*};

    fn encode<'a>(v: impl Into<emit::Value<'a>>) -> impl bytes::Buf {
        sval_protobuf::stream_to_protobuf(EmitValue(v.into())).into_cursor()
    }

    #[test]
    fn encode_string() {
        let de = common::AnyValue::decode(encode("string value")).unwrap();

        assert_eq!(string_value("string value"), de);
    }

    #[test]
    fn encode_bytes() {
        let de = common::AnyValue::decode(encode(emit::Value::capture_sval(
            &sval::BinarySlice::new(b"bytes value"),
        )))
        .unwrap();

        assert_eq!(bytes_value(b"bytes value"), de);
    }

    #[test]
    fn encode_bool() {
        let de = common::AnyValue::decode(encode(true)).unwrap();

        assert_eq!(bool_value(true), de);
    }

    #[test]
    fn encode_int_i32() {
        let de = common::AnyValue::decode(encode(42)).unwrap();

        assert_eq!(int_value(42), de);
    }

    #[test]
    fn encode_int_i128_oversize() {
        let de = common::AnyValue::decode(encode(i128::MAX)).unwrap();

        assert_eq!(string_value(i128::MAX.to_string()), de);
    }

    #[test]
    fn encode_double() {
        let de = common::AnyValue::decode(encode(42.1)).unwrap();

        assert_eq!(double_value(42.1), de);
    }

    #[test]
    fn encode_array() {
        let de = common::AnyValue::decode(encode(emit::Value::capture_sval(&[1, 2, 3]))).unwrap();

        assert_eq!(array_value([int_value(1), int_value(2), int_value(3)]), de);
    }

    #[test]
    fn encode_array_nested() {
        let de = common::AnyValue::decode(encode(emit::Value::capture_sval(&[
            emit::Value::from(1),
            emit::Value::from(emit::Value::capture_sval(&[1, 2, 3])),
            emit::Value::from(3),
        ])))
        .unwrap();

        assert_eq!(
            array_value([
                int_value(1),
                array_value([int_value(1), int_value(2), int_value(3)]),
                int_value(3)
            ]),
            de
        );
    }

    #[test]
    fn encode_kvlist() {
        let de =
            common::AnyValue::decode(encode(emit::Value::capture_sval(&sval::MapSlice::new(&[
                ("a", 1),
                ("b", 2),
                ("c", 3),
            ]))))
            .unwrap();

        assert_eq!(
            kvlist_value([
                ("a".into(), int_value(1)),
                ("b".into(), int_value(2)),
                ("c".into(), int_value(3)),
            ]),
            de
        );
    }

    #[test]
    fn encode_kvlist_nested() {
        let de = common::AnyValue::decode(encode(emit::Value::from_sval(&sval::MapSlice::new(&[
            ("a", emit::Value::from(1)),
            (
                "b",
                emit::Value::from_sval(&sval::MapSlice::new(&[("a", 1), ("b", 2), ("c", 3)])),
            ),
            ("c", emit::Value::from(3)),
        ]))))
        .unwrap();

        assert_eq!(
            kvlist_value([
                ("a".into(), int_value(1)),
                (
                    "b".into(),
                    kvlist_value([
                        ("a".into(), int_value(1)),
                        ("b".into(), int_value(2)),
                        ("c".into(), int_value(3)),
                    ])
                ),
                ("c".into(), int_value(3)),
            ]),
            de
        );
    }
}
