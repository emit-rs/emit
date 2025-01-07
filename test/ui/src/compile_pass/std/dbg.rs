/*!
Written as an independent test because `dbg!` can't use a custom runtime.
*/

use std::sync::{Arc, Mutex};

struct Emitter(Arc<Mutex<Vec<(String, String)>>>);

impl Emitter {
    #[track_caller]
    fn check(&self, prefix: &str) {
        let (msg, evt) = self.0.lock().unwrap().pop().unwrap();

        assert!(msg.starts_with(prefix), "expected '{msg}' to start with '{prefix}'\n{evt}");
    }
}

impl emit::Emitter for Emitter {
    fn emit<E: emit::event::ToEvent>(&self, evt: E) {
        let evt = evt.to_event();

        let msg = evt.msg().to_string();
        let evt = format!("{:?}", evt);

        self.0.lock().unwrap().push((msg, evt));
    }

    fn blocking_flush(&self, _: std::time::Duration) -> bool {
        false
    }
}

fn main() {
    let rt = emit::setup().emit_to(Emitter(Arc::new(Mutex::new(Vec::new())))).init();
    let emitter = rt.emitter();

    let a = 42;
    let b = "text";

    emit::dbg!();
    emitter.check("at ");

    emit::dbg!(a);
    emitter.check("a = 42 at ");

    emit::dbg!(a: 42);
    emitter.check("a = 42 at ");

    emit::dbg!(#[emit::as_display] a: Data { id: 42 });
    emitter.check("a = 42 at ");

    emit::dbg!(a, b);
    emitter.check("a = 42, b = text at ");

    emit::dbg!(#[emit::key("b")] a);
    emitter.check("a = 42 at ");

    emit::dbg!("{a}");
    emitter.check("42");

    emit::dbg!("{a}", b);
    emitter.check("42");

    emit::dbg!("{a}", #[emit::key("b")] a);
    emitter.check("42");
}

struct Data {
    id: i32,
}

impl std::fmt::Display for Data {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.id, f)
    }
}
