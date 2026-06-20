# Property attributes

This section calls out a few attributes you can use to change the way properties are captured. See [the crate docs](https://docs.rs/emit/2.21.0/emit/index.html#attributes) for a complete list of attributes defined by `emit`.

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

The [`#[key`](https://docs.rs/emit/2.21.0/emit/attr.key.html) attribute can be used to set the name of a captured property. This can be used to give a property a name that isn't a valid Rust identifier:

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

The [`#[fmt]`](https://docs.rs/emit/2.21.0/emit/attr.fmt.html) attribute applies a formatter to a property value when rendering it in the template. The accepted syntax is the same as Rust's [`std::fmt`](https://doc.rust-lang.org/std/fmt/index.html):

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

The [`#[as_debug]`](https://docs.rs/emit/2.21.0/emit/attr.as_debug.html) attribute captures a property value using its [`Debug`](https://doc.rust-lang.org/std/fmt/trait.Debug.html) implementation, instead of the default `Display + 'static`:

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

The [`#[as_serde]`](https://docs.rs/emit/2.21.0/emit/attr.as_serde.html) attribute captures a property value using its [`Serialize`](https://docs.rs/serde/latest/serde/trait.Serialize.html) implementation, instead of the default `Display + 'static`:

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

## `#[optional]`

The [`#[optional]`](https://docs.rs/emit/2.21.0/emit/attr.optional.html) attribute captures an `Option<&T>` property, omitting the property entirely when the value is `None`:

```rust
# extern crate emit;
let x = Some("some data");

emit::emit!("template {x}", #[emit::optional] x);
```

```text
Event {
    mdl: "my_app",
    tpl: "template {x}",
    extent: Some(
        "2024-10-02T22:06:00.000000000Z",
    ),
    props: {
        "x": "some data",
    },
}
```

When `x` is `None`, the property is omitted from the event:

```rust
# extern crate emit;
let x: Option<&str> = None;

emit::emit!("template {x}", #[emit::optional] x);
```

```text
Event {
    mdl: "my_app",
    tpl: "template {x}",
    extent: Some(
        "2024-10-02T22:06:00.000000000Z",
    ),
    props: {},
}
```

The value must be `Option<&T>`. If you have an `Option<T>`, call `.as_ref()`:

```rust
# extern crate emit;
let x = Some(42);

emit::emit!("template {x}", #[emit::optional] x: x.as_ref());
```

See also [`#[nullable]`](#nullable) if you want the property to be present with a `null` value instead of omitted.

## `#[nullable]`

The [`#[nullable]`](https://docs.rs/emit/2.21.0/emit/attr.nullable.html) attribute captures an `Option<&T>` property, emitting a `null` value when the value is `None`:

```rust
# extern crate emit;
let x = Some("some data");

emit::emit!("template {x}", #[emit::nullable] x);
```

```text
Event {
    mdl: "my_app",
    tpl: "template {x}",
    extent: Some(
        "2024-10-02T22:07:00.000000000Z",
    ),
    props: {
        "x": "some data",
    },
}
```

When `x` is `None`, the property is present with a `null` value:

```rust
# extern crate emit;
let x: Option<&str> = None;

emit::emit!("template {x}", #[emit::nullable] x);
```

```text
Event {
    mdl: "my_app",
    tpl: "template {x}",
    extent: Some(
        "2024-10-02T22:07:00.000000000Z",
    ),
    props: {
        "x": null,
    },
}
```

The value must be `Option<&T>`. If you have an `Option<T>`, call `.as_ref()`:

```rust
# extern crate emit;
let x = Some(42);

emit::emit!("template {x}", #[emit::nullable] x: x.as_ref());
```

This differs from [`#[optional]`](#optional), which omits the property entirely when `None`. Use `#[nullable]` when the presence of the key matters (e.g., to distinguish "not set" from "explicitly null" in downstream systems).
