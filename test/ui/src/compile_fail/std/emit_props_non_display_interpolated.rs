fn main() {
    let x = NotDisplay;

    emit::emit!("template {x}");
}

struct NotDisplay;
