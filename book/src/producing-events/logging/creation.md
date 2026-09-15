# Log event creation

`emit`'s log events are just regular instances of the [`Event`](https://docs.rs/emit/2.23.0/emit/struct.Event.html) type.

## Using macros

When you call the [`emit!`](https://docs.rs/emit/2.23.0/emit/macro.emit.html) macro, an `Event` is created in-place and emitted for you:

```rust
# extern crate emit;
fn confirm_email(user: &str, email: &str) {
    emit::emit!("{user} confirmed {email}");
}
```

You can also construct an event without emitting it using the [`evt!`](https://docs.rs/emit/2.23.0/emit/macro.evt.html) macro:

```rust
# extern crate emit;
fn confirm_email(user: &str, email: &str) {
    let evt = emit::evt!("{user} confirmed {email}");

    // We can choose to emit this event manually
    emit::emit!(evt);
}
```

## Using `Event` directly

You can construct an [`Event`](https://docs.rs/emit/2.23.0/emit/struct.Event.html) directly:

```rust
# extern crate emit;
fn confirm_email(user: &str, email: &str) {
    let evt = emit::Event::new(
        // Where the event came from
        emit::mdl!(),
        // What the event is about
        emit::tpl!("{user} confirmed {email}"),
        // When the event occurred
        emit::clock().now(),
        // Additional properties
        emit::props! {
            user,
            email,
        },
    );

    // We can choose to emit this event manually
    emit::emit(evt);
}
```

The above example still uses macros for the module, template, and additional properties. These can also be constructed manually too. See [Constructing events without macros](../../reference/events.md#constructing-events-without-macros) for details.
