#![deny(unused_must_use)]

fn main() {
    let my_metric = 42;
    emit::metric!(value: my_metric);
}
