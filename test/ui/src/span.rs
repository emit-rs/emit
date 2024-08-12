use std::time::Duration;

use emit::{Emitter, Props};

use crate::util::{static_runtime, StaticCalled, StaticRuntime};

#[test]
fn span_basic() {
    fn assert_event(evt: &emit::Event<impl Props>) {
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
    }

    static CALLED: StaticCalled = StaticCalled::new();
    static RT: StaticRuntime = static_runtime(
        |evt| {
            assert_event(evt);

            CALLED.record();
        },
        |_| true,
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
        |_| true,
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
        |_| true,
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
        |_| true,
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
        |_| true,
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

#[test]
fn span_guard() {
    static CALLED: StaticCalled = StaticCalled::new();
    static RT: StaticRuntime = static_runtime(|_| CALLED.record(), |_| true);

    #[emit::span(rt: RT, guard: span, "test")]
    fn exec() {
        let span: emit::span::SpanGuard<_, _, _> = span;

        assert!(span.is_enabled());
        span.complete();
    }

    exec();

    RT.emitter().blocking_flush(Duration::from_secs(1));

    assert!(CALLED.was_called());
}

#[test]
#[cfg(feature = "std")]
fn span_ok_lvl() {
    use std::io;

    static OK_CALLED: StaticCalled = StaticCalled::new();
    static OK_RT: StaticRuntime = static_runtime(
        |evt| {
            assert_eq!(
                emit::Level::Info,
                evt.props().pull::<emit::Level, _>("lvl").unwrap()
            );

            OK_CALLED.record()
        },
        |_| true,
    );

    static ERR_CALLED: StaticCalled = StaticCalled::new();
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

            ERR_CALLED.record()
        },
        |_| true,
    );

    #[emit::span(rt: OK_RT, ok_lvl: emit::Level::Info, "test")]
    fn exec_ok() -> Result<(), io::Error> {
        Ok(())
    }

    #[emit::span(rt: ERR_RT, ok_lvl: emit::Level::Info, "test")]
    fn exec_err() -> Result<(), io::Error> {
        Err(io::Error::new(io::ErrorKind::Other, "failed"))
    }

    let _ = exec_ok();
    let _ = exec_err();

    OK_RT.emitter().blocking_flush(Duration::from_secs(1));
    ERR_RT.emitter().blocking_flush(Duration::from_secs(1));

    assert!(OK_CALLED.was_called());
    assert!(ERR_CALLED.was_called());
}

#[test]
#[cfg(feature = "std")]
fn span_err_lvl() {
    use std::io;

    static OK_CALLED: StaticCalled = StaticCalled::new();
    static OK_RT: StaticRuntime = static_runtime(
        |evt| {
            assert!(evt.props().get("lvl").is_none());

            OK_CALLED.record()
        },
        |_| true,
    );

    static ERR_CALLED: StaticCalled = StaticCalled::new();
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

            ERR_CALLED.record()
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

    let _ = exec_ok();
    let _ = exec_err();

    OK_RT.emitter().blocking_flush(Duration::from_secs(1));
    ERR_RT.emitter().blocking_flush(Duration::from_secs(1));

    assert!(OK_CALLED.was_called());
    assert!(ERR_CALLED.was_called());
}

#[test]
#[cfg(feature = "std")]
fn info_span_ok_lvl() {
    use std::io;

    static OK_CALLED: StaticCalled = StaticCalled::new();
    static OK_RT: StaticRuntime = static_runtime(
        |evt| {
            assert_eq!(
                emit::Level::Debug,
                evt.props().pull::<emit::Level, _>("lvl").unwrap()
            );

            OK_CALLED.record()
        },
        |_| true,
    );

    static ERR_CALLED: StaticCalled = StaticCalled::new();
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

            ERR_CALLED.record()
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

    let _ = exec_ok();
    let _ = exec_err();

    OK_RT.emitter().blocking_flush(Duration::from_secs(1));
    ERR_RT.emitter().blocking_flush(Duration::from_secs(1));

    assert!(OK_CALLED.was_called());
    assert!(ERR_CALLED.was_called());
}

#[test]
#[cfg(feature = "std")]
fn info_span_err_lvl() {
    use std::io;

    static OK_CALLED: StaticCalled = StaticCalled::new();
    static OK_RT: StaticRuntime = static_runtime(
        |evt| {
            assert_eq!(
                emit::Level::Info,
                evt.props().pull::<emit::Level, _>("lvl").unwrap()
            );

            OK_CALLED.record()
        },
        |_| true,
    );

    static ERR_CALLED: StaticCalled = StaticCalled::new();
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

            ERR_CALLED.record()
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

    let _ = exec_ok();
    let _ = exec_err();

    OK_RT.emitter().blocking_flush(Duration::from_secs(1));
    ERR_RT.emitter().blocking_flush(Duration::from_secs(1));

    assert!(OK_CALLED.was_called());
    assert!(ERR_CALLED.was_called());
}

#[test]
#[cfg(feature = "std")]
fn span_props_precedence() {
    use std::io;

    static CALLED: StaticCalled = StaticCalled::new();
    static RT: StaticRuntime = static_runtime(
        |evt| {
            println!("{:?}", evt);

            assert_eq!("ctxt", evt.props().pull::<&str, _>("ctxt").unwrap());

            assert_eq!("evt", evt.props().pull::<&str, _>("ctxt_evt").unwrap());
            assert_eq!("evt", evt.props().pull::<&str, _>("evt").unwrap());
            assert_eq!("evt", evt.props().pull::<&str, _>("lvl").unwrap());

            CALLED.record()
        },
        |_| true,
    );

    #[emit::span(
        rt: RT,
        ok_lvl: "span",
        "test",
        ctxt_evt: "evt",
        evt: "evt",
        lvl: "evt",
    )]
    fn exec() -> Result<(), io::Error> {
        Ok(())
    }

    emit::Frame::push(
        RT.ctxt(),
        emit::props! {
            ctxt_evt: "ctxt",
            ctxt: "ctxt",
            lvl: "ctxt",
        },
    )
    .call(|| {
        let _ = exec();
    });

    RT.emitter().blocking_flush(Duration::from_secs(1));

    assert!(CALLED.was_called());
}
