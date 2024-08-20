use std::fmt;

use emit::Props;

#[test]
fn props_basic() {
    match emit::props! {
        b: 1,
        a: true,
        c: 2.0,
        d: "text",
    } {
        props => {
            assert!(props.is_unique());

            assert_eq!(1, props.pull::<i32, _>("b").unwrap());
            assert_eq!(true, props.pull::<bool, _>("a").unwrap());
            assert_eq!(2.0, props.pull::<f64, _>("c").unwrap());
            assert_eq!("text", props.pull::<&str, _>("d").unwrap());
        }
    }
}

#[test]
fn props_cfg() {
    match emit::props! {
        #[cfg(not(emit_disabled))]
        enabled: "enabled",
        #[cfg(emit_disabled)]
        disabled: "disabled",
    } {
        props => {
            assert_eq!("enabled", props.pull::<&str, _>("enabled").unwrap());
            assert!(props.get("disabled").is_none());
        }
    }
}

#[test]
#[cfg(feature = "std")]
fn props_capture_err() {
    use std::{error, io};

    match emit::props! {
        err: io::Error::new(io::ErrorKind::Other, "Some error"),
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
    match emit::props! {
        err: "Some error",
    } {
        props => {
            let err = props.pull::<&str, _>("err").unwrap();

            assert_eq!("Some error", err);
        }
    }
}

#[test]
fn props_capture_err_as_non_err() {
    match emit::props! {
        #[emit::as_display(inspect: true)] err: true,
    } {
        props => {
            let err = props.pull::<bool, _>("err").unwrap();

            assert_eq!(true, err);
        }
    }
}

#[test]
fn props_capture_lvl() {
    match emit::props! {
        lvl: emit::Level::Info,
    } {
        props => {
            assert_eq!(
                emit::Level::Info,
                props.pull::<emit::Level, _>("lvl").unwrap()
            );
        }
    }
}

#[test]
fn props_capture_lvl_string() {
    match emit::props! {
        lvl: "info",
    } {
        props => {
            assert_eq!(
                emit::Level::Info,
                props.pull::<emit::Level, _>("lvl").unwrap()
            );
        }
    }
}

#[test]
fn props_capture_lvl_as_non_lvl() {
    match emit::props! {
        #[emit::as_display(inspect: true)] lvl: true,
    } {
        props => {
            assert_eq!(true, props.pull::<bool, _>("lvl").unwrap());
        }
    }
}

#[test]
fn props_capture_trace_id() {
    match emit::props! {
        trace_id: emit::span::TraceId::from_u128(1),
    } {
        props => {
            assert_eq!(
                emit::span::TraceId::from_u128(1).unwrap(),
                props.pull::<emit::span::TraceId, _>("trace_id").unwrap()
            );
        }
    }
}

#[test]
fn props_capture_trace_id_string() {
    match emit::props! {
        trace_id: "00000000000000000000000000000001",
    } {
        props => {
            assert_eq!(
                emit::span::TraceId::from_u128(1).unwrap(),
                props.pull::<emit::span::TraceId, _>("trace_id").unwrap()
            );
        }
    }
}

#[test]
fn props_capture_trace_id_as_non_trace_id() {
    match emit::props! {
        #[emit::as_display(inspect: true)] trace_id: true,
    } {
        props => {
            assert_eq!(true, props.pull::<bool, _>("trace_id").unwrap());
        }
    }
}

#[test]
fn props_capture_span_id() {
    match emit::props! {
        span_id: emit::span::SpanId::from_u64(1),
    } {
        props => {
            assert_eq!(
                emit::span::SpanId::from_u64(1).unwrap(),
                props.pull::<emit::span::SpanId, _>("span_id").unwrap()
            );
        }
    }
}

#[test]
fn props_capture_span_id_string() {
    match emit::props! {
        span_id: "0000000000000001",
    } {
        props => {
            assert_eq!(
                emit::span::SpanId::from_u64(1).unwrap(),
                props.pull::<emit::span::SpanId, _>("span_id").unwrap()
            );
        }
    }
}

#[test]
fn props_capture_span_id_as_non_span_id() {
    match emit::props! {
        #[emit::as_display(inspect: true)] span_id: true,
    } {
        props => {
            assert_eq!(true, props.pull::<bool, _>("span_id").unwrap());
        }
    }
}

#[test]
fn props_capture_span_parent() {
    match emit::props! {
        span_parent: emit::span::SpanId::from_u64(1),
    } {
        props => {
            assert_eq!(
                emit::span::SpanId::from_u64(1).unwrap(),
                props.pull::<emit::span::SpanId, _>("span_parent").unwrap()
            );
        }
    }
}

#[test]
fn props_capture_span_parent_string() {
    match emit::props! {
        span_parent: "0000000000000001",
    } {
        props => {
            assert_eq!(
                emit::span::SpanId::from_u64(1).unwrap(),
                props.pull::<emit::span::SpanId, _>("span_parent").unwrap()
            );
        }
    }
}

#[test]
fn props_capture_span_parent_as_non_span_id() {
    match emit::props! {
        #[emit::as_display(inspect: true)] span_parent: true,
    } {
        props => {
            assert_eq!(true, props.pull::<bool, _>("span_parent").unwrap());
        }
    }
}

#[test]
fn props_key() {
    match emit::props! {
        #[emit::key("not an identifier")] a: 1,
    } {
        props => {
            assert_eq!(1, props.pull::<i32, _>("not an identifier").unwrap());
        }
    }
}

#[test]
fn props_optional() {
    match emit::props! {
        #[emit::optional] some: Some(1),
        #[emit::optional] none: None::<i32>,
    } {
        props => {
            assert_eq!(1, props.pull::<i32, _>("some").unwrap());
            assert!(props.get("none").is_none());
        }
    }
}

#[test]
fn props_as_debug() {
    #[derive(Debug)]
    struct Data;

    match emit::props! {
        #[emit::as_debug] a: Data,
    } {
        props => {
            assert_eq!(format!("{:?}", Data), props.get("a").unwrap().to_string());
        }
    }
}

#[test]
fn props_as_display() {
    struct Data;

    impl fmt::Display for Data {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "Data")
        }
    }

    match emit::props! {
        #[emit::as_display] a: Data,
    } {
        props => {
            assert_eq!(format!("{}", Data), props.get("a").unwrap().to_string());
        }
    }
}

