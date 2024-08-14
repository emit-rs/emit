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
fn props_capture_err() {
    todo!()
}

#[test]
fn props_capture_lvl() {
    todo!()
}

#[test]
fn props_capture_trace_id() {
    todo!()
}

#[test]
fn props_capture_span_id() {
    todo!()
}

#[test]
fn props_capture_span_parent() {
    todo!()
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
