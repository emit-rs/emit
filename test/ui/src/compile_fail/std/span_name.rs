#[emit::span("test", span_name: "custom")]
fn check() {}

fn main() {
    check();
}
