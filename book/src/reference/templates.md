# Template syntax and rendering

## Producing templates

`emit` templates are string literals with holes for properties between braces. This is an example of a template:

```rust
# extern crate emit;
let user = "Rust";

emit::emit!("Hello, {user}");
```

The [`emit!`](https://docs.rs/emit/1.4.1/emit/macro.emit.html) and [`#[span]`](https://docs.rs/emit/1.4.1/emit/attr.span.html) macros use the same syntax.

### Properties within templates

Properties in templates appear within braces:

```rust
# extern crate emit;
# let user = "Rust";
emit::emit!("Hello, {user}");
```

Braces may be escaped by doubling them:

```rust
# extern crate emit;
emit::emit!("Hello, {{user}}");
```

Properties use Rust's field value syntax, like you'd write when initializing struct fields. Usually they're a standalone identifer that will capture a property in scope with that name. Properties can also be given a value inline as an expression:

```rust
# extern crate emit;
emit::emit!("Hello, {user: \"Rust\"}");
```

Properties may have attributes applied to them:

```rust
# extern crate emit;
# let user = "Rust";
emit::emit!("Hello, {#[cfg(enabled)] user}")
```

See [Property attributes](./property-attributes.md) for details on attributes you can apply. Also see [Property capturing](./property-capturing.md) for details on what types of properties can be captured.

### Properties after templates

Complex property expressions are distracting within templates. Attributes and values for properties declared in the template can be written after it using the same field-value syntax:

```rust
# extern crate emit;
emit::emit!(
    "Hello, {user}",
    #[cfg(enabled)]
    user: "Rust",
);
```

Properties outside of the template don't need a corresponding hole to be captured:

```rust
# extern crate emit;
# let user = "Rust";
emit::emit!(
    "Hello, {user}",
    lang: "en",
);
```

### Properties before templates

Properties declared before the template aren't captured. They're called _control parameters_ and are used to change the way events are constructed or emitted:

```rust
# extern crate emit;
# let user = "Rust";
emit::emit!(
    mdl: emit::path!("a::b::c"),
    "Hello, {user}",
)
```

The names and values of control parameters are different between `emit!` and `#[span]`. See [Control parameters](./control-parameters.md) for details.

## Rendering templates

Templates are tokenized into sequences of text and holes for property interpolation:

```text
Hello, {user}
```

When tokenized, this template will look like:

```rust
# extern crate emit;
# use emit::template::Part;
let tokens = [
    Part::text("Hello, "),
    Part::hole("user"),
];
```

The template can then be fed a value for `user` and rendered:

```rust
# extern crate emit;
# use emit::{Template, template::Part};
# let tokens = [Part::text("Hello, "), Part::hole("user")];
let template = Template::new_ref(&tokens);

let rendered = template.render(("user", "Rust")).to_string();
# assert_eq!("Hello, Rust", rendered);
```

which will produce:

```text
Hello, Rust
```

Any holes in the template that are rendered without a matching property will reproduce the hole:

```rust
# extern crate emit;
# use emit::{Template, template::Part};
# let tokens = [Part::text("Hello, "), Part::hole("user")];
let template = Template::new_ref(&tokens);

let rendered = template.render(emit::Empty).to_string();
# assert_eq!("Hello, {user}", rendered);
```

```text
Hello, {user}
```

You can control how properties are rendered within templates by implementing the [`template::Write`](https://docs.rs/emit/1.4.1/emit/template/trait.Write.html) trait. `emit_term` uses this for example to render different property types in different colors.
