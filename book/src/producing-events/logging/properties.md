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

emit::emit!(
    "Hello, {user}",
    #[emit::as_debug]
    user: User {
        name: "Rust",
    }
);
```

```text
Event {
    mdl: "my_app",
    tpl: "Hello, {user}",
    extent: Some(
        "2024-10-02T22:03:23.588049400Z",
    ),
    props: {
        "user": User {
            name: "Rust",
        },
    },
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

emit::emit!(
    "Hello, {user}",
    #[emit::as_serde]
    user: User {
        name: "Rust",
    }
);
```

```text
Event {
    mdl: "my_app",
    tpl: "Hello, {user}",
    extent: Some(
        "2024-10-02T22:05:05.258099900Z",
    ),
    props: {
        "user": User {
            name: "Rust",
        },
    },
}
```
