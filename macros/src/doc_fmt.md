This macro is to [`macro@tpl`] and [`macro@emit`] what the standard library's `format!()` is to `format_args!()` and `write!()`. It formats a template into a string directly instead of constructing an event or emitting it.

# Examples

Formatting a template with captured properties in the template:

```ignore
let x = 42;
let y = true;

let formatted = emit::format!("got {x} and {y}");
```

Formatting a template with captured properties after the template:

```ignore
let x = 42;
let y = true;

let formatted = emit::format!("something of note", x, y);
```

# Syntax

This macro uses the same syntax as [`macro@emit`].

```text
(control_param),* template_literal, (property),*
```

where

- `control_param`: A Rust field-value with a pre-determined identifier (see below).
- `template_literal`: A template string literal (see below).
- `property`: A Rust field-value for a property to capture.

# Control parameters

This macro doesn't accept any control parameters.

# Template literals

Templates are text literals that include regular text with _holes_. A hole is a point in the template where a property should be interpolated in.

- `template_literal`: `"` `(text | hole)*` `"`
- `text`: A fragment of plain text where `{` are escaped as `{{` and `}` are escaped as `}}`.
- `hole`: `{` `property` `}`
- `property`: A Rust field-value expression.

The following are all examples of templates:

```text
"some text"
 ├───────┘
 text
```

```text
"some text and {x}"
 ├────────────┘ │
 text           property
```

```text
"some {{text}} and {x: 42} and {y}"
 ├────────────────┘ ├───┘ └───┤ │
 text               property  │ property
                              text
```

See [the guide](https://emit-rs.io/reference/templates.html) for more details and examples of templates.

# Properties

Properties that appear within the template or after it are added to the emitted event. The identifier of the property is its key. Property capturing can be adjusted through the `as_*` attribute macros.

See [the guide](https://emit-rs.io/reference/property-capturing.html) for more details on property capturing.

# Returns

A `String`.
