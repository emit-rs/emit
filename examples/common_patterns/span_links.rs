/*!
This example demonstrates how to add links to spans.

Links relate spans outside of the normal parent-child hierarchy.
Links are largely informative and may not be understood by downstream consumers.
*/

use std::time::Duration;

#[emit::span(
    guard: span,
    "outer"
)]
fn outer() {
    // Links will typically come from external context, like a messaging system
    // We're adding the links to the span here rather than as props in the `#[span]`
    // attribute because they shouldn't appear on child spans or events
    let links = [emit::span::SpanLink::new(
        emit::span::TraceId::from_u128(0x1).unwrap(),
        emit::span::SpanId::from_u64(0x1).unwrap(),
    )];

    let _span = span.push_prop("span_links", emit::Value::capture_serde(&links));

    inner();
}

#[emit::span("inner")]
fn inner() {}

fn main() {
    let rt = emit::setup()
        .emit_to(emit::emitter::from_fn(|evt| println!("{evt:?}")))
        .init();

    outer();

    rt.blocking_flush(Duration::from_secs(5));
}
