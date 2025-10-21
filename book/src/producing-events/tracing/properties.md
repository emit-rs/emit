# Attaching properties to spans

Properties added to the span macros are added to an ambient context and automatically included on any events emitted within that operation:

```rust
# extern crate emit;
#[emit::span("wait a bit", sleep_ms)]
fn wait_a_bit(sleep_ms: u64) {
    std::thread::sleep(std::time::Duration::from_millis(sleep_ms));

    emit::emit!("waiting a bit longer");

    std::thread::sleep(std::time::Duration::from_millis(sleep_ms));
}
```

```text
Event {
    mdl: "my_app",
    tpl: "waiting a bit longer",
    extent: Some(
        "2024-04-27T22:47:34.780288000Z",
    ),
    props: {
        "trace_id": d2a5e592546010570472ac6e6457c086,
        "sleep_ms": 1200,
        "span_id": ee9fde093b6efd78,
    },
}
Event {
    mdl: "my_app",
    tpl: "wait a bit",
    extent: Some(
        "2024-04-27T22:47:33.574839000Z".."2024-04-27T22:47:35.985844000Z",
    ),
    props: {
        "evt_kind": span,
        "span_name": "wait a bit",
        "trace_id": d2a5e592546010570472ac6e6457c086,
        "sleep_ms": 1200,
        "span_id": ee9fde093b6efd78,
    },
}
```

Any operations started within a span will inherit its identifiers:

```rust
# extern crate emit;
#[emit::span("outer span", sleep_ms)]
fn outer_span(sleep_ms: u64) {
    std::thread::sleep(std::time::Duration::from_millis(sleep_ms));

    inner_span(sleep_ms / 2);
}

#[emit::span("inner span", sleep_ms)]
fn inner_span(sleep_ms: u64) {
    std::thread::sleep(std::time::Duration::from_millis(sleep_ms));
}
```

```text
Event {
    mdl: "my_app",
    tpl: "inner span",
    extent: Some(
        "2024-04-27T22:50:50.385706000Z".."2024-04-27T22:50:50.994509000Z",
    ),
    props: {
        "evt_kind": span,
        "span_name": "inner span",
        "trace_id": 12b2fde225aebfa6758ede9cac81bf4d,
        "span_parent": 23995f85b4610391,
        "sleep_ms": 600,
        "span_id": fc8ed8f3a980609c,
    },
}
Event {
    mdl: "my_app",
    tpl: "outer span",
    extent: Some(
        "2024-04-27T22:50:49.180025000Z".."2024-04-27T22:50:50.994797000Z",
    ),
    props: {
        "evt_kind": span,
        "span_name": "outer span",
        "sleep_ms": 1200,
        "span_id": 23995f85b4610391,
        "trace_id": 12b2fde225aebfa6758ede9cac81bf4d,
    },
}
```

Notice the `span_parent` of `inner_span` is the same as the `span_id` of `outer_span`. That's because `inner_span` was called within the execution of `outer_span`.

## Capturing complex values

Properties aren't limited to strings; they can be arbitrarily complex structured values. See the following sections and [Value data model](../../reference/events.md#value-data-model) for more details.

### Using `fmt::Debug`

If you want to log a type that implements `Debug`, you can apply the [`#[as_debug]`](../../reference/property-attributes.md#as_debug) attribute to it to capture it with its debug format:

```rust
# extern crate emit;
#[derive(Debug)]
struct User<'a> {
    name: &'a str,
}

#[emit::span("greet {user}", #[emit::as_debug] user)]
fn greet(user: &User) {
    println!("Hello, {}", user.name);
}
```

### Using `serde::Serialize`

If you want to log a type that implements `Serialize`, you can apply the [`#[as_serde]`](../../reference/property-attributes.md#as_serde) attribute to it to capture it as a structured value:

```rust
# extern crate emit;
# #[macro_use] extern crate serde;
#[derive(Serialize)]
struct User<'a> {
    name: &'a str,
}

#[emit::span("greet {user}", #[emit::as_serde] user)]
fn greet(user: &User) {
    println!("Hello, {}", user.name);
}
```
