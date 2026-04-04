use std::fmt;

fn main() {
    let short_lived = String::from("x");
    let x = InternalRef(&short_lived);

    emit::emit!("template {x}");
}

struct InternalRef<'a>(&'a str);

impl<'a> fmt::Display for InternalRef<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self.0, f)
    }
}
