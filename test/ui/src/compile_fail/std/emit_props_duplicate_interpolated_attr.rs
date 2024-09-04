fn main() {
    emit::emit!("template {#[emit::optional] x}", x: 42);
}
