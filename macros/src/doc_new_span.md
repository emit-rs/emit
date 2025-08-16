See the [`SpanGuard::new`](https://docs.rs/emit/1.12.0/emit/span/struct.SpanGuard.html#method.new) for details on starting and completing the returned span.

Also see [the guide](https://emit-rs.io/producing-events/tracing/manual-span-creation.html) for more details on manual span construction.

# Examples

Note that [`macro@new_debug_span`], [`macro@new_info_span`], [`macro@new_warn_span`], and [`macro@new_error_span`] use the same syntax.

Creating a span with captured properties in the template:

```ignore
let x = 42;
let y = true;

let (span, guard) = emit::new_span!("got {x} and {y}");
```

Creating a span with captured properties after the template:

```ignore
let x = 42;
let y = true;

let (span, guard) = emit::new_span!("something of note", x, y);
```

Specifying control parameters before the template (in this example, `mdl`):

```ignore
let (span, guard) = emit::new_span!(mdl: emit::path!("a::b"), "something of note");
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

| name        | type                          | description                                                                                                                                                    |
| ----------- | ----------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `rt`        | `impl emit::runtime::Runtime` | The runtime to emit the event through.                                                                                                                         |
| `mdl`       | `impl Into<emit::Path>`       | The module the event belongs to. If unspecified the current module path is used.                                                                               |
| `when`      | `impl emit::Filter`           | A filter to use instead of the one configured on the runtime.                                                                                                  |
| `panic_lvl` | `str` or `emit::Level`        | Detect whether the function panics and use the given level if it does.                                                                                         |

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

A `(SpanGuard, Frame<impl Ctxt>)`.
