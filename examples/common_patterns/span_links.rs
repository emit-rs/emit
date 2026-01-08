/*!
This example demonstrates how to add links to spans.

Links relate spans outside of the normal parent-child hierarchy.
Links are largely informative and may not be understood by downstream consumers.
*/

use std::time::Duration;

#[emit::span("inner")]
fn exec() {}

fn main() {
    let rt = emit::setup()
        .emit_to(emit::emitter::from_fn(|evt| println!("{evt:?}")))
        .init();

    // Span links can be strings formatted as `{traceid}-{spanid}`
    // or an instance of a `SpanLink`
    let link = emit::span::SpanLink::new(
        emit::span::TraceId::from_u128(0x1).unwrap(),
        emit::span::SpanId::from_u64(0x1).unwrap(),
    );

    // The `span_links` well-known property can carry an array of span links
    //
    // Here we create some ambient context with a span link that the span
    // created by `exec` will include. Span links typically come from outside
    // callers, like a message queue.
    let frame = emit::Frame::push(
        emit::ctxt(),
        emit::props! {
            #[emit::as_serde]
            span_links: [
                link,
            ]
        },
    );

    frame.call(exec);

    rt.blocking_flush(Duration::from_secs(5));
}
