# Examples

Note that [`macro@debug_evt`], [`macro@info_evt`], [`macro@warn_evt`], and [`macro@error_evt`] use the same syntax.

Creating an event with captured properties in the template:

```ignore
let x = 42;
let y = true;

let evt = emit::evt!("got {x} and {y}");
```

Creating an event with captured properties after the template:

```ignore
let x = 42;
let y = true;

let evt = emit::evt!("something of note", x, y);
```

Specifying control parameters before the template (in this example, `mdl`):

```ignore
let evt = emit::evt!(mdl: emit::path!("a::b"), "something of note");
```

# Syntax

```text
(control_param),* template_literal, (property),*
```

where

- `control_param`: A Rust field-value with a pre-determined identifier (see below).
- `template_literal`: A template string literal.
- `property`: A Rust field-value for a property to capture.

# Control parameters

This macro accepts the following optional control parameters:

| name     | type                    | description                                                                      |
| -------- | ----------------------- | -------------------------------------------------------------------------------- |
| `mdl`    | `impl Into<emit::Path>` | The module the event belongs to. If unspecified the current module path is used. |
| `props`  | `impl emit::Props`      | A base set of properties to add to the event.                                    |
| `extent` | `impl emit::ToExtent`   | The extent to use on the event.                                                  |

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

An `emit::Event`.
