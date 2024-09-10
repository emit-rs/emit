fn main() {

}

#[emit::span(ok_lvl: "info", err_lvl: "warn", "try_work")]
fn try_work() -> Result<(), impl std::error::Error> {
    Ok::<(), std::io::Error>(())
}
