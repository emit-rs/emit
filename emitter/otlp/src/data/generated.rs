#[path = ""]
pub(crate) mod google {
    #[path = "./generated/google.rpc.rs"]
    pub(crate) mod rpc;
}

#[path = ""]
pub(crate) mod logs {
    #[path = "./generated/opentelemetry.proto.logs.v1.rs"]
    pub(crate) mod v1;
}

#[path = ""]
pub(crate) mod trace {
    #[path = "./generated/opentelemetry.proto.trace.v1.rs"]
    pub(crate) mod v1;
}

#[path = ""]
pub(crate) mod metrics {
    #[path = "./generated/opentelemetry.proto.metrics.v1.rs"]
    pub(crate) mod v1;
}

#[path = ""]
pub(crate) mod common {
    #[path = "./generated/opentelemetry.proto.common.v1.rs"]
    pub(crate) mod v1;
}

#[path = ""]
pub(crate) mod resource {
    #[path = "./generated/opentelemetry.proto.resource.v1.rs"]
    pub(crate) mod v1;
}

#[path = ""]
pub(crate) mod collector {
    #[path = ""]
    pub(crate) mod logs {
        #[path = "./generated/opentelemetry.proto.collector.logs.v1.rs"]
        pub(crate) mod v1;
    }

    #[path = ""]
    pub(crate) mod trace {
        #[path = "./generated/opentelemetry.proto.collector.trace.v1.rs"]
        pub(crate) mod v1;
    }

    #[path = ""]
    pub(crate) mod metrics {
        #[path = "./generated/opentelemetry.proto.collector.metrics.v1.rs"]
        pub(crate) mod v1;
    }
}

#[cfg(test)]
pub(crate) mod util {
    use super::*;

    pub(crate) fn string_value(v: impl Into<String>) -> common::v1::AnyValue {
        common::v1::AnyValue {
            value: Some(common::v1::any_value::Value::StringValue(v.into())),
        }
    }

    pub(crate) fn bool_value(v: impl Into<bool>) -> common::v1::AnyValue {
        common::v1::AnyValue {
            value: Some(common::v1::any_value::Value::BoolValue(v.into())),
        }
    }

    pub(crate) fn int_value(v: impl Into<i64>) -> common::v1::AnyValue {
        common::v1::AnyValue {
            value: Some(common::v1::any_value::Value::IntValue(v.into())),
        }
    }

    pub(crate) fn double_value(v: impl Into<f64>) -> common::v1::AnyValue {
        common::v1::AnyValue {
            value: Some(common::v1::any_value::Value::DoubleValue(v.into())),
        }
    }

    pub(crate) fn bytes_value(v: impl Into<Vec<u8>>) -> common::v1::AnyValue {
        common::v1::AnyValue {
            value: Some(common::v1::any_value::Value::BytesValue(v.into())),
        }
    }

    pub(crate) fn array_value(
        v: impl IntoIterator<Item = common::v1::AnyValue>,
    ) -> common::v1::AnyValue {
        common::v1::AnyValue {
            value: Some(common::v1::any_value::Value::ArrayValue(
                common::v1::ArrayValue {
                    values: v.into_iter().collect(),
                },
            )),
        }
    }

    pub(crate) fn kvlist_value(
        v: impl IntoIterator<Item = (String, common::v1::AnyValue)>,
    ) -> common::v1::AnyValue {
        common::v1::AnyValue {
            value: Some(common::v1::any_value::Value::KvlistValue(
                common::v1::KeyValueList {
                    values: v
                        .into_iter()
                        .map(|(key, value)| common::v1::KeyValue {
                            key,
                            value: Some(value),
                        })
                        .collect(),
                },
            )),
        }
    }
}
