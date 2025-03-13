/*!
Implementation details for `emit!` macros.

This crate is not intended to be consumed directly.
*/

/*
# Organization

This crate contains the proc-macros that are exported in the `emit` crate. It expands to code that uses the `emit::__private` API, in particular the `emit::macro_hooks` module.

# Hooks

Code is transformed through _hooks_. A hook is a well-known method call, like `a.__private_emit_capture_as_default()`. The behavior of the hook is defined in `emit::macro_hooks`. Attribute macros look for these hooks and replace them to change behavior. For example, `#[emit::as_debug]` looks for any `__private_emit_capture_as_*` method and replaces it with `__private_emit_capture_as_debug`.

# Testing

Tests for this project mostly live in the top-level `test/ui` crate.
*/

#![deny(missing_docs)]
#![doc(html_logo_url = "https://raw.githubusercontent.com/emit-rs/emit/main/asset/logo.svg")]

extern crate proc_macro;

#[macro_use]
extern crate quote;

#[macro_use]
extern crate syn;

use std::collections::HashMap;

use proc_macro2::TokenStream;

mod args;
mod build;
mod capture;
mod dbg;
mod emit;
mod fmt;
mod format;
mod hook;
mod key;
mod optional;
mod props;
mod span;
mod template;
mod util;

use util::ResultToTokens;

/**
The set of hooks defined as a map.

Hooks are regular attribute macros, but will be eagerly applied when expanding other macros to avoid the nightly-only feature that allows attribute macros on expressions. The `hook::eval_hooks` function will do this expansion.
*/
fn hooks() -> HashMap<&'static str, fn(TokenStream, TokenStream) -> syn::Result<TokenStream>> {
    let mut map = HashMap::new();

    map.insert(
        "fmt",
        (|args: TokenStream, expr: TokenStream| {
            fmt::rename_hook_tokens(fmt::RenameHookTokens { args, expr })
        }) as fn(TokenStream, TokenStream) -> syn::Result<TokenStream>,
    );

    map.insert(
        "key",
        (|args: TokenStream, expr: TokenStream| {
            key::rename_hook_tokens(key::RenameHookTokens { args, expr })
        }) as fn(TokenStream, TokenStream) -> syn::Result<TokenStream>,
    );

    map.insert(
        "optional",
        (|args: TokenStream, expr: TokenStream| {
            optional::rename_hook_tokens(optional::RenameHookTokens { args, expr })
        }) as fn(TokenStream, TokenStream) -> syn::Result<TokenStream>,
    );

    map.insert(
        "as_value",
        (|args: TokenStream, expr: TokenStream| {
            capture_as(
                "as_value",
                args,
                expr,
                quote!(__private_capture_as_value),
                quote!(__private_capture_anon_as_value),
            )
        }) as fn(TokenStream, TokenStream) -> syn::Result<TokenStream>,
    );

    map.insert(
        "as_debug",
        (|args: TokenStream, expr: TokenStream| {
            capture_as(
                "as_debug",
                args,
                expr,
                quote!(__private_capture_as_debug),
                quote!(__private_capture_anon_as_debug),
            )
        }) as fn(TokenStream, TokenStream) -> syn::Result<TokenStream>,
    );

    map.insert(
        "as_display",
        (|args: TokenStream, expr: TokenStream| {
            capture_as(
                "as_display",
                args,
                expr,
                quote!(__private_capture_as_display),
                quote!(__private_capture_anon_as_display),
            )
        }) as fn(TokenStream, TokenStream) -> syn::Result<TokenStream>,
    );

    map.insert(
        "as_sval",
        (|args: TokenStream, expr: TokenStream| {
            #[cfg(feature = "sval")]
            {
                capture_as(
                    "as_sval",
                    args,
                    expr,
                    quote!(__private_capture_as_sval),
                    quote!(__private_capture_anon_as_sval),
                )
            }
            #[cfg(not(feature = "sval"))]
            {
                use syn::spanned::Spanned;

                let _ = args;

                Err(syn::Error::new(expr.span(), "capturing with `sval` is only possible when the `sval` Cargo feature is enabled"))
            }
        }) as fn(TokenStream, TokenStream) -> syn::Result<TokenStream>
    );

    map.insert(
        "as_serde",
        (|args: TokenStream, expr: TokenStream| {
            #[cfg(feature = "serde")]
            {
                capture_as(
                    "as_serde",
                    args,
                    expr,
                    quote!(__private_capture_as_serde),
                    quote!(__private_capture_anon_as_serde),
                )
            }
            #[cfg(not(feature = "serde"))]
            {
                use syn::spanned::Spanned;

                let _ = args;

                Err(syn::Error::new(expr.span(), "capturing with `serde` is only possible when the `serde` Cargo feature is enabled"))
            }
        }) as fn(TokenStream, TokenStream) -> syn::Result<TokenStream>
    );

    map.insert(
        "as_error",
        (|args: TokenStream, expr: TokenStream| {
            #[cfg(feature = "std")]
            {
                capture_as(
                    "as_error",
                    args,
                    expr,
                    quote!(__private_capture_as_error),
                    quote!(__private_capture_as_error),
                )
            }
            #[cfg(not(feature = "std"))]
            {
                use syn::spanned::Spanned;

                let _ = args;

                Err(syn::Error::new(
                    expr.span(),
                    "capturing errors is only possible when the `std` Cargo feature is enabled",
                ))
            }
        }) as fn(TokenStream, TokenStream) -> syn::Result<TokenStream>,
    );

    map
}

