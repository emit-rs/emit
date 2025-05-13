Event emission is non-blocking.

See [the guide](https://emit-rs.io/producing-events/logging.html) for more details on logging.

# Examples

Note that [`macro@debug`], [`macro@info`], [`macro@warn`], and [`macro@error`] use the same syntax.

Emitting an event with captured properties in the template:

```ignore
let x = 42;
let y = true;

emit::emit!("got {x} and {y}");
```

Emitting an event with captured properties after the template:

```ignore
let x = 42;
let y = true;

emit::emit!("something of note", x, y);
```

Specifying control parameters before the template (in this example, `mdl`):

```ignore
emit::emit!(mdl: emit::path!("a::b"), "something of note");
```

# Syntax

```text
(control_param),* template_literal, (property),*
```

where

- `control_param`: A Rust field-value with a pre-determined identifier (see below).
- `template_literal`: A template string literal (see below).
- `property`: A Rust field-value for a property to capture.

# Control parameters

This macro accepts the following optional control parameters:

| name      | type                          | description                                                                                                                                                                                    |
| --------- | ----------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `rt`      | `impl emit::runtime::Runtime` | The runtime to emit the event through.                                                                                                                                                         |
| `mdl`     | `impl Into<emit::Path>`       | The module the event belongs to. If unspecified the current module path is used.                                                                                                               |
| `extent`  | `impl emit::ToExtent`         | The extent to use on the event. If it resolves to `None` then the clock on the runtime will be used to assign a point extent.                                                                  |
| `props`   | `impl emit::Props`            | A base set of properties to add to the event.                                                                                                                                                  |
| `evt`     | `impl emit::event::ToEvent`   | A base event to emit. Any properties captured by the macro will be appended to the base event. If this control parameter is specified then `mdl`, `props`, and `extent` cannot also be set. |
| `when`    | `impl emit::Filter`           | A filter to use instead of the one configured on the runtime.                                                                                                                                  |

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
