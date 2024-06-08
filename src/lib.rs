/*!
Structured diagnostics for Rust applications.

`emit` is a structured logging framework for manually instrumenting Rust applications with an expressive syntax inspired by [Message Templates](https://messagetemplates.org).

# A guided tour of `emit`

To get started, add `emit` to your `Cargo.toml`:

```toml
[dependencies.emit]
version = "*"
```

## Configuring an emitter

`emit` needs to be configured with at least an [`Emitter`] that receives events, otherwise your diagnostics will go nowhere. At the start of your `main` function, use [`setup()`] to initialize `emit`. At the end of your `main` function, use [`setup::Init::blocking_flush`] to ensure all emitted events are fully flushed before returning.

Here's an example of a simple configuration with an emitter that prints events using [`std::fmt`]:

```
# #[cfg(not(feature = "std"))] fn main() {}
# #[cfg(feature = "std")]
fn main() {
    let rt = emit::setup()
        .emit_to(emit::emitter::from_fn(|evt| println!("{evt:#?}")))
        .init();

    // Your app code goes here

    rt.blocking_flush(std::time::Duration::from_secs(5));
}
```

In real applications, you'll want to use a more sophisticated emitter, such as:

- [`emit_term`](https://docs.rs/emit_term/0.11.0-alpha.2/emit_term/index.html): Emit diagnostics to the console.
- [`emit_file`](https://docs.rs/emit_file/0.11.0-alpha.2/emit_file/index.html): Emit diagnostics to a set of rolling files.
- [`emit_otlp`](https://docs.rs/emit_otlp/0.11.0-alpha.2/emit_otlp/index.html): Emit diagnostics to a remote collector via OpenTelemetry Protocol.

For more advanced setup options, see the [`mod@setup`] module.

## Emitting events

Producing useful diagnostics in your applications is a critical aspect of building robust and maintainable software. If you don't find your diagnostics useful in development, then you won't find them useful in production either when you really need them. `emit` is a tool for application developers, designed to be straightforward to configure and integrate into your projects with low conceptual overhead.

`emit` uses macros with a special syntax to log events. Here's an example of an event:

```
# #[cfg(not(feature = "std"))] fn main() {}
# #[cfg(feature = "std")] fn main() {
emit::emit!("Hello, World");
# }
```

Using the `std::fmt` emitter from earlier, it will output:

```text
Event {
    module: "my_app",
    tpl: "Hello, world!",
    extent: Some(
        "2024-04-23T10:04:37.632304000Z",
    ),
    props: {},
}
```

This example is a perfect opportunity to introduce `emit`'s model of diagnostics: events. An event is a notable change in the state of a system that is broadcast to outside observers. Events are the combination of:

- `module`: The component that raised the event.
- `tpl`: A text template that can be rendered to describe the event.
- `extent`: The point in time when the event occurred, or the timespan for which it was active.
- `props`: A set of key-value pairs that define the event and capture the context surrounding it.

`emit`'s events are general enough to represent many observability paradigms including logs, distributed traces, metric samples, and more.

### Properties in templates

The string literal argument to the [`emit!`] macro is its template. Properties can be attached to events by interpolating them into the template between braces:

```
# #[cfg(not(feature = "std"))] fn main() {}
# #[cfg(feature = "std")] fn main() {
let user = "World";

emit::emit!("Hello, {user}!");
# }
```

```text
Event {
    module: "my_app",
    tpl: "Hello, `user`!",
    extent: Some(
        "2024-04-25T00:53:36.364794000Z",
    ),
    props: {
        "user": "World",
    },
}
```

`emit` uses Rust's field value syntax between braces in its templates, where the identifier becomes the key of the property. This is the same syntax used for struct field initialization. The above example could be written equivalently in other ways:

```
# #[cfg(not(feature = "std"))] fn main() {}
# #[cfg(feature = "std")] fn main() {
let greet = "World";

emit::emit!("Hello, {user: greet}!");
# }
```

```
# #[cfg(not(feature = "std"))] fn main() {}
# #[cfg(feature = "std")] fn main() {
emit::emit!("Hello, {user: \"World\"}!");
# }
```

In these examples we've been using the string `"World"` as the property value. Other primitive types such as booleans, integers, floats, and most library-defined datastructures like UUIDs and URIs can be captured by default in templates.

### Properties outside templates

Additional properties can be added to an event by listing them as field values after the template:

```
# #[cfg(not(feature = "std"))] fn main() {}
# #[cfg(feature = "std")] fn main() {
let user = "World";
let greeter = "emit";
let lang = "en";

emit::emit!("Hello, {user}!", lang, greeter);
# }
```

```text
Event {
    module: "my_app",
    tpl: "Hello, `user`!",
    extent: Some(
        "2024-04-25T22:32:23.013651640Z",
    ),
    props: {
        "greeter": "emit",
        "lang": "en",
        "user": "World",
    },
}
```

Properties inside templates may be initialized outside of them:

```
# #[cfg(not(feature = "std"))] fn main() {}
# #[cfg(feature = "std")] fn main() {
emit::emit!("Hello, {user}", user: "World");
# }
```

```text
Event {
    module: "my_app",
    tpl: "Hello, `user`",
    extent: Some(
        "2024-04-25T22:34:25.536438193Z",
    ),
    props: {
        "user": "World",
    },
}
```

### Controlling event construction and emission

Control parameters appear before the template. They use the same field value syntax as properties, but aren't captured as properties on the event. They instead customize other aspects of the event.

The `module` control parameter sets the module:

```
# #[cfg(not(feature = "std"))] fn main() {}
# #[cfg(feature = "std")] fn main() {
emit::emit!(module: "my_module", "Hello, World!");
# }
```

```text
Event {
    module: "my_module",
    tpl: "Hello, World!",
    extent: Some(
        "2024-04-25T22:42:52.180968127Z",
    ),
    props: {},
}
```

The `extent` control parameter sets the extent:

```
# #[cfg(not(feature = "std"))] fn main() {}
# #[cfg(feature = "std")] fn main() {
# use std::time::Duration;
emit::emit!(
    extent: emit::Timestamp::from_unix(Duration::from_secs(1000000000)),
    "Hello, World!",
);
# }
```

```text
Event {
    module: "my_app",
    tpl: "Hello, World!",
    extent: Some(
        "2001-09-09T01:46:40.000000000Z",
    ),
    props: {},
}
```

The `props` control parameter adds a base set of [`Props`] to an event in addition to any added through the template:

```
# #[cfg(not(feature = "std"))] fn main() {}
# #[cfg(feature = "std")] fn main() {
emit::emit!(
    props: emit::props! {
        lang: "en",
    },
    "Hello, {user}!",
    user: "World",
);
# }
```

```text
Event {
    module: "my_app",
    tpl: "Hello, `user`!",
    extent: Some(
        "2024-04-29T03:36:07.751177000Z",
    ),
    props: {
        "lang": "en",
        "user": "World",
    },
}
```

### Controlling property capturing

Property capturing is controlled by regular Rust attributes. For example, the [`key`] attribute can be used to set the key of a property to an arbitrary string:

```
# #[cfg(not(feature = "std"))] fn main() {}
# #[cfg(feature = "std")] fn main() {
emit::emit!(
    "Hello, {user}!",
    #[emit::key("user.name")]
    user: "World",
);
# }
```

```text
Event {
    module: "my_app",
    tpl: "Hello, `user.name`!",
    extent: Some(
        "2024-04-26T06:33:59.159727258Z",
    ),
    props: {
        "user.name": "World",
    },
}
```

By default, properties are captured based on their [`std::fmt::Display`] implementation. This can be changed by applying one of the `as` attributes to the property. For example, applying the [`as_debug`] attribute will capture the property using uts [`std::fmt::Debug`] implementation instead:

```
# #[cfg(not(feature = "std"))] fn main() {}
# #[cfg(feature = "std")] fn main() {
#[derive(Debug)]
struct User {
    name: &'static str,
}

emit::emit!(
    "Hello, {user}!",
    #[emit::as_debug]
    user: User {
        name: "World",
    },
);
# }
```

```text
Event {
    module: "my_app",
    tpl: "Hello, `user`!",
    extent: Some(
        "2024-04-26T06:29:55.778795150Z",
    ),
    props: {
        "user": User {
            name: "World",
        },
    },
}
```

In the above example, the structure of the `user` property is lost. It can be formatted using the `std::fmt` machinery, but if serialized to JSON it would produce a string:

```json
{
    "module": "my_app",
    "tpl": "Hello, `user`!",
    "extent": "2024-04-26T06:29:55.778795150Z",
    "props": {
        "user": "User { name: \"World\" }"
    }
}
```

To retain the structure of complex values, you can use the [`as_serde`] or [`as_sval`] attributes:

```
# #[cfg(not(all(feature = "std", feature = "serde")))] fn main() {}
# #[cfg(all(feature = "std", feature = "serde"))]
# fn main() {
#[derive(serde::Serialize)]
struct User {
    name: &'static str,
}

emit::emit!(
    "Hello, {user}!",
    #[emit::as_serde]
    user: User {
        name: "World",
    },
);
# }
```

Represented as JSON, this event will instead produce:

```json
{
    "module": "my_app",
    "tpl": "Hello, `user`!",
    "extent": "2024-04-26T06:29:55.778795150Z",
    "props": {
        "user": {
            "name": "World"
        }
    }
}
```

Primitive types like booleans and numbers are always structure-preserving, so `as` attributes don't need to be applied to them.

## Filtering events

Managing the volume of diagnostic data is an important activity in application development to keep costs down and make debugging more efficient. Ideally, applications would only produce useful diagnostics, but reality demands tooling to limit volume at a high-level. `emit` lets you configure a [`Filter`] during [`setup()`] to reduce the volume of diagnostic data. A useful filter is [`level::min_by_path_filter`], which only emits events when they are produced for at least a given [`Level`] within a given module:

```
# #[cfg(not(feature = "std"))] fn main() {}
# #[cfg(feature = "std")]
fn main() {
    let rt = emit::setup()
        .emit_to(emit::emitter::from_fn(|evt| println!("{evt:#?}")))
        .emit_when(emit::level::min_by_path_filter([
            ("my_app::submodule", emit::Level::Info)
        ]))
        .init();

    emit::debug!("Up and running");

    submodule::greet("World");

    rt.blocking_flush(std::time::Duration::from_secs(5));
}

# #[cfg(feature = "std")]
mod submodule {
    pub fn greet(user: &str) {
        emit::debug!("Preparing to greet {user}");
        emit::info!("Hello, {user}!");
    }
}
```

```text
Event {
    module: "my_app",
    tpl: "Up and running",
    extent: Some(
        "2024-04-29T04:31:24.085826100Z",
    ),
    props: {
        "lvl": debug,
    },
}
Event {
    module: "my_app::submodule",
    tpl: "Hello, `user`!",
    extent: Some(
        "2024-04-29T04:31:24.086327500Z",
    ),
    props: {
        "lvl": info,
        "user": "World",
    },
}
```

Filters can apply to any feature of a candidate event. For example, this filter only matches events with a property `lang` that matches `"en"`:

```
# #[cfg(not(feature = "std"))] fn main() {}
# #[cfg(feature = "std")] fn main() {
# let e =
emit::filter::from_fn(|evt| {
    use emit::Props as _;

    evt.props().pull::<emit::Str, _>("lang") == Some(emit::Str::new("en"))
});
# ;
# }
```

The `when` control parameter of the emit macros can be used to override the globally configured filter for a specific event:

```
# #[cfg(not(feature = "std"))] fn main() {}
# #[cfg(feature = "std")] fn main() {
// This event matches the filter
emit::emit!(
    when: emit::filter::from_fn(|evt| evt.module() == "my_app"),
    "Hello, World!",
);
# }
```

```text
Event {
    module: "my_app",
    tpl: "Hello, World!",
    extent: Some(
        "2024-04-25T22:54:50.055493407Z",
    ),
    props: {},
}
```

```
# #[cfg(not(feature = "std"))] fn main() {}
# #[cfg(feature = "std")] fn main() {
// This event does not match the filter
emit::emit!(
    when: emit::filter::from_fn(|evt| evt.module() == "my_app"),
    module: "not_my_app",
    "Hello, World!",
);
# }
```

```text

```

This can be useful to guarantee an event will always be emitted, regardless of any filter configuration:

```
# #[cfg(not(feature = "std"))] fn main() {}
# #[cfg(feature = "std")] fn main() {
// This event is never filtered out
emit::emit!(
    when: emit::filter::always(),
    "Hello, World!",
);
# }
```

## Tracing and metrics

`emit` can represent spans in distributed traces and metric samples as events using well-known properties. See the [`mod@span`] and [`mod@metric`] modules for details on each.

## Rendering templates

Templates are parsed at compile-time, but are rendered at runtime by passing the properties they capture back. The [`Event::msg`] method is a convenient way to render the template of an event using its properties. Taking an earlier example:

```
# #[cfg(not(feature = "std"))] fn main() {}
# #[cfg(feature = "std")] fn main() {
let user = "World";

emit::emit!("Hello, {user}!");
# }
```

If we change our emitter to:

```
# #[cfg(not(feature = "std"))] fn main() {}
# #[cfg(feature = "std")] fn main() {
# let e =
emit::emitter::from_fn(|evt| println!("{}", evt.msg()))
# ;
# }
```

then it will produce the output:

```text
Hello, World!
```

## Custom runtimes

Everything in `emit` is based on a [`runtime::Runtime`]; a fully isolated set of components that provide capabilities like clocks and randomness, as well as your configured emitters and filters. When a runtime isn't specified, it's [`runtime::shared`]. You can define your own runtimes too. The [`mod@setup`] module has more details.

## Troubleshooting

Emitters write their own diagnostics to an alternative `emit` runtime, which you can configure to debug them:

```
# mod emit_term { pub fn stdout() -> impl emit::runtime::InternalEmitter { emit::runtime::AssertInternal(emit::emitter::from_fn(|_| {})) } }
# #[cfg(not(feature = "std"))] fn main() {}
# #[cfg(feature = "std")]
fn main() {
    // Configure the internal runtime before your regular setup
    let internal_rt = emit::setup()
        .emit_to(emit_term::stdout())
        .init_internal();

    let rt = emit::setup()
        .emit_to(emit::emitter::from_fn(|evt| println!("{evt:#?}")))
        .init();

    // Your app code goes here

    rt.blocking_flush(std::time::Duration::from_secs(5));

    // Flush the internal runtime after your regular setup
    internal_rt.blocking_flush(std::time::Duration::from_secs(5));
}
```
*/