/**
Format a template.

# Syntax

See the [`macro@emit`] macro for syntax.

# Control parameters

This macro doesn't accept any control parameters.

# Returns

A `String`.
*/
#[proc_macro]
pub fn format(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    format::expand_tokens(format::ExpandTokens {
        input: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

/**
Construct an event.

# Syntax

```text
(control_param),* tpl, (property),*
```

where

- `control_param`: A Rust field-value with a pre-determined identifier (see below).
- `tpl`: A template string literal.
- `property`: A Rust field-value for a property to capture.

# Control parameters

This macro accepts the following optional control parameters:

| name     | type                    | description                                                                      |
| -------- | ----------------------- | -------------------------------------------------------------------------------- |
| `mdl`    | `impl Into<emit::Path>` | The module the event belongs to. If unspecified the current module path is used. |
| `props`  | `impl emit::Props`      | A base set of properties to add to the event.                                    |
| `extent` | `impl emit::ToExtent`   | The extent to use on the event.                                                  |

# Template

The template for the event. See the [`macro@tpl`] macro for syntax.

# Properties

Properties that appear within the template or after it are added to the emitted event. The identifier of the property is its key. Property capturing can be adjusted through the `as_*` attribute macros.

# Returns

An `emit::Event`.
*/
#[proc_macro]
pub fn evt(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    build::expand_evt_tokens(build::ExpandEvtTokens {
        level: None,
        input: item.into(),
    })
    .unwrap_or_compile_error()
}

/**
Construct a debug event.

# Syntax

See the [`macro@evt`] macro for syntax.

# Returns

An `emit::Event`.
*/
#[proc_macro]
pub fn debug_evt(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    build::expand_evt_tokens(build::ExpandEvtTokens {
        level: Some(quote!(emit::Level::Debug)),
        input: item.into(),
    })
    .unwrap_or_compile_error()
}

/**
Construct an info event.

# Syntax

See the [`macro@evt`] macro for syntax.

# Returns

An `emit::Event`.
*/
#[proc_macro]
pub fn info_evt(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    build::expand_evt_tokens(build::ExpandEvtTokens {
        level: Some(quote!(emit::Level::Info)),
        input: item.into(),
    })
    .unwrap_or_compile_error()
}

/**
Construct a warn event.

# Syntax

See the [`macro@evt`] macro for syntax.

# Returns

An `emit::Event`.
*/
#[proc_macro]
pub fn warn_evt(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    build::expand_evt_tokens(build::ExpandEvtTokens {
        level: Some(quote!(emit::Level::Warn)),
        input: item.into(),
    })
    .unwrap_or_compile_error()
}

/**
Construct an error event.

# Syntax

See the [`macro@evt`] macro for syntax.

# Returns

An `emit::Event`.
*/
#[proc_macro]
pub fn error_evt(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    build::expand_evt_tokens(build::ExpandEvtTokens {
        level: Some(quote!(emit::Level::Error)),
        input: item.into(),
    })
    .unwrap_or_compile_error()
}

/**
Wrap an operation in a span.

# Syntax

```text
(control_param),* tpl, (property),*
```

where

- `control_param`: A Rust field-value with a pre-determined identifier (see below).
- `tpl`: A template string literal.
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

# Template

The template for the event. See the [`macro@tpl`] macro for syntax.

# Properties

Properties that appear within the template or after it are added to the emitted event. The identifier of the property is its key. Property capturing can be adjusted through the `as_*` attribute macros.
*/
#[proc_macro_attribute]
pub fn span(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    span::expand_tokens(span::ExpandTokens {
        level: None,
        input: TokenStream::from(args),
        item: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

/**
Wrap an operation in a debug span.

# Syntax

See the [`macro@span`] macro for syntax.
*/
#[proc_macro_attribute]
pub fn debug_span(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    span::expand_tokens(span::ExpandTokens {
        level: Some(quote!(emit::Level::Debug)),
        input: TokenStream::from(args),
        item: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

/**
Wrap an operation in an info span.

# Syntax

See the [`macro@span`] macro for syntax.
*/
#[proc_macro_attribute]
pub fn info_span(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    span::expand_tokens(span::ExpandTokens {
        level: Some(quote!(emit::Level::Info)),
        input: TokenStream::from(args),
        item: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

/**
Wrap an operation in a warn span.

# Syntax

See the [`macro@span`] macro for syntax.
*/
#[proc_macro_attribute]
pub fn warn_span(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    span::expand_tokens(span::ExpandTokens {
        level: Some(quote!(emit::Level::Warn)),
        input: TokenStream::from(args),
        item: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

/**
Wrap an operation in an error span.

# Syntax

See the [`macro@span`] macro for syntax.
*/
#[proc_macro_attribute]
pub fn error_span(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    span::expand_tokens(span::ExpandTokens {
        level: Some(quote!(emit::Level::Error)),
        input: TokenStream::from(args),
        item: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

/**
Start a span.

See the [`SpanGuard::new`](https://docs.rs/emit/1.2.0/emit/span/struct.SpanGuard.html#method.new) for details on starting and completing the returned span.

# Syntax

```text
(control_param),* tpl, (property),*
```

where

- `control_param`: A Rust field-value with a pre-determined identifier (see below).
- `tpl`: A template string literal.
- `property`: A Rust field-value for a property to capture.

# Control parameters

This macro accepts the following optional control parameters:

| name        | type                          | description                                                                                                                                                    |
| ----------- | ----------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `rt`        | `impl emit::runtime::Runtime` | The runtime to emit the event through.                                                                                                                         |
| `mdl`       | `impl Into<emit::Path>`       | The module the event belongs to. If unspecified the current module path is used.                                                                               |
| `when`      | `impl emit::Filter`           | A filter to use instead of the one configured on the runtime.                                                                                                  |
| `panic_lvl` | `str` or `emit::Level`        | Detect whether the function panics and use the given level if it does.                                                                                         |

# Template

The template for the event. See the [`macro@tpl`] macro for syntax.

# Properties

Properties that appear within the template or after it are added to the emitted event. The identifier of the property is its key. Property capturing can be adjusted through the `as_*` attribute macros.
*/
#[proc_macro]
pub fn new_span(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    span::expand_new_tokens(span::ExpandNewTokens {
        level: None,
        input: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

/**
Start a debug span.

# Syntax

See the [`macro@new_span`] macro for syntax.
*/
#[proc_macro]
pub fn new_debug_span(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    span::expand_new_tokens(span::ExpandNewTokens {
        level: Some(quote!(emit::Level::Debug)),
        input: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

/**
Start an info span.

# Syntax

See the [`macro@new_span`] macro for syntax.
*/
#[proc_macro]
pub fn new_info_span(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    span::expand_new_tokens(span::ExpandNewTokens {
        level: Some(quote!(emit::Level::Info)),
        input: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

/**
Start a warning span.

# Syntax

See the [`macro@new_span`] macro for syntax.
*/
#[proc_macro]
pub fn new_warn_span(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    span::expand_new_tokens(span::ExpandNewTokens {
        level: Some(quote!(emit::Level::Warn)),
        input: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

/**
Start an error span.

# Syntax

See the [`macro@new_span`] macro for syntax.
*/
#[proc_macro]
pub fn new_error_span(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    span::expand_new_tokens(span::ExpandNewTokens {
        level: Some(quote!(emit::Level::Error)),
        input: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

/**
Construct a template.

Templates are text literals that include regular text with _holes_. A hole is a point in the template where a property should be interpolated in.

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
*/
#[proc_macro]
pub fn tpl(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    build::expand_tpl_tokens(build::ExpandTplTokens {
        input: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

/**
Emit an event.

# Syntax

```text
(control_param),* tpl, (property),*
```

where

- `control_param`: A Rust field-value with a pre-determined identifier (see below).
- `tpl`: A template string literal.
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

# Template

The template for the event. See the [`macro@tpl`] macro for syntax.

# Properties

Properties that appear within the template or after it are added to the emitted event. The identifier of the property is its key. Property capturing can be adjusted through the `as_*` attribute macros.
*/
#[proc_macro]
pub fn emit(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    emit::expand_tokens(emit::ExpandTokens {
        level: None,
        input: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

/**
Emit a debug event.

# Syntax

See the [`macro@emit`] macro for syntax.
*/
#[proc_macro]
pub fn debug(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    emit::expand_tokens(emit::ExpandTokens {
        level: Some(quote!(emit::Level::Debug)),
        input: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

/**
Emit an info event.

# Syntax

See the [`macro@emit`] macro for syntax.
*/
#[proc_macro]
pub fn info(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    emit::expand_tokens(emit::ExpandTokens {
        level: Some(quote!(emit::Level::Info)),
        input: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

/**
Emit a warn event.

# Syntax

See the [`macro@emit`] macro for syntax.
*/
#[proc_macro]
pub fn warn(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    emit::expand_tokens(emit::ExpandTokens {
        level: Some(quote!(emit::Level::Warn)),
        input: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

/**
Emit an error event.

# Syntax

See the [`macro@emit`] macro for syntax.
*/
#[proc_macro]
pub fn error(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    emit::expand_tokens(emit::ExpandTokens {
        level: Some(quote!(emit::Level::Error)),
        input: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

/**
Emit a temporary debug event.

# Syntax

```text
(property),*
tpl, (property),*
```

where

- `tpl`: A template string literal.
- `property`: A Rust field-value for a property to capture.

# Properties

Properties that appear within the template or after it are added to the emitted event. The identifier of the property is its key. Property capturing can be adjusted through the `as_*` attribute macros.

Unlike [`macro@debug`], this macro captures values using their [`Debug`](https://doc.rust-lang.org/std/fmt/trait.Debug.html) implementation by default.

# When to use `dbg`

This macro is a convenient way to pepper debug logs through code, but follows the same recommendations as the standard library's `dbg` macro.
You shouldn't expect `dbg` statements to be long lived, and use the [`macro@debug`] macro instead with more deliberate data.
*/
#[proc_macro]
pub fn dbg(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    dbg::expand_tokens(dbg::ExpandTokens {
        input: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

/**
Construct a path.

# Syntax

```text
path
```

where

- `path`: A string literal containing a valid `emit` path.
*/
#[proc_macro]
pub fn path(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    build::expand_path_tokens(build::ExpandPathTokens {
        input: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

/**
Construct a set of properties.

# Syntax

```text
(property),*
```

where

- `property`: A Rust field-value for a property. The identifier of the field-value is the key of the property.
*/
#[proc_macro]
pub fn props(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    build::expand_props_tokens(build::ExpandPropsTokens {
        input: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

/**
Specify Rust format flags to use when rendering a property in a template.

# Syntax

```text
fmt_string
```

where

- `fmt_string`: A string literal with the format flags, like `":?"`. See the [`std::fmt`](https://doc.rust-lang.org/std/fmt/index.html) docs for details on available flags.

# Applicable to

This attribute can be applied to properties that appear in a template.
*/
#[proc_macro_attribute]
pub fn fmt(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    (hook::get("fmt").unwrap())(TokenStream::from(args), TokenStream::from(item))
        .unwrap_or_compile_error()
}

/**
Specify the key for a property.

# Syntax

```text
key
```

where

- `key`: A string literal with the key to use. The key doesn't need to be a valid Rust identifier.

# Applicable to

This attribute can be applied to properties.
*/
#[proc_macro_attribute]
pub fn key(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    (hook::get("key").unwrap())(TokenStream::from(args), TokenStream::from(item))
        .unwrap_or_compile_error()
}

/**
Specify that a property value of `None` should not be captured, instead of being captured as `null`.

# Syntax

This macro doesn't accept any arguments.

# Applicable to

This attribute can be applied to properties where the type is `Option<&T>`.
*/
#[proc_macro_attribute]
pub fn optional(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    (hook::get("optional").unwrap())(TokenStream::from(args), TokenStream::from(item))
        .unwrap_or_compile_error()
}

/**
Capture a property using its `ToValue` implementation.

# Syntax

This macro doesn't accept any arguments.

# Applicable to

This attribute can be applied to properties.
*/
#[proc_macro_attribute]
pub fn as_value(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    (hook::get("as_value").unwrap())(TokenStream::from(args), TokenStream::from(item))
        .unwrap_or_compile_error()
}

/**
Capture a property using its `Debug` implementation.

# Syntax

This macro doesn't accept any arguments.

# Applicable to

This attribute can be applied to properties.
*/
#[proc_macro_attribute]
pub fn as_debug(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    (hook::get("as_debug").unwrap())(TokenStream::from(args), TokenStream::from(item))
        .unwrap_or_compile_error()
}

/**
Capture a property using its `Display` implementation.

# Syntax

This macro doesn't accept any arguments.

# Applicable to

This attribute can be applied to properties.
*/
#[proc_macro_attribute]
pub fn as_display(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    (hook::get("as_display").unwrap())(TokenStream::from(args), TokenStream::from(item))
        .unwrap_or_compile_error()
}

/**
Capture a property using its `sval::Value` implementation.

# Syntax

This macro doesn't accept any arguments.

# Applicable to

This attribute can be applied to properties.
*/
#[proc_macro_attribute]
pub fn as_sval(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    (hook::get("as_sval").unwrap())(TokenStream::from(args), TokenStream::from(item))
        .unwrap_or_compile_error()
}

/**
Capture a property using its `serde::Serialize` implementation.

# Syntax

This macro doesn't accept any arguments.

# Applicable to

This attribute can be applied to properties.
*/
#[proc_macro_attribute]
pub fn as_serde(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    (hook::get("as_serde").unwrap())(TokenStream::from(args), TokenStream::from(item))
        .unwrap_or_compile_error()
}

/**
Capture a property using its `Error` implementation.

# Syntax

This macro doesn't accept any arguments.

# Applicable to

This attribute can be applied to properties.
*/
#[proc_macro_attribute]
pub fn as_error(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    (hook::get("as_error").unwrap())(TokenStream::from(args), TokenStream::from(item))
        .unwrap_or_compile_error()
}

/*
fn base_emit(level: Option<TokenStream>, item: TokenStream) -> proc_macro::TokenStream {
    emit::expand_tokens(emit::ExpandTokens { level, input: item }).unwrap_or_compile_error()
}

fn base_span(
    level: Option<TokenStream>,
    input: TokenStream,
    item: TokenStream,
) -> proc_macro::TokenStream {
    if filter::matches_build_filter() {
        span::expand_tokens(span::ExpandTokens { level, input, item }).unwrap_or_compile_error()
    } else {
        item.into()
    }
}
*/

fn capture_as(
    name: &'static str,
    args: TokenStream,
    expr: TokenStream,
    as_fn: TokenStream,
    as_anon_fn: TokenStream,
) -> syn::Result<TokenStream> {
    capture::rename_hook_tokens(capture::RenameHookTokens {
        name,
        args,
        expr,
        to: |args: &capture::Args| {
            if args.inspect {
                as_fn.clone()
            } else {
                as_anon_fn.clone()
            }
        },
    })
}
