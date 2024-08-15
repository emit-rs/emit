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
    todo!()
}

#[test]
fn props_optional() {
    todo!()
}

#[test]
fn props_as_debug() {
    todo!()
}

#[test]
fn props_as_display() {
    todo!()
}

#[test]
fn props_as_error() {
    todo!()
}

#[test]
fn props_as_value() {
    todo!()
}

#[test]
fn props_as_sval() {
    todo!()
}

#[test]
fn props_as_serde() {
    todo!()
}
