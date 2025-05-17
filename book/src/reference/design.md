# Design notes on `emit`'s macros

_May, 2025_

The human-readable message is a key part of a diagnostic event, and so is a cornerstone of a framework's API. Without it, you're just looking at a bag of data. A useful message includes enough detail from the ambient environment to help contextualize you when you see it. In Rust, you might construct such a message with the `format!()` macro from `std::fmt`:

```rust,ignore
let user = "user-123";
let product = "product-456";

let msg = format!("{user} added {product} to their cart");
```

`emit`'s macros are a departure from how existing frameworks in this space work. `emit` invents its own syntax using procedural macros instead of deferring to `std::fmt`. This document outlines why `emit`'s macro syntax was chosen and how it works, so it might serve as a data point for others who want to explore this space more in their own projects.

The original motivations for `std::fmt`'s current template syntax [are largely lost to time](https://github.com/rust-lang/rust/pull/8182), but the result is in the same ballpark as similar features in other languages, including [Python](https://peps.python.org/pep-3101/) and [C#](https://learn.microsoft.com/en-us/dotnet/csharp/tutorials/string-interpolation). The complete syntax is documented [here](https://doc.rust-lang.org/std/fmt/index.html#syntax), which we'll revisit a bit later.

Existing diagnostic frameworks use `std::fmt` for their human-readable messages, and so inherit its template syntax. In `log`, it looks like this:

```rust,ignore
let user = "user-123";
let product = "product-456";

log::info!("{user} added {product} to their cart");
```

Relying on `std::fmt` makes sense; you get the exact same syntax Rust developers are already familiar with, along with all the work to integrate it into the compiler, optimizations, supporting IDE tooling, and you didn't have to do anything to get it.

## Why not build on `std::fmt`?

The output of `format!()` is a string, like `"user-123 added product-456 to their cart"`. When using `format!()` (or `format_args!()`, which can write to other destinations besides a string) you only see this final output. You don't get to see the template `"{user} added {product} to their cart"` or its arguments `user` or `product`. As a framework building on `std::fmt` you don't know that there's a variable called `user` or `product` that the user has plumbed in to their message.

The message is a key part of a diagnostic event, but it's not enough on its own. If all you have is the string `"user-123 added product-456 to their cart"` you can't reliably work backwards to figure out who `user` or `product` were. You need properties like `user` and `product` as structured data to make your diagnostic events useful.

To get around the opaqueness of `std::fmt`, frameworks relying on it need you to duplicate any values you format into the message, so they can also capture them as structured data. In `log`, it looks like this:

```rust,ignore
let user = "user-123";
let product = "product-456";

log::info!(user, product; "{user} added {product} to their cart");
```

It's not unreasonable for `std::fmt` to be largely opaque. Its sole job is to build strings, and minimizing its public API gives it options to optimize the way it does this.

