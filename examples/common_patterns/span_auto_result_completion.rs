use std::time::Duration;

#[derive(thiserror::Error, Debug)]
#[error("invalid number {n}")]
struct Error {
    n: i32,
}

// The `ok_lvl` and `err_lvl` control parameters can be used on functions
// returning a `Result` to change the level based on the result returned.
// `err_lvl` also attaches the error as a property.
#[emit::span(
    ok_lvl: emit::Level::Info,
    err_lvl: emit::Level::Error,
    "Running an example",
    i,
)]
fn example(i: i32) -> Result<(), Error> {
    let r = i + 1;

    if r == 4 {
        Err(Error { n: r })
    } else {
        Ok(())
    }
}

fn main() {
    let rt = emit::setup().emit_to(emit_term::stdout()).init();

    let _ = example(1);
    let _ = example(3);

    rt.blocking_flush(Duration::from_secs(5));
}
