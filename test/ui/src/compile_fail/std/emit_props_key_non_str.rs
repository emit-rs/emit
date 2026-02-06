fn main() {
    emit::emit!("template", #[emit::key(name: 42)] a: "some value");
}
