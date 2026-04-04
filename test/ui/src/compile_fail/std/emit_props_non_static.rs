use std::fmt;

fn main() {
    let short_lived = String::from("x");

    exec(&InternalRef(&short_lived));
}

pub fn exec(x: &InternalRef<'_>) {
    emit::emit!("template {x}");
}

struct InternalRef<'a>(&'a str);

impl<'a> fmt::Display for InternalRef<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self.0, f)
    }
}
