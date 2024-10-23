use std::time::Duration;

use emit::{Ctxt, Emitter, Props};

use crate::util::{static_runtime, StaticCalled, StaticRuntime};

#[test]
fn span_basic() {
    fn assert_event_base(evt: &emit::Event<impl Props>) {
        assert_eq!("greet Rust", evt.msg().to_string());
        assert_eq!("greet {user}", evt.tpl().to_string());
        assert_eq!(module_path!(), evt.mdl());

        assert!(evt.props().pull::<&str, _>("user").is_some());

        assert_eq!(
            "greet {user}",
            evt.props().pull::<&str, _>("span_name").unwrap()
        );

        assert_eq!(
            emit::Kind::Span,
            evt.props().pull::<emit::Kind, _>("evt_kind").unwrap()
        );

        assert!(evt.props().pull::<emit::TraceId, _>("trace_id").is_some());
        assert!(evt.props().pull::<emit::SpanId, _>("span_id").is_some());
    }

    fn assert_event(evt: &emit::Event<impl Props>) {
        assert_event_base(evt);

        assert!(evt.extent().unwrap().is_range());
    }

    static CALLED: StaticCalled = StaticCalled::new();
    static RT: StaticRuntime = static_runtime(
        |evt| {
            assert_event(evt);

            CALLED.record();
        },
        |evt| {
            assert_event_base(evt);

            true
        },
    );

    static DEBUG_CALLED: StaticCalled = StaticCalled::new();
    static DEBUG_RT: StaticRuntime = static_runtime(
        |evt| {
            assert_event(evt);

            assert_eq!(
                emit::Level::Debug,
                evt.props().pull::<emit::Level, _>("lvl").unwrap()
            );

            DEBUG_CALLED.record();
        },
        |evt| {
            assert_event_base(evt);

            true
        },
    );

    static INFO_CALLED: StaticCalled = StaticCalled::new();
    static INFO_RT: StaticRuntime = static_runtime(
        |evt| {
            assert_event(evt);

            assert_eq!(
                emit::Level::Info,
                evt.props().pull::<emit::Level, _>("lvl").unwrap()
            );

            INFO_CALLED.record();
        },
        |evt| {
            assert_event_base(evt);

            true
        },
    );

    static WARN_CALLED: StaticCalled = StaticCalled::new();
    static WARN_RT: StaticRuntime = static_runtime(
        |evt| {
            assert_event(evt);

            assert_eq!(
                emit::Level::Warn,
                evt.props().pull::<emit::Level, _>("lvl").unwrap()
            );

            WARN_CALLED.record();
        },
        |evt| {
            assert_event_base(evt);

            true
        },
    );

    static ERROR_CALLED: StaticCalled = StaticCalled::new();
    static ERROR_RT: StaticRuntime = static_runtime(
        |evt| {
            assert_event(evt);

            assert_eq!(
                emit::Level::Error,
                evt.props().pull::<emit::Level, _>("lvl").unwrap()
            );

            ERROR_CALLED.record();
        },
        |evt| {
            assert_event_base(evt);

            true
        },
    );

    #[emit::span(rt: RT, "greet {user}")]
    fn exec(user: &str) {
        RT.ctxt().with_current(|props| {
            assert_eq!(user, props.pull::<&str, _>("user").unwrap());
        });

        let _ = user;
    }

    #[emit::debug_span(rt: DEBUG_RT, "greet {user}")]
    fn exec_debug(user: &str) {
        DEBUG_RT.ctxt().with_current(|props| {
            assert_eq!(user, props.pull::<&str, _>("user").unwrap());
        });

        let _ = user;
    }

    #[emit::info_span(rt: INFO_RT, "greet {user}")]
    fn exec_info(user: &str) {
        INFO_RT.ctxt().with_current(|props| {
            assert_eq!(user, props.pull::<&str, _>("user").unwrap());
        });

        let _ = user;
    }

    #[emit::warn_span(rt: WARN_RT, "greet {user}")]
    fn exec_warn(user: &str) {
        WARN_RT.ctxt().with_current(|props| {
            assert_eq!(user, props.pull::<&str, _>("user").unwrap());
        });

        let _ = user;
    }

    #[emit::error_span(rt: ERROR_RT, "greet {user}")]
    fn exec_error(user: &str) {
        ERROR_RT.ctxt().with_current(|props| {
            assert_eq!(user, props.pull::<&str, _>("user").unwrap());
        });

        let _ = user;
    }

    exec("Rust");
    exec_debug("Rust");
    exec_info("Rust");
    exec_warn("Rust");
    exec_error("Rust");

    RT.emitter().blocking_flush(Duration::from_secs(1));
    DEBUG_RT.emitter().blocking_flush(Duration::from_secs(1));
    INFO_RT.emitter().blocking_flush(Duration::from_secs(1));
    WARN_RT.emitter().blocking_flush(Duration::from_secs(1));
    ERROR_RT.emitter().blocking_flush(Duration::from_secs(1));

    assert!(CALLED.was_called());
    assert!(DEBUG_CALLED.was_called());
    assert!(INFO_CALLED.was_called());
    assert!(WARN_CALLED.was_called());
    assert!(ERROR_CALLED.was_called());
}

