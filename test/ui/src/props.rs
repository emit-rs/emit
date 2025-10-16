use std::fmt;

use emit::Props;

#[test]
fn props_basic() {
    let props = emit::props! {
        b: 1,
        a: true,
        c: 2.0,
        d: "text",
    };

    assert!(props.is_unique());

    assert_eq!(1, props.pull::<i32, _>("b").unwrap());
    assert_eq!(true, props.pull::<bool, _>("a").unwrap());
    assert_eq!(2.0, props.pull::<f64, _>("c").unwrap());
    assert_eq!("text", props.pull::<&str, _>("d").unwrap());
}

#[test]
fn props_uncooked() {
    let props = emit::props! {
        r#type: 1,
    };

    assert_eq!(1, props.pull::<i32, _>("type").unwrap());
}

#[test]
fn props_external() {
    let x = 42;

    let props = emit::props! {
        x,
    };

    assert_eq!(42, props.pull::<i32, _>("x").unwrap());
}

#[test]
fn props_event_meta() {
    let _ = emit::props!(
        ts_start: "2024-01-01T00:00:00.000Z",
        ts: "2024-01-01T00:00:01.000Z",
        tpl: "template",
        msg: "message",
        mdl: "module",
    );
}

#[test]
fn props_cfg() {
    let props = emit::props! {
        #[cfg(not(emit_disabled))]
        enabled: "enabled",
        #[cfg(emit_disabled)]
        disabled: "disabled",
    };

    assert_eq!("enabled", props.pull::<&str, _>("enabled").unwrap());
    assert!(props.get("disabled").is_none());
}

#[test]
#[cfg(feature = "std")]
fn props_capture_err() {
    use std::{error, io};

    let err = io::Error::new(io::ErrorKind::Other, "Some error");

    match emit::props! {
        err,
    } {
        props => {
            let err = props
                .pull::<&(dyn error::Error + 'static), _>("err")
                .unwrap();

            assert_eq!("Some error", err.to_string());
        }
    }
}

#[test]
fn props_capture_err_string() {
    let props = emit::props! {
        err: "Some error",
    };

    let err = props.pull::<&str, _>("err").unwrap();

    assert_eq!("Some error", err);
}

#[test]
#[cfg(feature = "std")]
fn props_capture_err_anyhow() {
    use std::error;

    let err = anyhow::Error::msg("Some error");

    match emit::props! {
        err: emit::err::as_ref(&err),
    } {
        props => {
            let err = props
                .pull::<&(dyn error::Error + 'static), _>("err")
                .unwrap();

            assert_eq!("Some error", err.to_string());
        }
    }
}

#[test]
fn props_capture_err_as_non_err() {
    let props = emit::props! {
        #[emit::as_display(inspect: true)] err: true,
    };

    let err = props.pull::<bool, _>("err").unwrap();

    assert_eq!(true, err);
}

#[test]
fn props_capture_lvl() {
    let props = emit::props! {
        lvl: emit::Level::Info,
    };

    assert_eq!(
        emit::Level::Info,
        props.pull::<emit::Level, _>("lvl").unwrap()
    );
}

#[test]
fn props_capture_lvl_string() {
    let props = emit::props! {
        lvl: "info",
    };

    assert_eq!(
        emit::Level::Info,
        props.pull::<emit::Level, _>("lvl").unwrap()
    );
}

#[test]
fn props_capture_lvl_as_non_lvl() {
    let props = emit::props! {
        #[emit::as_display(inspect: true)] lvl: true,
    };

    assert_eq!(true, props.pull::<bool, _>("lvl").unwrap());
}

#[test]
fn props_capture_trace_id() {
    let trace_id = emit::TraceId::from_u128(1);

    let props = emit::props! {
        trace_id,
    };

    assert_eq!(
        emit::TraceId::from_u128(1).unwrap(),
        props.pull::<emit::TraceId, _>("trace_id").unwrap()
    );
}

#[test]
fn props_capture_trace_id_string() {
    let props = emit::props! {
        trace_id: "00000000000000000000000000000001",
    };

    assert_eq!(
        emit::TraceId::from_u128(1).unwrap(),
        props.pull::<emit::TraceId, _>("trace_id").unwrap()
    );
}

#[test]
fn props_capture_trace_id_u128() {
    let props = emit::props! {
        trace_id: 0x00000000000000000000000000000001u128,
    };

    assert_eq!(
        emit::TraceId::from_u128(1).unwrap(),
        props.pull::<emit::TraceId, _>("trace_id").unwrap()
    );
}

#[test]
fn props_capture_trace_id_as_non_trace_id() {
    let props = emit::props! {
        #[emit::as_display(inspect: true)] trace_id: true,
    };

    assert_eq!(true, props.pull::<bool, _>("trace_id").unwrap());
}

#[test]
fn props_capture_span_id() {
    let span_id = emit::SpanId::from_u64(1);

    let props = emit::props! {
        span_id,
    };

    assert_eq!(
        emit::SpanId::from_u64(1).unwrap(),
        props.pull::<emit::SpanId, _>("span_id").unwrap()
    );
}

#[test]
fn props_capture_span_id_string() {
    let props = emit::props! {
        span_id: "0000000000000001",
    };

    assert_eq!(
        emit::SpanId::from_u64(1).unwrap(),
        props.pull::<emit::SpanId, _>("span_id").unwrap()
    );
}

#[test]
fn props_capture_span_id_u64() {
    let props = emit::props! {
        span_id: 0x0000000000000001u64,
    };

    assert_eq!(
        emit::SpanId::from_u64(1).unwrap(),
        props.pull::<emit::SpanId, _>("span_id").unwrap()
    );
}

#[test]
fn props_capture_span_id_as_non_span_id() {
    let props = emit::props! {
        #[emit::as_display(inspect: true)] span_id: true,
    };

    assert_eq!(true, props.pull::<bool, _>("span_id").unwrap());
}

#[test]
fn props_capture_span_parent() {
    let span_parent = emit::SpanId::from_u64(1);

    let props = emit::props! {
        span_parent,
    };

    assert_eq!(
        emit::SpanId::from_u64(1).unwrap(),
        props.pull::<emit::SpanId, _>("span_parent").unwrap()
    );
}

#[test]
fn props_capture_span_parent_string() {
    let props = emit::props! {
        span_parent: "0000000000000001",
    };

    assert_eq!(
        emit::SpanId::from_u64(1).unwrap(),
        props.pull::<emit::SpanId, _>("span_parent").unwrap()
    );
}

#[test]
fn props_capture_span_parent_as_non_span_id() {
    let props = emit::props! {
        #[emit::as_display(inspect: true)] span_parent: true,
    };

    assert_eq!(true, props.pull::<bool, _>("span_parent").unwrap());
}

#[test]
fn props_key() {
    let props = emit::props! {
        #[emit::key("not an identifier")] a: 1,
    };

    assert_eq!(1, props.pull::<i32, _>("not an identifier").unwrap());
}

#[test]
fn props_optional() {
    let props = emit::props! {
        #[emit::optional] some: Some(&1),
        #[emit::optional] none: None::<&i32>,
    };

    assert_eq!(1, props.pull::<i32, _>("some").unwrap());
    assert!(props.get("none").is_none());
}

#[test]
fn props_optional_ref() {
    let s = String::from("short lived");

    let some: Option<&str> = Some(&s);

    let props = emit::props! {
        #[emit::optional] some,
    };

    assert_eq!("short lived", props.pull::<&str, _>("some").unwrap());
}

#[test]
fn props_as_debug() {
    #[derive(Debug)]
    struct Data;

    let props = emit::props! {
        #[emit::as_debug] a: Data,
    };

    assert_eq!(format!("{:?}", Data), props.get("a").unwrap().to_string());
}

#[test]
fn props_as_display() {
    struct Data;

    impl fmt::Display for Data {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "Data")
        }
    }

    let props = emit::props! {
        #[emit::as_display] a: Data,
    };

    assert_eq!(format!("{}", Data), props.get("a").unwrap().to_string());
}

