# Span links

Spans within a trace are linked by a single parent-child relationship. Each span may have any number of child spans that it causes within the same trace, but each child span may only have a single parent. The resulting trace can be expressed as a _tree_, which is a specific kind of graph. Span links make it possible to describe relationships between spans that can't be represented in a tree, and to link spans in different traces.

Here's an example of a link between two spans in different traces:

```mermaid
flowchart TB
    subgraph trace:0a85ccaf666e11aaca6bd5d469e2850d
        span:e58873f5ee2b697d
        span:e58873f5ee2b697d --- span:2b9caa35eaefed3a
        span:e58873f5ee2b697d --- span:50bd7df705bda59e
        span:50bd7df705bda59e --- span:13872d1c5fd475b4
    end

    subgraph trace:7090fa7447c7c99f94c035fc5753acbf
        span:d0627f2451098bee
        span:d0627f2451098bee -. link .- span:2b9caa35eaefed3a
    end
```

In this example, the link on span `d0627f2451098bee` is to the span `2b9caa35eaefed3a` in a different trace.

In `emit`, span links are expressed through the `span_links` [well-known property](https://docs.rs/emit/1.17.2/emit/well_known/index.html) as a sequence of formatted strings, or a sequence of the [`SpanLink`](https://docs.rs/emit/1.17.2/emit/span/struct.SpanLink.html) type:

```rust
# extern crate emit;
#[emit::span(
    guard: span,
    "wait a bit",
    sleep_ms,
)]
fn wait_a_bit(sleep_ms: u64) {
    // Add span links to the guard rather than as props in the macro so they aren't
    // also added to any child spans or events
    let span_links = [
        "0a85ccaf666e11aaca6bd5d469e2850d-2b9caa35eaefed3a",
    ];
    let _span = span.push_prop("span_links", emit::Value::capture_serde(&span_links));

    std::thread::sleep(std::time::Duration::from_millis(sleep_ms));

    emit::emit!("waiting a bit longer");

    std::thread::sleep(std::time::Duration::from_millis(sleep_ms));
}
```

Since the expected type of `span_links` is a sequence, you'll need to use either the [`#[as_serde]`](../../reference/property-attributes.md#as_serde) or [`#[as_sval]`](../../reference/property-attributes.md#as_sval) attributes to capture them.

In this example, the span link is a constant string. You can avoid allocating strings at runtime for links by using the [`SpanLink`](https://docs.rs/emit/1.17.2/emit/span/struct.SpanLink.html) type instead:

```rust
# extern crate emit;
#[emit::span(
    guard: span,
    "wait a bit",
    sleep_ms,
)]
fn wait_a_bit(sleep_ms: u64) {
    // Add span links to the guard rather than as props in the macro so they aren't
    // also added to any child spans or events
    let span_links = [
        emit::span::SpanLink::new(
            emit::span::TraceId::from_u128(0x0a85ccaf666e11aaca6bd5d469e2850d).unwrap(),
            emit::span::SpanId::from_u64(0x2b9caa35eaefed3a).unwrap(),
        ),
    ];
    let _span = span.push_prop("span_links", emit::Value::capture_serde(&span_links));

    std::thread::sleep(std::time::Duration::from_millis(sleep_ms));

    emit::emit!("waiting a bit longer");

    std::thread::sleep(std::time::Duration::from_millis(sleep_ms));
}
```
