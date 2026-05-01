/*!
This example demonstrates how to use `emit` in unit tests.

Using the `setup` control parameter, you can ensure `emit` is initialized before tests run,
and that any emitted events are flushed when the test completes.

Tests typically panic when they fail. You can use the `panic_lvl` control parameter to mark
spans as failed if a panic occurs.
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
    let rt = emit::setup()
        .emit_to(emit_term::stdout())
        .try_init()
        .map(|init| init.flush_on_drop(std::time::Duration::from_secs(1)));

    // Set a panic hook so the location of the panic will also be captured
    // We only need to do this once
    if rt.is_some() {
        std::panic::set_hook(Box::new(|payload| {
            emit::error!("panic detected", #[emit::as_display] err: payload);
        }));
    }

    rt
}

#[test]
#[emit::span(setup, fn_name, "test {fn_name}")]
fn add_1_1() {
    assert_eq!(2, add(1, 1));
}

#[test]
#[emit::span(setup, fn_name, "test {fn_name}")]
fn add_1_0() {
    assert_eq!(2, add(1, 0));
}

fn main() {}
