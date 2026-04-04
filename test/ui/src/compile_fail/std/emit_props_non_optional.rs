fn main() {
    let x = Some("some data");

    emit::emit!("template {x}");
}
