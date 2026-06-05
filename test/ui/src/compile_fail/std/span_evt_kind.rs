#[emit::span("test", evt_kind: "custom")]
fn check() {}

fn main() {
    check();
}