#[tokio::test]
async fn span_basic_async() {
    static CALLED: StaticCalled = StaticCalled::new();
    static RT: StaticRuntime = static_runtime(
        |evt| {
            assert_eq!("greet Rust", evt.msg().to_string());
            assert_eq!("greet {user}", evt.tpl().to_string());
            assert_eq!(module_path!(), evt.mdl());

            assert!(evt.extent().unwrap().is_range());
            assert!(evt.props().pull::<emit::TraceId, _>("trace_id").is_some());
            assert!(evt.props().pull::<emit::SpanId, _>("span_id").is_some());

            CALLED.record();
        },
        |_| true,
    );

    #[emit::span(rt: RT, "greet {user}")]
    async fn exec(user: &str) {
        let _ = user;
    }

    exec("Rust").await;

    RT.emitter().blocking_flush(Duration::from_secs(1));

    assert!(CALLED.was_called());
}

#[test]
fn span_rt_ref() {
    static CALLED: StaticCalled = StaticCalled::new();
    static RT: StaticRuntime = static_runtime(
        |_| {
            CALLED.record();
        },
        |_| true,
    );

    #[emit::span(rt: &RT, "test")]
    fn exec() {}

    exec();

    RT.emitter().blocking_flush(Duration::from_secs(1));

    assert_eq!(1, CALLED.called_times());
}

#[test]
fn span_by_value_arg() {
    static RT: StaticRuntime = static_runtime(|_| {}, |_| true);

    fn take_string(_: String) {}

    #[emit::span(rt: &RT, "test")]
    fn exec(arg: String) {
        take_string(arg);
    }

    exec("Owned".to_owned());

    RT.emitter().blocking_flush(Duration::from_secs(1));
}

#[tokio::test]
async fn async_span_by_value_arg() {
    static RT: StaticRuntime = static_runtime(|_| {}, |_| true);

    fn take_string(_: String) {}

    #[emit::span(rt: &RT, "test")]
    async fn exec(arg: String) {
        take_string(arg);
    }

    exec("Owned".to_owned()).await;

    RT.emitter().blocking_flush(Duration::from_secs(1));
}

#[test]
fn span_guard() {
    static CALLED: StaticCalled = StaticCalled::new();
    static RT: StaticRuntime = static_runtime(
        |_| {
            CALLED.record();
        },
        |_| true,
    );

    #[emit::span(rt: RT, guard: span, "test")]
    fn exec() {
        let span: emit::span::SpanGuard<_, _, _> = span;
        span.complete();
    }

    exec();

    assert!(CALLED.was_called());
}

