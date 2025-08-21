#[emit::span(rt: RT, guard: span, ok_lvl: emit::Level::Info, "test")]
pub fn exec(fail: bool) -> Result<bool, std::io::Error> {
    if fail {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "failed"));
    }

    span.complete();

    Ok(true)
}

fn main() {

}