This is a major papercut in existing APIs though. Ideally, we should be able to capture values we interpolate as structured data without having duplicating them elsewhere. This is the main idea behind [message templates](https://messagetemplates.org), which is widely used in the .NET ecosystem. This isn't possible with `std::fmt`, and I would say is pretty much out-of-scope for it to do, so we have to implement something new.

## `std::fmt` syntax vs new syntax

We need to build an alternative to `std::fmt` so we can see inside the template and capture values within it as structured data. We can either do this using the same end-user syntax as `std::fmt`, or we can invent something new. `emit` opted to invent something new. To understand why, let's first explore a few relevant features of `std::fmt`'s syntax in more detail.

### `std::fmt`'s capturing syntax

`std::fmt` interpolates _values_ into its _template literal_ by formatting them in _holes_ with an _identifer_ for the value to interpolate. These holes are denoted by `{}` within the template:

```rust,ignore
format!("hello, {x}");
//              --- hole
//       ---------- template literal
```

The identifier is optional, but we're not looking at positional parameters here. If you want to interpolate a more complex expression, instead of binding a hole's identifier to some local variable, you can assign it a value that's specified after the template as a named parameter:

```rust,ignore
format!("hello, {x}", x = 42);
//                        -- value
//                    - identifier
//                    ------ named parameter
//               - identifier
//              --- hole
//       ---------- template literal
```

### `std::fmt`'s configuration syntax

Inside a hole, you can also specify _flags_ that can set formatting options for the value. Each flag has its own bespoke, compact syntax. When formatting, `std::fmt` will pass a formatter with these particular flags set to an implementation of a formatting trait on the interpolated value. Let's take an example:

```rust,ignore
format!("hello, {x:>08.3}");
//                 ----- flags
//               - identifier
//              --------- hole
//       ---------------- template literal
```

We're specifying two flags here:

1. `>08`: Left padding with the `>` sigil, followed by the padding character, `0`, and the total width to pad to, `8`.
2. `.3`: Fractional precision with the `.` sigil, followed by the number of fractional digits, `3`.

The choice of formatting trait is also configurable via sigils. Here's another example:

```rust,ignore
format!("hello, {x:?}");
//                 - type
//               - identifier
//              ----- hole
//       ------------ template literal
```

The `?` sigil uses `x`'s `std::fmt::Debug` implementation instead of its `std::fmt::Display`.

### `emit`'s capturing syntax

`emit` adopts some of `std::fmt`'s capturing syntax. It also uses identifiers to label a hole in its template literals, which can refer to a local variable:

```rust,ignore
emit::info!("hello, {x}");
//                   - identifier
//                  --- hole
//          ------------ template literal
```

In the above example, `x` isn't just interpolated into the message. It's also captured as a structured value on the diagnostic event with the key `"x"`.

A major decision point is how to represent named parameters after the template literal. There are two main options here: the named function argument syntax used by `std::fmt` with `ident = expr`, or struct field initialization syntax with `ident: expr`.

`emit` uses `ident: expr` struct field initialization syntax:

```rust,ignore
emit::info!("hello, {x}", x: 42);
//                           -- value
//                        - identifier
//                        ----- named parameter
//                   - identifier
//                  --- hole
//           ---------- template literal
```

The choice of  `ident = expr` hypothetical named function argument syntax would also have been perfectly defensible here. In the end I opted for `ident: expr` because the set of properties becomes a datastructure, like a map of `{ prop_0: value_0, prop_1: value_1, prop_n: value_n }`. I think it would also work naturally with some datastructure-like syntax extensions we might consider in the future.

Since `emit` wholesale adopts struct field initialization syntax, it means you can put arbitrarily complex expressions in template holes:

```rust,ignore
emit::info!("hello, {x: 42}");
```

This is both a benefit and a drawback. It's convenient to have a single syntax that works the same everywhere, but it means an `emit` template literal is not fully parseable without full Rust syntax support. That could become an issue for fully interpreted templates in the future.

### `emit`'s configuration syntax

`std::fmt`'s flags are largely irrelevant for capturing structured data the way `emit` does. If you're capturing an `f64`, the flag to pad it to 8 characters is meaningless if you're eventually serializing it into a protobuf message as a native binary floating point. You do still need some syntax to configure capturing though. Rust doesn't have reflection, so you can't tell just by looking at a value what the best trait to capture it with is.

Standard Rust supports meta configuration through attributes, like `#[cfg]`. Since `emit` uses struct field initialization syntax, it already technically supports attributes on them. For example, in `std::fmt` syntax you can format a value using its `Debug` implementation using the `?` sigil:

```rust,ignore
format!("{x:?}");
```

In `emit` using attributes, you can do this:

```rust,ignore
emit::info!("{#[emit::as_debug] x}");
```

or this:

```rust,ignore
emit::info!("{x}", #[emit::as_debug] x);
```

Rust attributes can accept arguments of their own. `emit` opts to re-use struct field initialization syntax here:

```rust,ignore
emit::info!("{x}", #[emit::as_debug(inspect: true)] x);
```

The drawback of attributes is that they're less compact than sigil-based flags.

It's worth noting here that `#[emit::as_debug]` is not magically understood by `emit::info!`. It's just a regular procedural macro that operates on the output of `emit::info!`. `emit`'s attribute macros are based on _hooks_. These are named function calls emitted by previously evaluated macros that the attribute looks for and replaces. As an exmaple, `emit::info!` converts this:

```rust,ignore
emit::info!("hello", x);
```

into something like this:

```rust,ignore
match ({
    ("x", x.__private_capture_as_default())
}) {
    p0 => {
        // ..
    }
}
```

The `#[emit::as_debug]` attribute hooks into `__private_capture_as` calls it finds in the annotated expression and replaces them with a new one:

```rust,ignore
match ({
    ("x", x.__private_capture_as_debug())
}) {
    p0 => {
        // ..
    }
}
```

This is a surprisingly simple and powerful pattern for composing macros. Using standard Rust attributes means `emit`'s capturing is technically user-extensible. There's nothing that `#[emit::as_debug]` does that an end-user's own attribute macros couldn't.

### Runtime Interpolation API

`emit` uses lazy interpolation rather than eager. To render a template, you give it a set of `emit::Props` and an implementation of `emit::template::Write`, which is fed the text and property holes in sequence. This ends up working a lot like [JavaScript's tagged templates](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Template_literals#tagged_templates), and makes it possible to do things like `emit_term`'s type-based coloring:

![`emit_term` colored output](../asset/emit_term.png)

## Possible future directions

We may want to introduce some kind of spread syntax for merging sets of properties:

```rust,ignore
emit::info!(
    "hello, {x}",
    x: 42,
    ...props,
    y: 13,
);
```

If Rust does start seriously considering some kind of named function parameter syntax using `ident = expr`, we may want to start accepting that syntax in attributes and control parameters (meta configuration values that appear before the template literal and aren't captured as properties):

```rust,ignore
emit::info!(
    mdl = "a::b",
    "hello, {x}",
    #[emit::as_debug(inspect = true)]
    x: 42,
);
```

The syntactic difference here may also make it clearer what's captured as a property, and what's meta configuration.