#[test]
fn span_mdl() {
    static RT: StaticRuntime = static_runtime(
        |evt| {
            assert_eq!("custom_module", evt.mdl());
        },
        |evt| {
            assert_eq!("custom_module", evt.mdl());

            true
        },
    );

    #[emit::span(rt: RT, mdl: emit::path!("custom_module"), "test")]
    fn exec() {}

    exec();

    RT.emitter().blocking_flush(Duration::from_secs(1));
}

#[test]
fn span_filter() {
    static CALLED: StaticCalled = StaticCalled::new();
    static RT: StaticRuntime = static_runtime(|_| CALLED.record(), |evt| evt.mdl() == "true");

    #[emit::span(rt: RT, mdl: emit::path!("true"), "test")]
    fn exec_true() {}

    #[emit::span(rt: RT, mdl: emit::path!("false"), "test")]
    fn exec_false() {}

    exec_true();
    exec_false();

    RT.emitter().blocking_flush(Duration::from_secs(1));

    assert_eq!(1, CALLED.called_times());
}

#[test]
fn span_when() {
    static CALLED: StaticCalled = StaticCalled::new();
    static RT: StaticRuntime = static_runtime(|_| CALLED.record(), |evt| evt.mdl() == "tralse");

    #[emit::span(rt: RT, when: emit::filter::from_fn(|evt| evt.mdl() == "false"), mdl: emit::path!("true"), "test")]
    fn exec_true() {}

    #[emit::span(rt: RT, when: emit::filter::from_fn(|evt| evt.mdl() == "false"), mdl: emit::path!("false"), "test")]
    fn exec_false() {}

    exec_true();
    exec_false();

    RT.emitter().blocking_flush(Duration::from_secs(1));

    assert_eq!(1, CALLED.called_times());
}

#[test]
fn span_explicit_ids() {
    static CALLED: StaticCalled = StaticCalled::new();
    static RT: StaticRuntime = static_runtime(
        |evt| {
            assert_eq!(emit::TraceId::from_u128(1), evt.props().pull("trace_id"));
            assert_eq!(emit::SpanId::from_u64(2), evt.props().pull("span_parent"));
            assert_eq!(emit::SpanId::from_u64(3), evt.props().pull("span_id"));

            CALLED.record();
        },
        |evt| {
            assert_eq!(emit::TraceId::from_u128(1), evt.props().pull("trace_id"));
            assert_eq!(emit::SpanId::from_u64(2), evt.props().pull("span_parent"));
            assert_eq!(emit::SpanId::from_u64(3), evt.props().pull("span_id"));

            true
        },
    );

    #[emit::span(rt: RT, "test", trace_id, span_parent, span_id)]
    fn exec(trace_id: &str, span_parent: &str, span_id: &str) {
        let ctxt = emit::SpanCtxt::current(RT.ctxt());

        assert_eq!(emit::TraceId::from_u128(1), ctxt.trace_id().copied());
        assert_eq!(emit::SpanId::from_u64(2), ctxt.span_parent().copied());
        assert_eq!(emit::SpanId::from_u64(3), ctxt.span_id().copied());
    }

    exec(
        "00000000000000000000000000000001",
        "0000000000000002",
        "0000000000000003",
    );

    assert!(CALLED.was_called());
}

#[test]
fn span_explicit_ids_ctxt() {
    static CALLED: StaticCalled = StaticCalled::new();
    static RT: StaticRuntime = static_runtime(
        |evt| {
            assert_eq!(emit::TraceId::from_u128(1), evt.props().pull("trace_id"));
            assert_eq!(emit::SpanId::from_u64(2), evt.props().pull("span_parent"));
            assert_eq!(emit::SpanId::from_u64(3), evt.props().pull("span_id"));

            CALLED.record();
        },
        |evt| {
            assert_eq!(emit::TraceId::from_u128(1), evt.props().pull("trace_id"));
            assert_eq!(emit::SpanId::from_u64(2), evt.props().pull("span_parent"));
            assert_eq!(emit::SpanId::from_u64(3), evt.props().pull("span_id"));

            true
        },
    );

    #[emit::span(rt: RT, "test", trace_id: ctxt.trace_id(), span_parent: ctxt.span_parent(), span_id: ctxt.span_id())]
    fn exec(ctxt: emit::SpanCtxt) {
        let current = emit::SpanCtxt::current(RT.ctxt());

        assert_eq!(ctxt, current);
    }

    exec(emit::SpanCtxt::new(
        emit::TraceId::from_u128(1),
        emit::SpanId::from_u64(2),
        emit::SpanId::from_u64(3),
    ));

    assert!(CALLED.was_called());
}

