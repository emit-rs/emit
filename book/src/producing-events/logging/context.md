# Ambient context

`emit` supports enriching events automatically with properties from the ambient environment.

## Using the `#[span]` macro

The most straightforward way to add ambient context is using the [`#[span]`](https://docs.rs/emit/0.11.8/emit/attr.span.html) macro. Any properties captured by the span will also be added to events emitted during its execution:

```rust
# extern crate emit;
#[emit::span("greet {user}", lang)]
fn greet(user: &str, lang: &str) {
    match lang {
        "el" => emit::emit!("Γεια σου, {user}"),
        "en" => emit::emit!("Hello, {user}"),
        "eo" => emit::emit!("Saluton, {user}"),
# /*
        ..
# */    _ => (),
    }
}

greet("Rust", "eo");
```

```text
Event {
    mdl: "my_app",
    tpl: "Saluton, {user}",
    extent: Some(
        "2024-10-02T20:53:29.580999000Z",
    ),
    props: {
        "user": "Rust",
        "span_id": b26bfe938b77eb19,
        "lang": "eo",
        "user": "Rust",
        "trace_id": 7fc2efc824915dc180c29a79af358e78,
    },
}
```

Note the presence of the `lang` property on the events produced by `emit!`. They appear because they're added to the ambient context by the `#[span]` attribute on `greet()`.

See [Tracing](../tracing.md) for more details.

## Manually

Ambient context can be worked with directly. `emit` stores its ambient context in an implementation of the [`Ctxt`](https://docs.rs/emit/0.11.8/emit/trait.Ctxt.html) trait. Properties in the ambient context can be added or removed using [`Frame`s](https://docs.rs/emit/0.11.8/emit/frame/struct.Frame.html). You may want to work with context directly when you're not trying to produce spans in a distributed trace, or when your application doesn't have a single point where attributes could be applied to manipulate ambient context.

When converted to use ambient context manually, the previous example looks like this:

```rust
# extern crate emit;
fn greet(user: &str, lang: &str) {
    // Get a frame over the amient context that pushes the `lang` property
    let mut frame = emit::Frame::push(
        emit::ctxt(),
        emit::props! {
            lang,
        },
    );

    // Make the frame active
    // While this guard is in scope the `lang` property will be present
    // When this guard is dropped the `lang` property will be removed
    // Frames may be entered and exited multiple times
    let _guard = frame.enter();

    // The rest of the function proceeds as normal
    match lang {
        "el" => emit::emit!("Γεια σου, {user}"),
        "en" => emit::emit!("Hello, {user}"),
        "eo" => emit::emit!("Saluton, {user}"),
# /*
        ..
# */     _ => (),
    }
}

greet("Rust", "eo");
```

```text
Event {
    mdl: "my_app",
    tpl: "Saluton, {user}",
    extent: Some(
        "2024-10-02T21:01:50.534810000Z",
    ),
    props: {
        "user": "Rust",
        "lang": "eo",
    },
}
```

When using ambient context manually, it's important that frames are treated like a stack. They need to be exited in the opposite order they were entered in.
