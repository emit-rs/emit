use sval_derive::Value;

use super::{stream_attributes, stream_field, AnyValue, KeyValue};

#[derive(Value)]
pub struct Resource<'a, A: ?Sized = InlineResourceAttributes<'a>> {
    #[sval(flatten)]
    pub attributes: &'a A,
}

const RESOURCE_ATTRIBUTES_LABEL: sval::Label =
    sval::Label::new("attributes").with_tag(&sval::tags::VALUE_IDENT);

const RESOURCE_ATTRIBUTES_INDEX: sval::Index = sval::Index::new(1);

#[derive(Value)]
pub struct InlineResourceAttributes<'a> {
    #[sval(index = 1)]
    pub attributes: &'a [KeyValue<&'a str, &'a AnyValue<'a>>],
}

pub struct PropsResourceAttributes<P>(pub P);

impl<P: emit::props::Props> sval::Value for PropsResourceAttributes<P> {
    fn stream<'sval, S: sval::Stream<'sval> + ?Sized>(&'sval self, stream: &mut S) -> sval::Result {
        stream.record_tuple_begin(None, None, None, None)?;

        stream_field(
            &mut *stream,
            &RESOURCE_ATTRIBUTES_LABEL,
            &RESOURCE_ATTRIBUTES_INDEX,
            |stream| {
                stream_attributes(stream, &self.0, |mut stream, k, v| {
                    stream.stream_attribute(k, v)
                })
            },
        )?;

        stream.record_tuple_end(None, None, None)
    }
}
