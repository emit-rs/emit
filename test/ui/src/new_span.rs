use std::time::Duration;

use emit::{Emitter, Props};

use crate::util::{Called, simple_runtime};

#[test]
fn new_span_basic() {
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
            None => {
                let (mut guard, frame) = emit::new_span!(rt, "Hello, {user}");

                frame.call(move || {
                    guard.start();
                });
            }
            Some(emit::Level::Debug) => {
                let (mut guard, frame) = emit::new_debug_span!(rt, "Hello, {user}");

                frame.call(move || {
                    guard.start();
                });
            }
            Some(emit::Level::Info) => {
                let (mut guard, frame) = emit::new_info_span!(rt, "Hello, {user}");

                frame.call(move || {
                    guard.start();
                });
            }
            Some(emit::Level::Warn) => {
                let (mut guard, frame) = emit::new_warn_span!(rt, "Hello, {user}");

                frame.call(move || {
                    guard.start();
                });
            }
            Some(emit::Level::Error) => {
                let (mut guard, frame) = emit::new_error_span!(rt, "Hello, {user}");

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
async fn new_span_basic_async() {
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
            None => {
                let (mut guard, frame) = emit::new_span!(rt, "Hello, {user}");

                frame
                    .in_future(async move {
                        guard.start();

                        tokio::time::sleep(Duration::from_micros(1)).await;

                        guard.complete();
                    })
                    .await;
            }
            Some(emit::Level::Debug) => {
                let (mut guard, frame) = emit::new_debug_span!(rt, "Hello, {user}");

                frame
                    .in_future(async move {
                        guard.start();

                        tokio::time::sleep(Duration::from_micros(1)).await;

                        guard.complete();
                    })
                    .await;
            }
            Some(emit::Level::Info) => {
                let (mut guard, frame) = emit::new_info_span!(rt, "Hello, {user}");

                frame
                    .in_future(async move {
                        guard.start();

                        tokio::time::sleep(Duration::from_micros(1)).await;

                        guard.complete();
                    })
                    .await;
            }
            Some(emit::Level::Warn) => {
                let (mut guard, frame) = emit::new_warn_span!(rt, "Hello, {user}");

                frame
                    .in_future(async move {
                        guard.start();

                        tokio::time::sleep(Duration::from_micros(1)).await;

                        guard.complete();
                    })
                    .await;
            }
            Some(emit::Level::Error) => {
                let (mut guard, frame) = emit::new_error_span!(rt, "Hello, {user}");

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