#[test]
#[cfg(feature = "std")]
fn span_ok_lvl() {
    use std::io;

    static OK_RT: StaticRuntime = static_runtime(
        |evt| {
            assert_eq!(
                emit::Level::Info,
                evt.props().pull::<emit::Level, _>("lvl").unwrap()
            );
        },
        |evt| {
            assert!(evt.props().get("lvl").is_none());

            true
        },
    );

    static ERR_RT: StaticRuntime = static_runtime(
        |evt| {
            assert_eq!(
                "failed",
                evt.props()
                    .pull::<&(dyn std::error::Error + 'static), _>("err")
                    .unwrap()
                    .to_string()
            );
            assert_eq!(Some(emit::Level::Error), evt.props().pull("lvl"));
        },
        |evt| {
            assert!(evt.props().get("lvl").is_none());

            true
        },
    );

    #[emit::span(rt: OK_RT, ok_lvl: emit::Level::Info, "test")]
    fn exec_ok() -> Result<bool, io::Error> {
        Ok(true)
    }

    #[emit::span(rt: ERR_RT, ok_lvl: emit::Level::Info, "test")]
    fn exec_err() -> Result<bool, io::Error> {
        Err(io::Error::new(io::ErrorKind::Other, "failed"))
    }

    assert!(exec_ok().unwrap());
    exec_err().unwrap_err();
}

#[test]
#[cfg(feature = "std")]
fn span_err_lvl() {
    use std::io;

    static OK_RT: StaticRuntime = static_runtime(
        |evt| {
            assert!(evt.props().get("lvl").is_none());
        },
        |_| true,
    );

    static ERR_RT: StaticRuntime = static_runtime(
        |evt| {
            assert_eq!(
                "failed",
                evt.props()
                    .pull::<&(dyn std::error::Error + 'static), _>("err")
                    .unwrap()
                    .to_string()
            );
            assert_eq!(
                emit::Level::Warn,
                evt.props().pull::<emit::Level, _>("lvl").unwrap()
            );
        },
        |_| true,
    );

    #[emit::span(rt: OK_RT, err_lvl: emit::Level::Warn, "test")]
    fn exec_ok() -> Result<(), io::Error> {
        Ok(())
    }

    #[emit::span(rt: ERR_RT, err_lvl: emit::Level::Warn, "test")]
    fn exec_err() -> Result<(), io::Error> {
        Err(io::Error::new(io::ErrorKind::Other, "failed"))
    }

    exec_ok().unwrap();
    exec_err().unwrap_err();
}

#[test]
#[cfg(feature = "std")]
fn span_err_lvl_explicit_return() {
    use std::io;

    static RT: StaticRuntime = static_runtime(
        |evt| {
            assert_eq!(
                "failed",
                evt.props()
                    .pull::<&(dyn std::error::Error + 'static), _>("err")
                    .unwrap()
                    .to_string()
            );
            assert_eq!(
                emit::Level::Warn,
                evt.props().pull::<emit::Level, _>("lvl").unwrap()
            );
        },
        |_| true,
    );

    #[emit::span(rt: RT, err_lvl: emit::Level::Warn, "test")]
    fn exec(fail: bool) -> Result<bool, io::Error> {
        if fail {
            return Err(io::Error::new(io::ErrorKind::Other, "failed"));
        }

        Ok(true)
    }

    exec(true).unwrap_err();
}

#[tokio::test]
#[cfg(feature = "std")]
async fn span_err_lvl_explicit_return_async() {
    use std::io;

    static RT: StaticRuntime = static_runtime(
        |evt| {
            assert_eq!(
                "failed",
                evt.props()
                    .pull::<&(dyn std::error::Error + 'static), _>("err")
                    .unwrap()
                    .to_string()
            );
            assert_eq!(
                emit::Level::Warn,
                evt.props().pull::<emit::Level, _>("lvl").unwrap()
            );
        },
        |_| true,
    );

    #[emit::span(rt: RT, err_lvl: emit::Level::Warn, "test")]
    async fn exec(fail: bool) -> Result<bool, io::Error> {
        if fail {
            return Err(io::Error::new(io::ErrorKind::Other, "failed"));
        }

        Ok(true)
    }

    exec(true).await.unwrap_err();
}

#[test]
#[cfg(feature = "std")]
fn span_err_lvl_impl_return() {
    use std::io;

    static RT: StaticRuntime = static_runtime(
        |evt| {
            assert_eq!(
                "failed",
                evt.props()
                    .pull::<&(dyn std::error::Error + 'static), _>("err")
                    .unwrap()
                    .to_string()
            );
            assert_eq!(
                emit::Level::Warn,
                evt.props().pull::<emit::Level, _>("lvl").unwrap()
            );
        },
        |_| true,
    );

    #[emit::span(rt: RT, err_lvl: emit::Level::Warn, "test")]
    fn exec(fail: bool) -> Result<bool, impl std::error::Error + 'static> {
        if fail {
            return Err(io::Error::new(io::ErrorKind::Other, "failed"));
        }

        Ok(true)
    }

    exec(true).unwrap_err();
}

#[tokio::test]
#[cfg(feature = "std")]
async fn span_err_lvl_impl_return_async() {
    use std::io;

    static RT: StaticRuntime = static_runtime(
        |evt| {
            assert_eq!(
                "failed",
                evt.props()
                    .pull::<&(dyn std::error::Error + 'static), _>("err")
                    .unwrap()
                    .to_string()
            );
            assert_eq!(
                emit::Level::Warn,
                evt.props().pull::<emit::Level, _>("lvl").unwrap()
            );
        },
        |_| true,
    );

    #[emit::span(rt: RT, err_lvl: emit::Level::Warn, "test")]
    async fn exec(fail: bool) -> Result<bool, impl std::error::Error + 'static> {
        if fail {
            return Err(io::Error::new(io::ErrorKind::Other, "failed"));
        }

        Ok(true)
    }

    exec(true).await.unwrap_err();
}

#[test]
#[cfg(feature = "std")]
fn span_err() {
    use std::io;

    static ERR_RT: StaticRuntime = static_runtime(
        |evt| {
            assert_eq!(
                "failed",
                evt.props()
                    .pull::<&(dyn std::error::Error + 'static), _>("err")
                    .unwrap()
                    .to_string()
            );
            assert_eq!(
                emit::Level::Error,
                evt.props().pull::<emit::Level, _>("lvl").unwrap()
            );
        },
        |_| true,
    );

    fn as_err(err: &anyhow::Error) -> &(dyn std::error::Error + 'static) {
        err.as_ref()
    }

    #[emit::span(rt: ERR_RT, err: as_err, "test")]
    fn exec_err() -> Result<(), anyhow::Error> {
        Err(anyhow::Error::from(io::Error::new(
            io::ErrorKind::Other,
            "failed",
        )))
    }

    exec_err().unwrap_err();
}

#[test]
#[cfg(feature = "std")]
fn span_err_shadow() {
    static ERR_RT: StaticRuntime = static_runtime(|_| {}, |_| true);

    fn err(err: &anyhow::Error) -> &(dyn std::error::Error + 'static) {
        err.as_ref()
    }

    #[emit::span(rt: ERR_RT, err, "test")]
    fn exec_err() -> Result<(), anyhow::Error> {
        Err(anyhow::Error::msg("failed"))
    }

    exec_err().unwrap_err();
}

#[test]
#[cfg(feature = "std")]
fn span_err_inline() {
    static ERR_RT: StaticRuntime = static_runtime(|_| {}, |_| true);

    type EmitError = dyn std::error::Error + 'static;

    #[emit::span(rt: ERR_RT, err: (|err| AsRef::<EmitError>::as_ref(err)), "test")]
    fn exec_err() -> Result<(), anyhow::Error> {
        Err(anyhow::Error::msg("failed"))
    }

    exec_err().unwrap_err();
}

#[test]
#[cfg(feature = "std")]
fn span_err_err_lvl() {
    use std::io;

    static ERR_RT: StaticRuntime = static_runtime(
        |evt| {
            assert_eq!(
                "failed",
                evt.props()
                    .pull::<&(dyn std::error::Error + 'static), _>("err")
                    .unwrap()
                    .to_string()
            );
            assert_eq!(
                emit::Level::Warn,
                evt.props().pull::<emit::Level, _>("lvl").unwrap()
            );
        },
        |_| true,
    );

    fn as_err(err: &anyhow::Error) -> &(dyn std::error::Error + 'static) {
        err.as_ref()
    }

    #[emit::span(rt: ERR_RT, err_lvl: "warn", err: as_err, "test")]
    fn exec_err() -> Result<(), anyhow::Error> {
        Err(anyhow::Error::from(io::Error::new(
            io::ErrorKind::Other,
            "failed",
        )))
    }

    exec_err().unwrap_err();
}

#[test]
#[cfg(feature = "std")]
fn span_err_str() {
    static ERR_RT: StaticRuntime = static_runtime(
        |evt| {
            assert_eq!("failed", evt.props().get("err").unwrap().to_string());
            assert_eq!(
                emit::Level::Warn,
                evt.props().pull::<emit::Level, _>("lvl").unwrap()
            );
        },
        |_| true,
    );

    #[emit::span(rt: ERR_RT, err_lvl: "warn", err: (|_| "failed"), "test")]
    fn exec_err() -> Result<(), ()> {
        Err(())
    }

    exec_err().unwrap_err();
}

#[test]
#[cfg(feature = "std")]
fn info_span_ok_lvl() {
    use std::io;

    static OK_RT: StaticRuntime = static_runtime(
        |evt| {
            assert_eq!(
                emit::Level::Debug,
                evt.props().pull::<emit::Level, _>("lvl").unwrap()
            );
        },
        |_| true,
    );

    static ERR_RT: StaticRuntime = static_runtime(
        |evt| {
            assert_eq!(
                "failed",
                evt.props()
                    .pull::<&(dyn std::error::Error + 'static), _>("err")
                    .unwrap()
                    .to_string()
            );
            assert_eq!(
                emit::Level::Info,
                evt.props().pull::<emit::Level, _>("lvl").unwrap()
            );
        },
        |_| true,
    );

    #[emit::info_span(rt: OK_RT, ok_lvl: emit::Level::Debug, "test")]
    fn exec_ok() -> Result<(), io::Error> {
        Ok(())
    }

    #[emit::info_span(rt: ERR_RT, ok_lvl: emit::Level::Debug, "test")]
    fn exec_err() -> Result<(), io::Error> {
        Err(io::Error::new(io::ErrorKind::Other, "failed"))
    }

    exec_ok().unwrap();
    exec_err().unwrap_err();
}

#[test]
#[cfg(feature = "std")]
fn info_span_err_lvl() {
    use std::io;

    static OK_RT: StaticRuntime = static_runtime(
        |evt| {
            assert_eq!(
                emit::Level::Info,
                evt.props().pull::<emit::Level, _>("lvl").unwrap()
            );
        },
        |_| true,
    );

    static ERR_RT: StaticRuntime = static_runtime(
        |evt| {
            assert_eq!(
                "failed",
                evt.props()
                    .pull::<&(dyn std::error::Error + 'static), _>("err")
                    .unwrap()
                    .to_string()
            );
            assert_eq!(
                emit::Level::Error,
                evt.props().pull::<emit::Level, _>("lvl").unwrap()
            );
        },
        |_| true,
    );

    #[emit::info_span(rt: OK_RT, err_lvl: emit::Level::Error, "test")]
    fn exec_ok() -> Result<(), io::Error> {
        Ok(())
    }

    #[emit::info_span(rt: ERR_RT, err_lvl: emit::Level::Error, "test")]
    fn exec_err() -> Result<(), io::Error> {
        Err(io::Error::new(io::ErrorKind::Other, "failed"))
    }

    exec_ok().unwrap();
    exec_err().unwrap_err();
}

#[tokio::test]
#[cfg(feature = "std")]
async fn info_span_err_lvl_async() {
    use std::io;

    static OK_RT: StaticRuntime = static_runtime(
        |evt| {
            assert_eq!(
                emit::Level::Info,
                evt.props().pull::<emit::Level, _>("lvl").unwrap()
            );
        },
        |_| true,
    );

    static ERR_RT: StaticRuntime = static_runtime(
        |evt| {
            assert_eq!(
                "failed",
                evt.props()
                    .pull::<&(dyn std::error::Error + 'static), _>("err")
                    .unwrap()
                    .to_string()
            );
            assert_eq!(
                emit::Level::Error,
                evt.props().pull::<emit::Level, _>("lvl").unwrap()
            );
        },
        |_| true,
    );

    #[emit::info_span(rt: OK_RT, err_lvl: emit::Level::Error, "test")]
    async fn exec_ok() -> Result<(), io::Error> {
        Ok(())
    }

    #[emit::info_span(rt: ERR_RT, err_lvl: emit::Level::Error, "test")]
    async fn exec_err() -> Result<(), io::Error> {
        Err(io::Error::new(io::ErrorKind::Other, "failed"))
    }

    exec_ok().await.unwrap();
    exec_err().await.unwrap_err();
}

#[test]
#[cfg(feature = "std")]
fn span_panic_lvl() {
    use std::panic;

    static CALLED: StaticCalled = StaticCalled::new();
    static RT: StaticRuntime = static_runtime(
        |evt| {
            // NOTE: We're panicking here, so can't assert
            if evt.props().pull("lvl") == Some(emit::Level::Error) {
                CALLED.record();
            }
        },
        |evt| {
            assert!(evt.props().get("lvl").is_none());

            true
        },
    );

    #[emit::span(rt: RT, panic_lvl: emit::Level::Error, "test")]
    fn exec() {
        panic!("explicit panic")
    }

    let _ = panic::catch_unwind(panic::AssertUnwindSafe(|| exec()));

    assert!(CALLED.was_called());
}

#[tokio::test]
#[cfg(feature = "std")]
async fn span_panic_lvl_async() {
    use std::panic;

    static CALLED: StaticCalled = StaticCalled::new();
    static RT: StaticRuntime = static_runtime(
        |evt| {
            // NOTE: We're panicking here, so can't assert
            if evt.props().pull("lvl") == Some(emit::Level::Error) {
                CALLED.record();
            }
        },
        |evt| {
            assert!(evt.props().get("lvl").is_none());

            true
        },
    );

    #[emit::span(rt: RT, panic_lvl: emit::Level::Error, "test")]
    async fn exec() {
        panic!("explicit panic")
    }

    let _ = tokio::spawn(async {
        exec().await;
    })
    .await;

    assert!(CALLED.was_called());
}

#[test]
fn span_setup() {
    static SETUP_CALLED: StaticCalled = StaticCalled::new();
    static DROP_CALLED: StaticCalled = StaticCalled::new();

    static RT: StaticRuntime = static_runtime(|_| {}, |_| true);

    struct Guard;

    impl Drop for Guard {
        fn drop(&mut self) {
            DROP_CALLED.record();
        }
    }

    fn setup() -> Guard {
        SETUP_CALLED.record();
        Guard
    }

    #[emit::span(rt: RT, setup, "greet {user}")]
    fn exec(user: &str) {
        assert!(SETUP_CALLED.was_called());
        assert!(!DROP_CALLED.was_called());
    }

    exec("Rust");

    assert!(SETUP_CALLED.was_called());
    assert!(DROP_CALLED.was_called());
}

#[tokio::test]
async fn span_setup_async() {
    static SETUP_CALLED: StaticCalled = StaticCalled::new();
    static DROP_CALLED: StaticCalled = StaticCalled::new();

    static RT: StaticRuntime = static_runtime(|_| {}, |_| true);

    struct Guard;

    impl Drop for Guard {
        fn drop(&mut self) {
            DROP_CALLED.record();
        }
    }

    fn setup() -> Guard {
        SETUP_CALLED.record();
        Guard
    }

    #[emit::span(rt: RT, setup, "greet {user}")]
    async fn exec(user: &str) {
        tokio::time::sleep(Duration::from_millis(1)).await;

        assert!(SETUP_CALLED.was_called());
        assert!(!DROP_CALLED.was_called());
    }

    exec("Rust").await;

    assert!(SETUP_CALLED.was_called());
    assert!(DROP_CALLED.was_called());
}

#[test]
#[cfg(feature = "std")]
fn span_props_precedence() {
    use std::io;

    static RT: StaticRuntime = static_runtime(
        |evt| {
            assert_eq!(
                "outer_ctxt",
                evt.props().pull::<&str, _>("outer_ctxt").unwrap()
            );

            assert_eq!(
                "inner_ctxt",
                evt.props()
                    .pull::<&str, _>("outer_ctxt_inner_ctxt")
                    .unwrap()
            );
            assert_eq!(
                "inner_ctxt",
                evt.props().pull::<&str, _>("inner_ctxt").unwrap()
            );

            assert_eq!("span", evt.props().pull::<&str, _>("lvl").unwrap());
        },
        |evt| {
            assert_eq!(
                "outer_ctxt",
                evt.props().pull::<&str, _>("outer_ctxt").unwrap()
            );

            assert_eq!(
                "inner_ctxt",
                evt.props()
                    .pull::<&str, _>("outer_ctxt_inner_ctxt")
                    .unwrap()
            );
            assert_eq!(
                "inner_ctxt",
                evt.props().pull::<&str, _>("inner_ctxt").unwrap()
            );

            assert_eq!("inner_ctxt", evt.props().pull::<&str, _>("lvl").unwrap());

            true
        },
    );

    #[emit::span(
        rt: RT,
        ok_lvl: "span",
        "test",
        outer_ctxt_inner_ctxt: "inner_ctxt",
        inner_ctxt: "inner_ctxt",
        lvl: "inner_ctxt",
    )]
    fn exec() -> Result<(), io::Error> {
        Ok(())
    }

    emit::Frame::push(
        RT.ctxt(),
        emit::props! {
            outer_ctxt_inner_ctxt: "outer_ctxt",
            outer_ctxt: "outer_ctxt",
            lvl: "outer_ctxt",
        },
    )
    .call(|| {
        let _ = exec();
    });
}

#[test]
fn span_impl_trait_return() {
    static CALLED: StaticCalled = StaticCalled::new();
    static RT: StaticRuntime = static_runtime(
        |_| {
            CALLED.record();
        },
        |_| true,
    );

    #[emit::span(rt: RT, "greet {user}")]
    fn exec(user: &str) -> impl std::fmt::Display {
        let _ = user;

        "done"
    }

    let _ = exec("Rust");

    RT.emitter().blocking_flush(Duration::from_secs(1));

    assert!(CALLED.was_called());
}

#[tokio::test]
async fn span_impl_trait_return_async() {
    static CALLED: StaticCalled = StaticCalled::new();
    static RT: StaticRuntime = static_runtime(
        |_| {
            CALLED.record();
        },
        |_| true,
    );

    #[emit::span(rt: RT, "greet {user}")]
    async fn exec(user: &str) -> impl std::fmt::Display {
        let _ = user;

        "done"
    }

    let _ = exec("Rust").await;

    RT.emitter().blocking_flush(Duration::from_secs(1));

    assert!(CALLED.was_called());
}
