Templates are text literals that include regular text with _holes_. A hole is a point in the template where a property should be interpolated in.

# Examples

Creating a template with captured properties in the literal:

```ignore
let x = 42;
let y = true;

let template = emit::tpl!("got {x} and {y}");
```

Creating a template with captured properties after the literal:

```ignore
let x = 42;
let y = true;

let template = emit::tpl!("something of note", x, y);
```

# Syntax

```text
template_literal
```

where

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

# Returns

An `emit::Template`.
