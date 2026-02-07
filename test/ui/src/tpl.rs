#[allow(unused_imports)]
use crate::shadow::*;

#[test]
fn tpl_basic() {
    let tpl = emit::tpl!("Hello, {user}");

    let parts = tpl.parts().collect::<::std::vec::Vec<_>>();

    assert_eq!("Hello, ", parts[0].as_text().unwrap());
    assert_eq!("user", parts[1].label().unwrap());
}

#[test]
fn tpl_event_meta() {
    let _ = emit::tpl!("{ts_start}..{ts} {mdl} {tpl} {msg}");
}

#[test]
fn tpl_cfg() {
    assert_eq!(
        "Hello, {user}",
        emit::tpl!("Hello, {#[cfg(not(emit_disabled))] user}{#[cfg(emit_disabled)]ignored}")
            .to_string()
    );
}

#[test]
fn tpl_fmt() {
    let tpl = emit::tpl!(
        "Hello, {user}",
        #[emit::fmt("?")]
        user
    );

    let parts = tpl.parts().collect::<::std::vec::Vec<_>>();

    assert_eq!("Hello, ", parts[0].as_text().unwrap());
    assert_eq!("user", parts[1].label().unwrap());
    assert!(parts[1].formatter().is_some());
}
