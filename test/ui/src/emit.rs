use std::time::Duration;

use emit::{Emitter, Props};

use crate::util::{simple_runtime, Called};

#[test]
fn emit_basic() {
    for lvl in [
        Some(emit::Level::Debug),
        Some(emit::Level::Info),
        Some(emit::Level::Warn),
        Some(emit::Level::Error),
        None,
    ] {
        let called = Called::new();

        let rt = simple_runtime(
            |evt| {
                assert_eq!("Hello, Rust", evt.msg().to_string());
                assert_eq!("Hello, {user}", evt.tpl().to_string());
                assert_eq!(module_path!(), evt.module());

                assert!(evt.extent().is_some());

                assert_eq!("Rust", evt.props().pull::<&str, _>("user").unwrap());

                assert_eq!(lvl, evt.props().pull::<emit::Level, _>("lvl"));

                called.record();
            },
            |_| true,
        );

        let user = "Rust";

        match lvl {
            None => emit::emit!(rt, "Hello, {user}"),
            Some(emit::Level::Debug) => emit::debug!(rt, "Hello, {user}"),
            Some(emit::Level::Info) => emit::info!(rt, "Hello, {user}"),
            Some(emit::Level::Warn) => emit::warn!(rt, "Hello, {user}"),
            Some(emit::Level::Error) => emit::error!(rt, "Hello, {user}"),
        }

        rt.emitter().blocking_flush(Duration::from_secs(1));

        assert!(called.was_called());
    }
}

#[test]
fn emit_filter() {
    let called = Called::new();
    let rt = simple_runtime(|_| called.record(), |evt| evt.module() == "true");

    emit::emit!(rt, module: emit::path!("false"), "test");
    emit::emit!(rt, module: emit::path!("true"), "test");

    rt.emitter().blocking_flush(Duration::from_secs(1));

    assert_eq!(1, called.called_times());
}

#[test]
fn emit_when() {
    let called = Called::new();
    let rt = simple_runtime(|_| called.record(), |evt| evt.module() == "true");

    emit::emit!(rt, when: emit::filter::from_fn(|_| true), module: emit::path!("false"), "test");

    rt.emitter().blocking_flush(Duration::from_secs(1));

    assert!(called.was_called());
}

#[test]
fn emit_extent_point() {
    let rt = simple_runtime(
        |evt| {
            assert_eq!(
                emit::Timestamp::from_unix(Duration::from_secs(42)).unwrap(),
                evt.extent().unwrap().as_point()
            );
        },
        |_| true,
    );

    emit::emit!(
        rt,
        extent: emit::Timestamp::from_unix(Duration::from_secs(42)),
        "test",
    );
}

#[test]
fn emit_extent_span() {
    let rt = simple_runtime(
        |evt| {
            assert_eq!(
                emit::Timestamp::from_unix(Duration::from_secs(42)).unwrap()
                    ..emit::Timestamp::from_unix(Duration::from_secs(47)).unwrap(),
                evt.extent().unwrap().as_span().unwrap().clone()
            );
        },
        |_| true,
    );

    emit::emit!(
        rt,
        extent: emit::Timestamp::from_unix(Duration::from_secs(42))..emit::Timestamp::from_unix(Duration::from_secs(47)),
        "test",
    );
}

#[test]
fn emit_module() {
    let rt = simple_runtime(
        |evt| {
            assert_eq!("custom_module", evt.module());
        },
        |_| true,
    );

    emit::emit!(rt, module: emit::path!("custom_module"), "test");
}

#[test]
fn emit_props() {
    let rt = simple_runtime(
        |evt| {
            assert_eq!(1, evt.props().pull::<i32, _>("ambient_prop1").unwrap());
            assert_eq!(2, evt.props().pull::<i32, _>("ambient_prop2").unwrap());

            assert_eq!(1, evt.props().pull::<i32, _>("evt_prop1").unwrap());
            assert_eq!(2, evt.props().pull::<i32, _>("evt_prop2").unwrap());
        },
        |_| true,
    );

    emit::emit!(
        rt,
        props: emit::props! {
            ambient_prop1: 1,
            ambient_prop2: 2,
        },
        "test",
        evt_prop1: 1,
        evt_prop2: 2,
    );
}

#[test]
fn emit_event() {
    let rt = simple_runtime(
        |evt| {
            assert_eq!("Hello, Rust", evt.msg().to_string());
            assert_eq!("Hello, {user}", evt.tpl().to_string());
            assert_eq!(module_path!(), evt.module());

            assert!(evt.extent().is_some());

            assert_eq!("Rust", evt.props().pull::<&str, _>("user").unwrap());
        },
        |_| true,
    );

    emit::emit!(
        rt,
        evt: emit::event!(
            "Hello, {user}",
            user: "Rust",
        ),
    );
}

#[test]
fn emit_event_filter() {
    let called = Called::new();
    let rt = simple_runtime(|_| called.record(), |evt| evt.module() == "true");

    emit::emit!(rt, evt: emit::event!(module: emit::path!("false"), "test"));
    emit::emit!(rt, evt: emit::event!(module: emit::path!("true"), "test"));

    rt.emitter().blocking_flush(Duration::from_secs(1));

    assert_eq!(1, called.called_times());
}

#[test]
fn emit_event_when() {
    let called = Called::new();
    let rt = simple_runtime(|_| called.record(), |evt| evt.module() == "true");

    emit::emit!(rt, when: emit::filter::from_fn(|_| true), evt: emit::event!(module: emit::path!("false"), "test"));

    rt.emitter().blocking_flush(Duration::from_secs(1));

    assert!(called.was_called());
}

#[test]
fn emit_props_precedence() {
    let rt = simple_runtime(
        |evt| {
            assert_eq!(
                "evt",
                evt.props().pull::<&str, _>("ctxt_props_evt").unwrap()
            );
            assert_eq!("props", evt.props().pull::<&str, _>("ctxt_props").unwrap());
            assert_eq!("ctxt", evt.props().pull::<&str, _>("ctxt").unwrap());

            assert_eq!("evt", evt.props().pull::<&str, _>("props_evt").unwrap());
            assert_eq!("props", evt.props().pull::<&str, _>("props").unwrap());

            assert_eq!("evt", evt.props().pull::<&str, _>("evt").unwrap());
        },
        |_| true,
    );

    emit::Frame::push(
        rt.ctxt(),
        emit::props! {
            ctxt_props_evt: "ctxt",
            ctxt_props: "ctxt",
            ctxt: "ctxt",
        },
    )
    .call(|| {
        emit::emit!(
            rt,
            props: emit::props! {
                ctxt_props_evt: "props",
                ctxt_props: "props",
                props_evt: "props",
                props: "props",
            },
            "test",
            ctxt_props_evt: "evt",
            props_evt: "evt",
            evt: "evt",
        );
    });
}
