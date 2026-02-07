use emit::Props;

#[allow(unused_imports)]
use crate::shadow::*;

#[test]
fn event_basic() {
    let evt = emit::evt!(
        "Hello, {user}",
        user: "Rust",
    );

    assert_eq!("Hello, Rust", evt.msg().to_string());
    assert_eq!("Hello, {user}", evt.tpl().to_string());
    assert_eq!(module_path!(), evt.mdl());

    assert!(evt.extent().is_none());

    assert_eq!("Rust", evt.props().pull::<&str, _>("user").unwrap());
}

#[test]
fn event_mdl() {
    let evt = emit::evt!(
        mdl: emit::path!("x"),
        "template",
    );

    assert_eq!(emit::path!("x"), evt.mdl());
}

#[test]
fn event_extent() {
    let evt = emit::evt!(
        extent: emit::Timestamp::MIN,
        "template",
    );

    assert_eq!(emit::Timestamp::MIN, evt.ts().unwrap());
}

#[test]
fn event_base_props() {
    let props = emit::props! {
        a: "base",
    };

    let evt = emit::evt!(
        props,
        "template",
        b: "evt",
    );

    assert_eq!("base", evt.props().pull::<&str, _>("a").unwrap());
    assert_eq!("evt", evt.props().pull::<&str, _>("b").unwrap());
}
