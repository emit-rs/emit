#[allow(unused_imports)]
use crate::shadow::*;

#[test]
fn format_basic() {
    assert_eq!("Hello, Rust", emit::format!("Hello, {user}", user: "Rust"));
}

#[test]
fn format_fmt_pad_numeric() {
    assert_eq!(
        "The value is 015",
        emit::format!("The value is {value}", #[emit::fmt(">03")] value: 15)
    );
}

#[test]
fn format_fmt_pad_left() {
    assert_eq!(
        "The value is x  ",
        emit::format!("The value is {value}", #[emit::fmt("<3")] value: "x")
    );
}

#[test]
fn format_fmt_pad_right() {
    assert_eq!(
        "The value is   x",
        emit::format!("The value is {value}", #[emit::fmt(">3")] value: "x")
    );
}

#[test]
fn format_fmt_sign_numeric() {
    assert_eq!(
        "The value is +15",
        emit::format!("The value is {value}", #[emit::fmt("+")] value: 15)
    );
}

#[test]
fn format_fmt_precision_numeric() {
    assert_eq!(
        "The value is 15.000",
        emit::format!("The value is {value}", #[emit::fmt(".3")] value: 15.0)
    );
}
