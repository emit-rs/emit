use std::time::Duration;

use emit::{Emitter, Props};

use crate::util::{static_runtime, StaticCalled, StaticRuntime};

#[test]
fn span_basic() {
    // We need a static runtime so it's available in item position for the `#[span]` macro
    static CALLED: StaticCalled = StaticCalled::new();
    static RT: StaticRuntime = static_runtime(
        |evt| {
            assert_eq!("greet Rust", evt.msg().to_string());
            assert_eq!("greet {user}", evt.tpl().to_string());
            assert_eq!(module_path!(), evt.module());

            assert!(evt.extent().unwrap().is_span());
            assert!(evt
                .props()
                .pull::<emit::span::TraceId, _>("trace_id")
                .is_some());
            assert!(evt
                .props()
                .pull::<emit::span::SpanId, _>("span_id")
                .is_some());

            CALLED.record();
        },
        |_| true,
    );

    #[emit::span(rt: RT, "greet {user}")]
    fn exec(user: &str) {
        let _ = user;
    }

    exec("Rust");

    RT.emitter().blocking_flush(Duration::from_secs(1));

    assert!(CALLED.was_called());
}

#[test]
fn span_guard() {
    /*
    Complete the guard by dropping
    */
    todo!()
}

#[test]
fn span_ok_lvl() {
    /*
    Autocomplete a result span
    */
    todo!()
}

#[test]
fn span_ok_err_lvl() {
    /*
    Autocomplete a result span
    */
    todo!()
}

#[test]
fn span_props_precedence() {
    /*
    Ensure event props override span props
    */
    todo!()
}