#[test]
#[cfg(feature = "std")]
fn props_ref() {
    let a = String::from("short lived");

    // NOTE: `a` worked inline on 2021 edition, but doesn't anymore
    // We've fixed this for macros like `emit!()` that don't return values
    match emit::props! {
        a,
    } {
        props => {
            assert_eq!("short lived", props.pull::<&str, _>("a").unwrap());
        }
    }

    drop(a);
}

#[test]
#[cfg(feature = "std")]
fn props_as_error() {
    use std::{error, io};

    let a = io::Error::new(io::ErrorKind::Other, "Some error");

    match emit::props! {
        #[emit::as_error] a,
    } {
        props => {
            let err = props.pull::<&(dyn error::Error + 'static), _>("a").unwrap();

            assert_eq!("Some error", err.to_string());
        }
    }
}

#[test]
fn props_as_value() {
    struct Data;

    impl emit::value::ToValue for Data {
        fn to_value(&self) -> emit::Value<'_> {
            "Data".to_value()
        }
    }

    let props = emit::props! {
        #[emit::as_value] data: Data,
        #[emit::as_value] some: Some(Data),
        #[emit::as_value] none: None::<Data>,
    };

    assert_eq!("Data", props.pull::<&str, _>("data").unwrap());
    assert_eq!("Data", props.pull::<&str, _>("some").unwrap());
    assert!(props.get("none").unwrap().is_null());
}

#[test]
#[cfg(feature = "sval")]
fn props_as_sval() {
    #[derive(Value)]
    struct Data {
        a: i32,
    }

    let props = emit::props! {
        #[emit::as_sval] a: Data { a: 42 },
    };

    assert_eq!(
        "{\"a\":42}",
        sval_json::stream_to_string(props.get("a").unwrap()).unwrap()
    );
}

#[test]
#[cfg(feature = "serde")]
fn props_as_serde() {
    #[derive(Serialize)]
    struct Data {
        a: i32,
    }

    let props = emit::props! {
        #[emit::as_serde] a: Data { a: 42 },
    };

    assert_eq!(
        "{\"a\":42}",
        serde_json::to_string(&props.get("a").unwrap()).unwrap()
    );
}
