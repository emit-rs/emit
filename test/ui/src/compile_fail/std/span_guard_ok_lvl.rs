use std::io;

#[emit::span(rt: RT, guard: span, ok_lvl: emit::Level::Info, "test")]
pub fn exec(fail: bool) -> Result<bool, io::Error> {
    if fail {
        return Err(io::Error::new(io::ErrorKind::Other, "failed"));
    }

    span.complete();

    Ok(true)
}

fn main() {

}
