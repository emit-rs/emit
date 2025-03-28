# Logging

Logs provide immediate feedback on the operation of your applications. Logs let you capture events that are significant in your application's domain, like orders being placed. Logs also help you debug issues in live applications, like lost order confirmation emails.

## The `emit!` macro

In `emit`, logs are events; a combination of timestamp, message template, and properties. When something significant happens in the execution of your applications, you can log an event for it:

```rust
# extern crate emit;
fn confirm_email(user: &str, email: &str) {
    emit::emit!("{user} confirmed {email}");
}
```

```text
Event {
    mdl: "my_app",
    tpl: "{user} confirmed {email}",
    extent: Some(
        "2024-10-01T22:21:08.136228524Z",
    ),
    props: {
        "email": "user-123@example.com",
        "user": "user-123",
    },
}
```

`emit` also defines macros for emitting events at different levels for filtering:

- [`debug!`](https://docs.rs/emit/1.6.0/emit/macro.debug.html) for events supporting live debugging.
- [`info!`](https://docs.rs/emit/1.6.0/emit/macro.info.html) for most informative events.
- [`warn!`](https://docs.rs/emit/1.6.0/emit/macro.warn.html) for errors that didn't cause the calling operation to fail.
- [`error!`](https://docs.rs/emit/1.6.0/emit/macro.error.html) for errors that caused the calling operation to fail.

See [Levels](./logging/levels.md) for details.

To learn more about `emit`'s macro syntax, see [Template syntax and rendering](../reference/templates.md).

-----

![an example log rendered to the console](../asset/term-err.png)

_An example log produced by `emit` rendered to the console_