#[test]
#[cfg(feature = "std")]
fn props_as_error() {
    use std::{error, io};

    match emit::props! {
        #[emit::as_error] a: io::Error::new(io::ErrorKind::Other, "Some error"),
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
        fn to_value(&self) -> emit::Value {
            "Data".to_value()
        }
    }

    match emit::props! {
        #[emit::as_value] data: Data,
        #[emit::as_value] some: Some(Data),
        #[emit::as_value] none: None::<Data>,
    } {
        props => {
            assert_eq!("Data", props.pull::<&str, _>("data").unwrap());
            assert_eq!("Data", props.pull::<&str, _>("some").unwrap());
            assert!(props.get("none").unwrap().is_null());
        }
    }
}

#[test]
#[cfg(feature = "sval")]
fn props_as_sval() {
    #[derive(Value)]
    struct Data {
        a: i32,
    }

    match emit::props! {
        #[emit::as_sval] a: Data { a: 42 },
    } {
        props => {
            assert_eq!(
                "{\"a\":42}",
                sval_json::stream_to_string(props.get("a").unwrap()).unwrap()
            );
        }
    }
}

#[test]
#[cfg(feature = "serde")]
fn props_as_serde() {
    #[derive(Serialize)]
    struct Data {
        a: i32,
    }

    match emit::props! {
        #[emit::as_serde] a: Data { a: 42 },
    } {
        props => {
            assert_eq!(
                "{\"a\":42}",
                serde_json::to_string(&props.get("a").unwrap()).unwrap()
            );
        }
    }
}