# Quick debugging

It can be useful when you're actively working on a piece of code to get a quick window into what it's doing by logging data at various points. This kind of diagnostic is useful in the short term, but if left becomes noise. `emit` provides the [`dbg!`](https://docs.rs/emit/1.9.0/emit/macro.dbg.html) macro as a quick alternative to [first-class logging](./logging.md) for these temporary debugging aids.

## The `dbg!` macro

`emit`'s [`dbg!`](https://docs.rs/emit/1.9.0/emit/macro.dbg.html) macro works similarly to [the standard library's of the same name](https://doc.rust-lang.org/std/macro.dbg.html), and shares the same motivations.

When given a field-value expression, `dbg!` will emit an event that includes it along with the source location:

```rust
# extern crate emit;
fn confirm_email(user: &str, email: &str) {
    emit::dbg!(user);

    // ..
}
```

```text
Event {
    mdl: "dbg",
    tpl: "user = {user} at {file}:{line}",
    extent: Some(
        "2025-01-07T03:55:19.738224881Z",
    ),
    props: {
        "file": "main.rs",
        "line": 11,
        "lvl": debug,
        "user": "Rust",
    },
}
```

`dbg!` accepts multiple field-values:

```rust
# extern crate emit;
fn confirm_email(user: &str, email: &str) {
    emit::dbg!(user, email);

    // ..
}
```

```text
Event {
    mdl: "dbg",
    tpl: "email = {email}, user = {user} at {file}:{line}",
    extent: Some(
        "2025-01-07T03:56:02.570025234Z",
    ),
    props: {
        "email": "rust@example.com",
        "file": "main.rs",
        "line": 11,
        "lvl": debug,
        "user": "Rust",
    },
}
```

You can also specify a template, just like in [regular logging](./logging.md):

```rust
# extern crate emit;
fn confirm_email(user: &str, email: &str) {
    emit::dbg!("got {user} and {email}");

    // ..
}
```

```text
Event {
    mdl: "dbg",
    tpl: "got {user} and {email}",
    extent: Some(
        "2025-01-07T03:56:28.150050260Z",
    ),
    props: {
        "email": "rust@example.com",
        "file": "main.rs",
        "line": 11,
        "lvl": debug,
        "user": "Rust",
    },
}
```

In order to be captured by `dbg!`, a value only needs to implement [`Debug`](https://doc.rust-lang.org/std/fmt/trait.Debug.html). This is different from regular logging, where values need to implement [`Display + 'static` by default](../reference/property-capturing.md).

## Where do `dbg!` events go?

`dbg!` events use the same infrastructure as [regular logging](./logging.md). In order to see them, you need to configure `emit` to write them to the console or other destination. See [Getting started](../getting-started.md) and [Emitting events](../emitting-events.md) for more details.

## `dbg!` vs `debug!`

`emit` also defines a [`debug!`](https://docs.rs/emit/1.9.0/emit/macro.debug.html) macro for events supporting live debugging. You should use `dbg!` for temporary logging that helps you actively debug code you're working on. You should use `debug!` for longer-lived logging that's useful for debugging a live system. When writing `debug!` or other logs, you should put more attention into when you're logging and what you're logging, so that you get the most value from the least volume. When writing `dbg!`, you should be unafraid to emit whatever you need to make sense of the code you're working on.

Don't simply convert `dbg!` statements to `debug!` ones. Once you're done with them, you're better off removing `dbg!` altogether. They're unlikely to be useful to you over time.
