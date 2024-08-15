#[test]
fn tpl_basic() {
    match emit::tpl!("Hello, {user}") {
        tpl => {
            let parts = tpl.parts().collect::<Vec<_>>();

            assert_eq!("Hello, ", parts[0].as_text().unwrap());
            assert_eq!("user", parts[1].label().unwrap());
        }
    }
}

#[test]
fn tpl_parts() {
    let parts = emit::tpl_parts!("Hello, {user}");

    assert_eq!("Hello, ", parts[0].as_text().unwrap());
    assert_eq!("user", parts[1].label().unwrap());
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
    match emit::tpl!(
        "Hello, {user}",
        #[emit::fmt("?")]
        user
    ) {
        tpl => {
            let parts = tpl.parts().collect::<Vec<_>>();

            assert_eq!("Hello, ", parts[0].as_text().unwrap());
            assert_eq!("user", parts[1].label().unwrap());
            assert!(parts[1].formatter().is_some());
        }
    }
}
