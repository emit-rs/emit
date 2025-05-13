# Examples

```ignore
let x = 42;
let y = true;

emit::dbg!(x, y);
```

# Syntax

```text
(property),*
template_literal, (property),*
```

where

- `template_literal`: A template string literal.
- `property`: A Rust field-value for a property to capture.

# Template literals

Templates are text literals that include regular text with _holes_. A hole is a point in the template where a property should be interpolated in.

See [the guide](https://emit-rs.io/reference/templates.html) for more details and examples of templates.

# Properties

Properties that appear within the template or after it are added to the emitted event. The identifier of the property is its key. Property capturing can be adjusted through the `as_*` attribute macros.

Unlike [`macro@debug`], this macro captures values using their [`Debug`](https://doc.rust-lang.org/std/fmt/trait.Debug.html) implementation by default.

See [the guide](https://emit-rs.io/reference/property-capturing.html) for more details on property capturing.

# When to use `dbg`

This macro is a convenient way to pepper debug logs through code, but follows the same recommendations as the standard library's `dbg` macro.
You shouldn't expect `dbg` statements to be long lived, and use the [`macro@debug`] macro instead with more deliberate data.

See [the guide](https://emit-rs.io/producing-events/quick-debugging.html) for more details.
