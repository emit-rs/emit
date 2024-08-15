#[test]
fn tpl_basic() {
    todo!()
}

#[test]
fn tpl_parts() {
    todo!()
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
    todo!()
}
