See [the guide](https://emit-rs.io/producing-events/tracing.html) for more details on tracing and span construction, including more advanced use cases.

# Examples

Note that [`macro@debug_span`], [`macro@info_span`], [`macro@warn_span`], and [`macro@error_span`] use the same syntax.

Emitting an event with captured properties in the template:

```ignore
#[emit::span!("executing with {x} and {y}")]
fn exec(x: i32, y: bool) {
    // Your code goes here
}
```

Emitting an event with captured properties after the template:

```ignore
#[emit::span!("executing", x, y)]
fn exec(x: i32, y: bool) {
    // Your code goes here
}
```

Specifying control parameters before the template (in this example, `mdl`):

```ignore
#[emit::span!(mdl: emit::path!("a::b"), "executing")]
fn exec(x: i32, y: bool) {
    // Your code goes here
}
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
| `guard`     | -                             | An identifier to bind an `emit::SpanGuard` to in the body of the span for manual completion.                                                                        |
| `ok_lvl`    | `str` or `emit::Level`        | Assume the instrumented block returns a `Result`. Assign the event the given level when the result is `Ok`.                                                    |
| `err_lvl`   | `str` or `emit::Level`        | Assume the instrumented block returns a `Result`. Assign the event the given level when the result is `Err` and attach the error as the `err` property.        |
| `panic_lvl` | `str` or `emit::Level`        | Detect whether the function panics and use the given level if it does.                                                                                         |
| `err`       | `impl Fn(&E) -> T`            | Assume the instrumented block returns a `Result`. Map the `Err` variant into a new type `T` that is `str`, `&(dyn Error + 'static)`, or `impl Error + 'static` |
| `setup`     | `impl Fn() -> T`              | Invoke the expression before creating the span, binding the result to a value that's dropped at the end of the annotated function.                             |

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
