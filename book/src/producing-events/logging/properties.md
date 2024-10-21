# Attaching properties to events

Properties can be attached to log events by including them in the message template:

```rust
# extern crate emit;
let user = "Rust";

emit::emit!("Hello, {user}");
```

```text
Event {
    mdl: "my_app",
    tpl: "Hello, {user}",
    extent: Some(
        "2024-10-02T21:59:35.084177500Z",
    ),
    props: {
        "user": "Rust",
    },
}
```

Properties can also be attached after the template:

```rust
# extern crate emit;
let user = "Rust";

emit::emit!("Saluten, {user}", lang: "eo");
```

```text
Event {
    mdl: "my_app",
    tpl: "Saluten, {user}",
    extent: Some(
        "2024-10-02T21:59:56.406474900Z",
    ),
    props: {
        "lang": "eo",
        "user": "Rust",
    },
}
```

See [Template syntax and rendering](../../reference/templates.md) for more details.

Properties aren't limited to strings; they can be arbitrarily complex structured values. See [Value data model](../../reference/events.md#value-data-model) for more details.
