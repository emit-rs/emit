# Property attributes

This section calls out a few attributes you can use to change the way properties are captured. See [the crate docs](https://docs.rs/emit/1.17.2/emit/index.html#attributes) for a complete list of attributes defined by `emit`.

## `#[cfg]`

You can add the standard `#[cfg]` attribute to properties in templates. If the `#[cfg]` evaluates to `false` then the entire hole will be omitted from the template.

```rust
# extern crate emit;
emit::emit!("Hello, {#[cfg(disabled)] user}");
```

```text
Event {
    mdl: "my_app",
    tpl: "Hello, ",
    extent: Some(
        "2024-10-02T22:01:01.431485400Z",
    ),
    props: {},
}
```

## `#[key]`

The [`#[key`](https://docs.rs/emit/1.17.2/emit/attr.key.html) attribute can be used to set the name of a captured property. This can be used to give a property a name that isn't a valid Rust identifier:

```rust
# extern crate emit;
# let user = "Rust";
emit::emit!("Hello, {user}", #[emit::key("user.name")] user);
```

```text
Event {
    mdl: "my_app",
    tpl: "Hello, {user.name}",
    extent: Some(
        "2024-10-02T22:01:24.321035400Z",
    ),
    props: {
        "user.name": "Rust",
    },
}
```

## `#[fmt]`

The [`#[fmt]`](https://docs.rs/emit/1.17.2/emit/attr.fmt.html) attribute applies a formatter to a property value when rendering it in the template. The accepted syntax is the same as Rust's [`std::fmt`](https://doc.rust-lang.org/std/fmt/index.html):

```rust
# extern crate emit;
emit::emit!("pi is {pi}", #[emit::fmt(".3")] pi: 3.1415927);
```

```text
Event {
    mdl: "my_app",
    tpl: "pi is {pi}",
    extent: Some(
        "2024-10-02T22:01:58.842629700Z",
    ),
    props: {
        "pi": 3.1415927,
    },
}
```

When rendered, the template will produce:

```text
pi is 3.142
```

## `#[as_debug]`

The [`#[as_debug]`](https://docs.rs/emit/1.17.2/emit/attr.as_debug.html) attribute captures a property value using its [`Debug`](https://doc.rust-lang.org/std/fmt/trait.Debug.html) implementation, instead of the default `Display + 'static`:

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

Note that the structure of the captured value is lost. It'll be treated as a string like `"User { name: \"Rust\" }"` when serialized:

```json
{
    "mdl": "my_app",
    "tpl": "Hello, {user}",
    "ts": "2024-10-02T22:03:23.588049400Z",
    "user": "User { name: \"Rust\" }"
}
```

See [Property capturing](./property-capturing.md) for more details.

## `#[as_serde]`

The [`#[as_serde]`](https://docs.rs/emit/1.17.2/emit/attr.as_serde.html) attribute captures a property value using its [`Serialize`](https://docs.rs/serde/latest/serde/trait.Serialize.html) implementation, instead of the default `Display + 'static`:

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

The structure of properties captured this way is fully preserved:

```json
{
    "mdl": "my_app",
    "tpl": "Hello, {user}",
    "ts": "2024-10-02T22:05:05.258099900Z",
    "user": {
        "name": "Rust"
    }
}
```

See [Property capturing](./property-capturing.md) for more details.