#![doc(html_logo_url = "https://raw.githubusercontent.com/emit-rs/emit/main/asset/logo.svg")]

#![deny(missing_docs)]
#![cfg_attr(not(any(test, feature = "std")), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;
extern crate core;

/**
Get a [`Path`] of the executing module for use in [`Event::module`].
*/
#[macro_export]
macro_rules! module {
    () => {
        $crate::Path::new($crate::__private::core::module_path!())
    };
}

#[doc(inline)]
pub use emit_macros::*;

#[doc(inline)]
pub use emit_core::*;

pub mod frame;
pub mod kind;
pub mod level;
pub mod metric;
pub mod platform;
pub mod span;
pub mod timer;

pub use self::{
    clock::Clock, ctxt::Ctxt, emitter::Emitter, empty::Empty, event::Event, extent::Extent,
    filter::Filter, frame::Frame, kind::Kind, level::Level, metric::Metric, path::Path,
    props::Props, rng::Rng, span::Span, str::Str, template::Template, timer::Timer,
    timestamp::Timestamp, value::Value,
};

mod macro_hooks;

#[cfg(feature = "std")]
pub mod setup;
#[cfg(feature = "std")]
pub use setup::{setup, Setup};

#[doc(hidden)]
pub mod __private {
    pub extern crate core;
    pub use crate::macro_hooks::*;
}

mod internal {
    pub struct Erased<T>(pub(crate) T);
}
