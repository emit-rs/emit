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
mod sample;
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

#[doc = "Format a template."]
#[doc = ""]
#[doc = include_str!("./doc_fmt.md")]
#[proc_macro]
pub fn format(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    format::expand_tokens(format::ExpandTokens {
        input: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

#[doc = "Construct an event that can be emitted manually."]
#[doc = ""]
#[doc = include_str!("./doc_evt.md")]
#[proc_macro]
pub fn evt(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    build::expand_evt_tokens(build::ExpandEvtTokens {
        level: None,
        input: item.into(),
    })
    .unwrap_or_compile_error()
}

#[doc = "Construct an event at the debug level that can be emitted manually."]
#[doc = ""]
#[doc = include_str!("./doc_evt.md")]
#[proc_macro]
pub fn debug_evt(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    build::expand_evt_tokens(build::ExpandEvtTokens {
        level: Some(quote!(emit::Level::Debug)),
        input: item.into(),
    })
    .unwrap_or_compile_error()
}

#[doc = "Construct an event at the info level that can be emitted manually."]
#[doc = ""]
#[doc = include_str!("./doc_evt.md")]
#[proc_macro]
pub fn info_evt(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    build::expand_evt_tokens(build::ExpandEvtTokens {
        level: Some(quote!(emit::Level::Info)),
        input: item.into(),
    })
    .unwrap_or_compile_error()
}

#[doc = "Construct an event at the warn level that can be emitted manually."]
#[doc = ""]
#[doc = include_str!("./doc_evt.md")]
#[proc_macro]
pub fn warn_evt(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    build::expand_evt_tokens(build::ExpandEvtTokens {
        level: Some(quote!(emit::Level::Warn)),
        input: item.into(),
    })
    .unwrap_or_compile_error()
}

#[doc = "Construct an event at the error level that can be emitted manually."]
#[doc = ""]
#[doc = include_str!("./doc_evt.md")]
#[proc_macro]
pub fn error_evt(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    build::expand_evt_tokens(build::ExpandEvtTokens {
        level: Some(quote!(emit::Level::Error)),
        input: item.into(),
    })
    .unwrap_or_compile_error()
}

#[doc = "Trace the execution of a function."]
#[doc = ""]
#[doc = include_str!("./doc_span.md")]
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

#[doc = "Trace the execution of a function at the debug level."]
#[doc = ""]
#[doc = include_str!("./doc_span.md")]
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

#[doc = "Trace the execution of a function at the info level."]
#[doc = ""]
#[doc = include_str!("./doc_span.md")]
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

#[doc = "Trace the execution of a function at the warn level."]
#[doc = ""]
#[doc = include_str!("./doc_span.md")]
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

#[doc = "Trace the execution of a function at the error level."]
#[doc = ""]
#[doc = include_str!("./doc_span.md")]
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

#[doc = "Create a span that can be started and completed manually."]
#[doc = ""]
#[doc = include_str!("./doc_new_span.md")]
#[proc_macro]
pub fn new_span(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    span::expand_new_tokens(span::ExpandNewTokens {
        level: None,
        input: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

#[doc = "Create a span at the debug level that can be started and completed manually."]
#[doc = ""]
#[doc = include_str!("./doc_new_span.md")]
#[proc_macro]
pub fn new_debug_span(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    span::expand_new_tokens(span::ExpandNewTokens {
        level: Some(quote!(emit::Level::Debug)),
        input: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

#[doc = "Create a span at the info level that can be started and completed manually."]
#[doc = ""]
#[doc = include_str!("./doc_new_span.md")]
#[proc_macro]
pub fn new_info_span(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    span::expand_new_tokens(span::ExpandNewTokens {
        level: Some(quote!(emit::Level::Info)),
        input: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

#[doc = "Create a span at the warn level that can be started and completed manually."]
#[doc = ""]
#[doc = include_str!("./doc_new_span.md")]
#[proc_macro]
pub fn new_warn_span(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    span::expand_new_tokens(span::ExpandNewTokens {
        level: Some(quote!(emit::Level::Warn)),
        input: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

#[doc = "Create a span at the error level that can be started and completed manually."]
#[doc = ""]
#[doc = include_str!("./doc_new_span.md")]
#[proc_macro]
pub fn new_error_span(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    span::expand_new_tokens(span::ExpandNewTokens {
        level: Some(quote!(emit::Level::Error)),
        input: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

#[doc = "Construct a template."]
#[doc = ""]
#[doc = include_str!("./doc_tpl.md")]
#[proc_macro]
pub fn tpl(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    build::expand_tpl_tokens(build::ExpandTplTokens {
        input: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

#[doc = "Emit an event."]
#[doc = ""]
#[doc = include_str!("./doc_emit.md")]
#[proc_macro]
pub fn emit(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    emit::expand_tokens(emit::ExpandTokens {
        level: None,
        input: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

#[doc = "Emit an event at the debug level."]
#[doc = ""]
#[doc = include_str!("./doc_emit.md")]
#[proc_macro]
pub fn debug(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    emit::expand_tokens(emit::ExpandTokens {
        level: Some(quote!(emit::Level::Debug)),
        input: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

#[doc = "Emit an event at the info level."]
#[doc = ""]
#[doc = include_str!("./doc_emit.md")]
#[proc_macro]
pub fn info(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    emit::expand_tokens(emit::ExpandTokens {
        level: Some(quote!(emit::Level::Info)),
        input: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

#[doc = "Emit an event at the warn level."]
#[doc = ""]
#[doc = include_str!("./doc_emit.md")]
#[proc_macro]
pub fn warn(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    emit::expand_tokens(emit::ExpandTokens {
        level: Some(quote!(emit::Level::Warn)),
        input: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

#[doc = "Emit an event at the error level."]
#[doc = ""]
#[doc = include_str!("./doc_emit.md")]
#[proc_macro]
pub fn error(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    emit::expand_tokens(emit::ExpandTokens {
        level: Some(quote!(emit::Level::Error)),
        input: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

#[doc = "Emit a temporary event as a quick-and-dirty debugging aid."]
#[doc = ""]
#[doc = include_str!("./doc_dbg.md")]
#[proc_macro]
pub fn dbg(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    dbg::expand_tokens(dbg::ExpandTokens {
        input: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

/**
Emit a metric sample.

# Examples

Emit a metric sample from a value:

```ignore
let my_metric = 42;

emit::sample!(value: my_metric);
```

In the above example, the `name` is inferred to be `"my_metric"` using the name of the identifier in the `value` control parameter.

The `name` can also be specified manually, and is required if `value` is not an identifier:

```ignore
emit::sample!(name: "my_metric", value: 42);
```

Properties can be attached to metric samples:

```ignore
let my_metric = 42;

let metric = emit::sample!(value: my_metric, props: emit::props! { my_property: "some value" });
```
*/
#[doc = ""]
#[doc = include_str!("./doc_sample.md")]
#[proc_macro]
pub fn sample(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    sample::expand_tokens(sample::ExpandTokens {
        agg: None,
        input: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

/**
Emit a metric sample with `count` as its aggregation.

# Examples

See [`macro@sample`].
*/
#[doc = ""]
#[doc = include_str!("./doc_sample.md")]
#[proc_macro]
pub fn count_sample(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    sample::expand_tokens(sample::ExpandTokens {
        agg: Some({
            let agg = emit_core::well_known::METRIC_AGG_COUNT;

            quote!(#agg)
        }),
        input: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

/**
Emit a metric sample with `sum` as its aggregation.

# Examples

See [`macro@sample`].
*/
#[doc = ""]
#[doc = include_str!("./doc_sample.md")]
#[proc_macro]
pub fn sum_sample(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    sample::expand_tokens(sample::ExpandTokens {
        agg: Some({
            let agg = emit_core::well_known::METRIC_AGG_SUM;

            quote!(#agg)
        }),
        input: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

/**
Emit a metric sample with `min` as its aggregation.

# Examples

See [`macro@sample`].
*/
#[doc = ""]
#[doc = include_str!("./doc_sample.md")]
#[proc_macro]
pub fn min_sample(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    sample::expand_tokens(sample::ExpandTokens {
        agg: Some({
            let agg = emit_core::well_known::METRIC_AGG_MIN;

            quote!(#agg)
        }),
        input: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

/**
Emit a metric sample with `max` as its aggregation.

# Examples

See [`macro@sample`].
*/
#[doc = ""]
#[doc = include_str!("./doc_sample.md")]
#[proc_macro]
pub fn max_sample(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    sample::expand_tokens(sample::ExpandTokens {
        agg: Some({
            let agg = emit_core::well_known::METRIC_AGG_MAX;

            quote!(#agg)
        }),
        input: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

/**
Emit a metric sample with `last` as its aggregation.

# Examples

See [`macro@sample`].
*/
#[doc = ""]
#[doc = include_str!("./doc_sample.md")]
#[proc_macro]
pub fn last_sample(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    sample::expand_tokens(sample::ExpandTokens {
        agg: Some({
            let agg = emit_core::well_known::METRIC_AGG_LAST;

            quote!(#agg)
        }),
        input: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

/**
Construct a metric sample.

# Examples

Construct a metric sample from a value:

```ignore
let my_metric = 42;

let metric = emit::metric!(value: my_metric);
```

In the above example, the `name` is inferred to be `"my_metric"` using the name of the identifier in the `value` control parameter.

The `name` can also be specified manually, and is required if `value` is not an identifier:

```ignore
let metric = emit::metric!(name: "my_metric", value: 42);
```

Properties can be attached to metric samples:

```ignore
let my_metric = 42;

let metric = emit::metric!(value: my_metric, props: emit::props! { my_property: "some value" });
```
*/
#[doc = ""]
#[doc = include_str!("./doc_metric.md")]
#[proc_macro]
pub fn metric(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    sample::expand_metric_tokens(sample::ExpandTokens {
        agg: None,
        input: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

/**
Construct a metric sample with `count` as its aggregation.

# Examples

See [`macro@metric`].
*/
#[doc = ""]
#[doc = include_str!("./doc_metric.md")]
#[proc_macro]
pub fn count_metric(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    sample::expand_metric_tokens(sample::ExpandTokens {
        agg: Some({
            let agg = emit_core::well_known::METRIC_AGG_COUNT;

            quote!(#agg)
        }),
        input: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

/**
Construct a metric sample with `sum` as its aggregation.

# Examples

See [`macro@metric`].
*/
#[doc = ""]
#[doc = include_str!("./doc_metric.md")]
#[proc_macro]
pub fn sum_metric(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    sample::expand_metric_tokens(sample::ExpandTokens {
        agg: Some({
            let agg = emit_core::well_known::METRIC_AGG_SUM;

            quote!(#agg)
        }),
        input: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

/**
Construct a metric sample with `min` as its aggregation.

# Examples

See [`macro@metric`].
*/
#[doc = ""]
#[doc = include_str!("./doc_metric.md")]
#[proc_macro]
pub fn min_metric(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    sample::expand_metric_tokens(sample::ExpandTokens {
        agg: Some({
            let agg = emit_core::well_known::METRIC_AGG_MIN;

            quote!(#agg)
        }),
        input: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

/**
Construct a metric sample with `max` as its aggregation.

# Examples

See [`macro@metric`].
*/
#[doc = ""]
#[doc = include_str!("./doc_metric.md")]
#[proc_macro]
pub fn max_metric(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    sample::expand_metric_tokens(sample::ExpandTokens {
        agg: Some({
            let agg = emit_core::well_known::METRIC_AGG_MAX;

            quote!(#agg)
        }),
        input: TokenStream::from(item),
    })
    .unwrap_or_compile_error()
}

/**
Construct a metric sample with `last` as its aggregation.

# Examples

See [`macro@metric`].
*/
#[doc = ""]
#[doc = include_str!("./doc_metric.md")]
#[proc_macro]
pub fn last_metric(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    sample::expand_metric_tokens(sample::ExpandTokens {
        agg: Some({
            let agg = emit_core::well_known::METRIC_AGG_LAST;

            quote!(#agg)
        }),
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

# Returns

An `emit::Path`.
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

# Returns

An `impl emit::Props`.
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

This macro can also be called with an explicit `name` identifier:

```text
name: key
```

where

- `key`: An expression that evaluates to a string value for the key to use. The key doesn't need to be a valid Rust identifier.

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
