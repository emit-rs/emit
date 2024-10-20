/*!
This example demonstrates how to use `emit` in unit tests.

Using the `setup` control parameter, you can ensure `emit` is initialized before tests run,
and that any emitted events are flushed when the test completes.
*/

// This is the piece of code we're going to test
pub fn add(a: i32, b: i32) -> i32 {
    let r = a + b;

    emit::debug!("{r} = {a} + {b}");

    r
}

// This function is invoked by `#[emit::span]`s because they use it
// as the `setup` control parameter. It's bound to a value that's dropped
// at the end of the annotated function
#[cfg(test)]
fn setup() -> Option<impl Drop> {
    emit::setup()
        .emit_to(emit_term::stdout())
        .try_init()
        .map(|init| init.flush_on_drop(std::time::Duration::from_secs(1)))
}

#[test]
#[emit::span(setup, "add_1_1")]
fn add_1_1() {
    assert_eq!(2, add(1, 1));
}

#[test]
#[emit::span(setup, "add_1_0")]
fn add_1_0() {
    assert_eq!(1, add(1, 0));
}

fn main() {}
