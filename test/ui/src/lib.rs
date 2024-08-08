#![cfg(test)]

mod util;

use std::{sync::LazyLock, time::Duration};

use emit::{Emitter, Props};

use crate::util::{simple_runtime, SimpleRuntime};
use util::Called;

#[test]
fn emit_basic() {
    static CALLED: LazyLock<Called> = LazyLock::new(Called::new);
    let rt = simple_runtime(
        |evt| {
            assert_eq!("Hello, Rust", evt.msg().to_string());
            assert_eq!("Hello, {user}", evt.tpl().to_string());

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
fn span_basic() {
    // We need a static runtime so it's available in item position for the `#[span]` macro
    static CALLED: LazyLock<Called> = LazyLock::new(Called::new);
    static RT: SimpleRuntime = simple_runtime(
        |evt| {
            assert_eq!("greet Rust", evt.msg().to_string());
            assert_eq!("greet {user}", evt.tpl().to_string());

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
