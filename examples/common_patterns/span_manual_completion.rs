use std::time::Duration;

// The `arg` control parameter binds an `emit::Span` that you can use
// to manually complete your span, adding extra properties if needed.
//
// If you don't complete the span manually then it will complete on its
// own when it falls out of scope.
#[emit::span(arg: span, "Running an example", i)]
fn example(i: i32) {
    let r = i + 1;

    if r == 4 {
        span.complete_with(|event| {
            emit::error!(event, "Running an example failed with {r}");
        });
    } else {
        span.complete_with(|event| {
            emit::info!(event, "Running an example produced {r}");
        });
    }
}

fn main() {
    let rt = emit::setup().emit_to(emit_term::stdout()).init();

    example(1);
    example(3);

    rt.blocking_flush(Duration::from_secs(5));
}
