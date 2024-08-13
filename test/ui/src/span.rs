use std::time::Duration;

use emit::{Emitter, Props};

use crate::util::{static_runtime, StaticCalled, StaticRuntime};

#[test]
fn span_basic() {
    fn assert_event_base(evt: &emit::Event<impl Props>) {
        assert_eq!("greet Rust", evt.msg().to_string());
        assert_eq!("greet {user}", evt.tpl().to_string());
        assert_eq!(module_path!(), evt.module());

        assert_eq!(
            "greet {user}",
            evt.props().pull::<&str, _>("span_name").unwrap()
        );

        assert_eq!(
            emit::Kind::Span,
            evt.props().pull::<emit::Kind, _>("evt_kind").unwrap()
        );

        assert!(evt
            .props()
            .pull::<emit::span::TraceId, _>("trace_id")
            .is_some());
        assert!(evt
            .props()
            .pull::<emit::span::SpanId, _>("span_id")
            .is_some());
    }

    fn assert_event(evt: &emit::Event<impl Props>) {
        assert!(evt.extent().unwrap().is_span());
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
        let _ = user;
    }

    #[emit::debug_span(rt: DEBUG_RT, "greet {user}")]
    fn exec_debug(user: &str) {
        let _ = user;
    }

    #[emit::info_span(rt: INFO_RT, "greet {user}")]
    fn exec_info(user: &str) {
        let _ = user;
    }

    #[emit::warn_span(rt: WARN_RT, "greet {user}")]
    fn exec_warn(user: &str) {
        let _ = user;
    }

    #[emit::error_span(rt: ERROR_RT, "greet {user}")]
    fn exec_error(user: &str) {
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
    async fn exec(user: &str) {
        let _ = user;
    }

    exec("Rust").await;

    RT.emitter().blocking_flush(Duration::from_secs(1));

    assert!(CALLED.was_called());
}

#[test]
fn span_guard() {
    static RT: StaticRuntime = static_runtime(|_| {}, |_| true);

    #[emit::span(rt: RT, guard: span, "test")]
    fn exec() {
        let span: emit::span::SpanGuard<_, _, _> = span;

        assert!(span.is_enabled());
        span.complete();
    }

    exec();
}

#[test]
fn span_filter() {
    static CALLED: StaticCalled = StaticCalled::new();
    static RT: StaticRuntime = static_runtime(|_| CALLED.record(), |evt| evt.module() == "true");

    #[emit::span(rt: RT, module: emit::path!("true"), "test")]
    fn exec_true() {}

    #[emit::span(rt: RT, module: emit::path!("false"), "test")]
    fn exec_false() {}

    exec_true();
    exec_false();

    RT.emitter().blocking_flush(Duration::from_secs(1));

    assert_eq!(1, CALLED.called_times());
}

#[test]
fn span_filter_guard() {
    static CALLED: StaticCalled = StaticCalled::new();
    static RT: StaticRuntime = static_runtime(|_| CALLED.record(), |evt| evt.module() == "true");

    #[emit::span(rt: RT, guard: span, module: emit::path!("true"), "test")]
    fn exec_true() {
        assert!(span.is_enabled());
    }

    #[emit::span(rt: RT, guard: span, module: emit::path!("false"), "test")]
    fn exec_false() {
        assert!(!span.is_enabled());
    }

    exec_true();
    exec_false();

    RT.emitter().blocking_flush(Duration::from_secs(1));

    assert_eq!(1, CALLED.called_times());
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
            assert!(evt.props().get("lvl").is_none());
        },
        |evt| {
            assert!(evt.props().get("lvl").is_none());

            true
        },
    );

    #[emit::span(rt: OK_RT, ok_lvl: emit::Level::Info, "test")]
    fn exec_ok() -> Result<(), io::Error> {
        Ok(())
    }

    #[emit::span(rt: ERR_RT, ok_lvl: emit::Level::Info, "test")]
    fn exec_err() -> Result<(), io::Error> {
        Err(io::Error::new(io::ErrorKind::Other, "failed"))
    }

    exec_ok().unwrap();
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
