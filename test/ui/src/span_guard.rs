use ::std::time::Duration;

use emit::{Emitter, Props};

use crate::util::{Called, simple_runtime};

#[allow(unused_imports)]
use crate::shadow::*;

#[test]
fn span_guard_basic() {
    for lvl in [
        ::std::option::Option::Some(emit::Level::Debug),
        ::std::option::Option::Some(emit::Level::Info),
        ::std::option::Option::Some(emit::Level::Warn),
        ::std::option::Option::Some(emit::Level::Error),
        ::std::option::Option::None,
    ] {
        let called = Called::new();

        let rt = simple_runtime(
            |evt| {
                assert_eq!("Hello, Rust", evt.msg().to_string());
                assert_eq!("Hello, {user}", evt.tpl().to_string());
                assert_eq!(module_path!(), evt.mdl());

                assert!(evt.extent().is_some());

                assert_eq!("Rust", evt.props().pull::<&str, _>("user").unwrap());

                assert_eq!(lvl, evt.props().pull::<emit::Level, _>("lvl"));

                assert!(evt.props().get("trace_id").is_some());
                assert!(evt.props().get("span_id").is_some());

                called.record();
            },
            |_| true,
        );

        let user = "Rust";

        match lvl {
            ::std::option::Option::None => {
                let (mut guard, frame) = emit::span_guard!(rt, "Hello, {user}");

                frame.call(move || {
                    guard.start();
                });
            }
            ::std::option::Option::Some(emit::Level::Debug) => {
                let (mut guard, frame) = emit::debug_span_guard!(rt, "Hello, {user}");

                frame.call(move || {
                    guard.start();
                });
            }
            ::std::option::Option::Some(emit::Level::Info) => {
                let (mut guard, frame) = emit::info_span_guard!(rt, "Hello, {user}");

                frame.call(move || {
                    guard.start();
                });
            }
            ::std::option::Option::Some(emit::Level::Warn) => {
                let (mut guard, frame) = emit::warn_span_guard!(rt, "Hello, {user}");

                frame.call(move || {
                    guard.start();
                });
            }
            ::std::option::Option::Some(emit::Level::Error) => {
                let (mut guard, frame) = emit::error_span_guard!(rt, "Hello, {user}");

                frame.call(move || {
                    guard.start();
                });
            }
        }

        rt.emitter().blocking_flush(Duration::from_secs(1));

        assert!(called.was_called());
    }
}

#[tokio::test]
async fn span_guard_basic_async() {
    for lvl in [
        ::std::option::Option::Some(emit::Level::Debug),
        ::std::option::Option::Some(emit::Level::Info),
        ::std::option::Option::Some(emit::Level::Warn),
        ::std::option::Option::Some(emit::Level::Error),
        ::std::option::Option::None,
    ] {
        let called = Called::new();

        let rt = simple_runtime(
            |evt| {
                assert_eq!("Hello, Rust", evt.msg().to_string());
                assert_eq!("Hello, {user}", evt.tpl().to_string());
                assert_eq!(module_path!(), evt.mdl());

                assert!(evt.extent().is_some());

                assert_eq!("Rust", evt.props().pull::<&str, _>("user").unwrap());

                assert_eq!(lvl, evt.props().pull::<emit::Level, _>("lvl"));

                assert!(evt.props().get("trace_id").is_some());
                assert!(evt.props().get("span_id").is_some());

                called.record();
            },
            |_| true,
        );

        let user = "Rust";

        match lvl {
            ::std::option::Option::None => {
                let (mut guard, frame) = emit::span_guard!(rt, "Hello, {user}");

                frame
                    .in_future(async move {
                        guard.start();

                        tokio::time::sleep(Duration::from_micros(1)).await;

                        guard.complete();
                    })
                    .await;
            }
            ::std::option::Option::Some(emit::Level::Debug) => {
                let (mut guard, frame) = emit::debug_span_guard!(rt, "Hello, {user}");

                frame
                    .in_future(async move {
                        guard.start();

                        tokio::time::sleep(Duration::from_micros(1)).await;

                        guard.complete();
                    })
                    .await;
            }
            ::std::option::Option::Some(emit::Level::Info) => {
                let (mut guard, frame) = emit::info_span_guard!(rt, "Hello, {user}");

                frame
                    .in_future(async move {
                        guard.start();

                        tokio::time::sleep(Duration::from_micros(1)).await;

                        guard.complete();
                    })
                    .await;
            }
            ::std::option::Option::Some(emit::Level::Warn) => {
                let (mut guard, frame) = emit::warn_span_guard!(rt, "Hello, {user}");

                frame
                    .in_future(async move {
                        guard.start();

                        tokio::time::sleep(Duration::from_micros(1)).await;

                        guard.complete();
                    })
                    .await;
            }
            ::std::option::Option::Some(emit::Level::Error) => {
                let (mut guard, frame) = emit::error_span_guard!(rt, "Hello, {user}");

                frame
                    .in_future(async move {
                        guard.start();

                        tokio::time::sleep(Duration::from_micros(1)).await;

                        guard.complete();
                    })
                    .await;
            }
        }

        rt.emitter().blocking_flush(Duration::from_secs(1));

        assert!(called.was_called());
    }
}
