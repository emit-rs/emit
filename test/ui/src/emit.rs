use std::time::Duration;

use emit::Emitter;

use crate::util::{simple_runtime, StaticCalled};

#[test]
fn emit_basic() {
    static CALLED: StaticCalled = StaticCalled::new();
    let rt = simple_runtime(
        |evt| {
            assert_eq!("Hello, Rust", evt.msg().to_string());
            assert_eq!("Hello, {user}", evt.tpl().to_string());
            assert_eq!(module_path!(), evt.module());

            assert!(evt.extent().is_some());

            CALLED.record();
        },
        |_| true,
    );

    let user = "Rust";

    emit::emit!(rt, "Hello, {user}");

    rt.emitter().blocking_flush(Duration::from_secs(1));

    assert!(CALLED.was_called());
}

#[test]
fn emit_filter() {
    static CALLED: StaticCalled = StaticCalled::new();
    let rt = simple_runtime(|_| CALLED.record(), |evt| evt.module() == "true");

    emit::emit!(rt, module: emit::path!("false"), "test");
    emit::emit!(rt, module: emit::path!("true"), "test");

    rt.emitter().blocking_flush(Duration::from_secs(1));

    assert_eq!(1, CALLED.called_times());
}

#[test]
fn emit_when() {
    static CALLED: StaticCalled = StaticCalled::new();
    let rt = simple_runtime(|_| CALLED.record(), |evt| evt.module() == "true");

    emit::emit!(rt, when: emit::filter::from_fn(|_| true), module: emit::path!("false"), "test");

    rt.emitter().blocking_flush(Duration::from_secs(1));

    assert_eq!(1, CALLED.called_times());
}

#[test]
fn emit_module() {
    static CALLED: StaticCalled = StaticCalled::new();
    let rt = simple_runtime(
        |evt| {
            assert_eq!("custom_module", evt.module());

            CALLED.record();
        },
        |_| true,
    );

    emit::emit!(rt, module: emit::path!("custom_module"), "test");

    rt.emitter().blocking_flush(Duration::from_secs(1));

    assert!(CALLED.was_called());
}
