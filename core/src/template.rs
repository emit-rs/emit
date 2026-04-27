/*!
The [`Template`] type.

Templates are the primary way of describing [`crate::event::Event`]s. A template is a block of text with named holes for [`Value`]s to be interpolated into. When the template is fed a set of [`Props`] it can be rendered into text using an instance of [`Write`]. Here's an example of a template:

```text
Hello, {user}.
```

If this template is fed the property `user: "Rust"`, it can be rendered into text:

```text
Hello, Rust.
```

`Template`s are conceptually similar to the standard library's `Arguments` type. The key difference between them is that templates are a runtime construct rather than a compile time one. You can construct a template programmatically, inspect its holes, and choose to render it in any way you like. The standard library's formatting APIs are optimized for producing strings. Templates are both a property capturing and a formatting tool.
*/

use core::{
    cmp, fmt,
    hash::{Hash, Hasher},
    slice,
};

#[cfg(feature = "alloc")]
use alloc::boxed::Box;

use crate::{
    empty::Empty,
    props::Props,
    str::Str,
    value::{ToValue, Value},
};

/**
A lazily evaluated text template with named holes for interpolating properties into.

The [`Template::render`] method can be used to format a template with [`Props`] into a string or other representation.

Template equality is based on the equality of their renderings.

Two templates can be equal if they'd render to the same outputs. That means they must have the same holes in the same positions, but their text tokens may be split differently, so long as they produce the same text.
*/
#[derive(Clone)]
pub struct Template<'a> {
    kind: TemplateKind<'a>,
}

#[derive(Clone, Debug)]
enum TemplateKind<'a> {
    Literal([Part<'a>; 1]),
    Parts(&'a [Part<'a>]),
    #[cfg(feature = "alloc")]
    Owned(Box<[Part<'static>]>),
}

impl<'a> TemplateKind<'a> {
    fn parts(&self) -> &[Part<'a>] {
        match self {
            TemplateKind::Literal(ref parts) => parts,
            TemplateKind::Parts(parts) => parts,
            #[cfg(feature = "alloc")]
            TemplateKind::Owned(parts) => parts,
        }
    }
}

impl<'a> fmt::Debug for Template<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.render(Empty).with_escaping(true), f)
    }
}

impl<'a> fmt::Display for Template<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.render(Empty).with_escaping(true), f)
    }
}

impl<'a> From<&'a [Part<'a>]> for Template<'a> {
    fn from(value: &'a [Part<'a>]) -> Self {
        Template::new_ref(value)
    }
}

impl Template<'static> {
    /**
    Create a template from a set of tokens.
    */
    pub const fn new(parts: &'static [Part<'static>]) -> Self {
        Template {
            kind: TemplateKind::Parts(parts),
        }
    }

    /**
    Create a template from a string literal with no holes.
    */
    pub const fn literal(text: &'static str) -> Self {
        Template {
            kind: TemplateKind::Literal([Part::text(text)]),
        }
    }
}

impl<'a> Template<'a> {
    /**
    Create a template from a borrowed set of tokens.

    The [`Template::new`] method should be preferred where possible.
    */
    pub const fn new_ref(parts: &'a [Part<'a>]) -> Self {
        Template {
            kind: TemplateKind::Parts(parts),
        }
    }

    /**
    Create a template from a string literal with no holes.

    The [`Template::literal`] method should be preferred where possible.
    */
    pub const fn literal_ref(text: &'a str) -> Self {
        Template {
            kind: TemplateKind::Literal([Part::text_ref(text)]),
        }
    }

    /**
    Get a new template, borrowing data from this one.
    */
    pub fn by_ref<'b>(&'b self) -> Template<'b> {
        match self.kind {
            TemplateKind::Literal([ref part]) => Template {
                kind: TemplateKind::Literal([part.by_ref()]),
            },
            TemplateKind::Parts(parts) => Template {
                kind: TemplateKind::Parts(parts),
            },
            #[cfg(feature = "alloc")]
            TemplateKind::Owned(ref parts) => Template {
                kind: TemplateKind::Parts(parts),
            },
        }
    }

    /**
    Try get the value of the template as a string literal.

    If the template only has a single token, and that token is text, then this method will return `Some`. Otherwise this method will return `None`.
    */
    pub fn as_literal(&'_ self) -> Option<&'_ Str<'a>> {
        match self.kind.parts() {
            [part] => part.as_text(),
            _ => None,
        }
    }

    /**
    Iterate over the parts of the template.
    */
    pub fn parts(&'_ self) -> Parts<'_, 'a> {
        Parts(self.kind.parts().iter())
    }

    /**
    Lazily render the template, using the given properties for interpolation.
    */
    pub fn render<'b, P>(&'b self, props: P) -> Render<'b, P> {
        Render {
            tpl: self.by_ref(),
            escape: false,
            props,
        }
    }
}

/**
The result of calling [`Template::parts`].
*/
pub struct Parts<'a, 'b>(slice::Iter<'a, Part<'b>>);

impl<'a, 'b> Iterator for Parts<'a, 'b> {
    type Item = &'a Part<'b>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl<'a> ToValue for Template<'a> {
    fn to_value(&self) -> Value<'_> {
        if let Some(tpl) = self.as_literal() {
            Value::from_any(tpl)
        } else {
            Value::from_display(self)
        }
    }
}

impl<'a, 'b> PartialEq<Template<'b>> for Template<'a> {
    /**
    Compare two templates for equality.

    Templates are considered equal if they'd produce the same value when
    formatted by `Display`. Note that the presence of formatters on holes
    does not affect equality.
    */
    fn eq(&self, other: &Template<'b>) -> bool {
        // Optimize for the case where both templates are just text literals
        if let (Some(a), Some(b)) = (self.as_literal(), other.as_literal()) {
            return a == b;
        }

        // Index into parts of `a` and `b`
        let mut ai = 0;
        let mut bi = 0;

        // Index into the current text fragment of `a` and `b`
        let mut ati = 0;
        let mut bti = 0;

        let a = self.kind.parts();
        let b = other.kind.parts();

        while ai < a.len() && bi < b.len() {
            let ap = &a[ai];
            let bp = &b[bi];

            match (&ap.0, &bp.0) {
                // Compare text fragments
                (
                    PartKind::Text {
                        value: ref a,
                        needs_escaping: _,
                    },
                    PartKind::Text {
                        value: ref b,
                        needs_escaping: _,
                    },
                ) => {
                    // Scan through the text fragments in `a` and `b`
                    //
                    // So long as the concatenated results are equal we consider
                    // `a` and `b` to be equal. Usually, you'd expect two equal
                    // templates to have the same exact text fragments, so this
                    // will just compare them in their entirety in that case

                    let a = a.get();
                    let b = b.get();

                    let at = &a[ati..];
                    let bt = &b[bti..];

                    let len = cmp::min(at.len(), bt.len());

                    let at = &at[..len];
                    let bt = &bt[..len];

                    if at != bt {
                        return false;
                    }

                    ati += len;
                    bti += len;

                    if ati == a.len() {
                        ai += 1;
                        ati = 0;
                    }

                    if bti == b.len() {
                        bi += 1;
                        bti = 0;
                    }

                    continue;
                }
                // Compare hole fragments
                (PartKind::Hole { label: ref a, .. }, PartKind::Hole { label: ref b, .. }) => {
                    // Holes are not partial, so must be exactly equal
                    if a != b {
                        return false;
                    }

                    ai += 1;
                    bi += 1;

                    continue;
                }
                // Ignore empty fragments
                (PartKind::Text { value: ref a, .. }, PartKind::Hole { .. })
                    if a.get().is_empty() =>
                {
                    ai += 1;

                    continue;
                }
                (PartKind::Hole { .. }, PartKind::Text { value: ref b, .. })
                    if b.get().is_empty() =>
                {
                    bi += 1;

                    continue;
                }
                // Any other mismatch means the templates aren't equal
                _ => return false,
            }
        }

        // Any trailing parts after `a` or `b` is finished must be empty fragments
        for part in a[ai..].iter().chain(b[bi..].iter()) {
            let PartKind::Text {
                ref value,
                needs_escaping: _,
            } = part.0
            else {
                return false;
            };

            if !value.get().is_empty() {
                return false;
            }
        }

        // If we get this far then the templates are equal
        true
    }
}

impl<'a> Eq for Template<'a> {}

impl<'a> Hash for Template<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        /*
        The hash format used here emulates the same equality behavior as `partial_eq`.

        Two templates hash to the same value if they format the same when escaping is applied.
        That means hashing doesn't depend on the composition of text fragments if they have the
        same concatenated value.
        */

        #[derive(Clone, Copy, PartialEq, Eq, Hash)]
        enum LastType {
            Text,
            Hole,
        }

        fn hash_bytes(
            state: &mut impl Hasher,
            bytes_written_slot: &mut usize,
            last_type_slot: &mut LastType,
            bytes: &[u8],
            last_type: LastType,
        ) {
            state.write(bytes);
            *bytes_written_slot += bytes.len();
            *last_type_slot = last_type;
        }

        fn hash_trailer(
            state: &mut impl Hasher,
            bytes_written_slot: &mut usize,
            last_type: LastType,
        ) {
            last_type.hash(state);
            bytes_written_slot.hash(state);
            *bytes_written_slot = 0;
        }

        let mut last_type = LastType::Text;
        let mut bytes_written = 0;

        for part in self.kind.parts() {
            match &part.0 {
                // Text hashing defers writing the trailer until we hit the end of the template,
                // or we hit a hole fragment
                PartKind::Text {
                    value,
                    needs_escaping: _,
                } => {
                    let bytes = value.get().as_bytes();

                    // Ignore empty fragments
                    if bytes.len() > 0 {
                        hash_bytes(
                            state,
                            &mut bytes_written,
                            &mut last_type,
                            bytes,
                            LastType::Text,
                        );
                    }
                }
                // Hole fragments always write a trailer
                PartKind::Hole {
                    label,
                    formatter: _,
                } => {
                    // NOTE: The redundant trailer here when templates start with a hole is fine
                    if last_type == LastType::Text {
                        hash_trailer(state, &mut bytes_written, last_type);
                    }

                    hash_bytes(
                        state,
                        &mut bytes_written,
                        &mut last_type,
                        label.get().as_bytes(),
                        LastType::Hole,
                    );
                    hash_trailer(state, &mut bytes_written, last_type);
                }
            }
        }

        if last_type == LastType::Text {
            hash_trailer(state, &mut bytes_written, last_type);
        }
    }
}

/**
The result of calling [`Template::render`].

The template can be converted to text either using the [`fmt::Display`] implementation of `Render`, or by calling [`Render::write`] with an instance of [`Write`].
*/
pub struct Render<'a, P> {
    tpl: Template<'a>,
    escape: bool,
    props: P,
}

impl<'a, P> Render<'a, P> {
    /**
    Set the properties to interpolate.
    */
    pub fn with_props<U>(self, props: U) -> Render<'a, U> {
        Render {
            tpl: self.tpl,
            escape: self.escape,
            props,
        }
    }

    /**
    Whether to escape `{` and `}` in text fragments as `{{` and `}}`.

    Rendering will not escape by default.
    */
    pub fn with_escaping(mut self, escape: bool) -> Self {
        self.escape = escape;
        self
    }

    /**
    Try get the value of the template as a string literal.
    */
    pub fn as_literal(&'_ self) -> Option<&'_ Str<'a>> {
        self.tpl.as_literal()
    }
}

impl<'a, P: Props> Render<'a, P> {
    /**
    Format the template into the given writer, interpolating its properties.

    The [`Write`] is fed the tokens of the template along with properties matching the labels of its holes.
    */
    pub fn write(&self, mut writer: impl Write) -> fmt::Result {
        for part in self.tpl.kind.parts() {
            part.write(self.escape, &mut writer, &self.props)?;
        }

        Ok(())
    }
}

impl<'a, P: Props> ToValue for Render<'a, P> {
    fn to_value(&self) -> Value<'_> {
        if let Some(tpl) = self.as_literal() {
            Value::from_any(tpl)
        } else {
            Value::from_display(self)
        }
    }
}

/**
A template-aware writer used by [`Render::write`] to format a template.
*/
pub trait Write: fmt::Write {
    /**
    Write a text fragment.

    This method is called for any [`Part::text`] in the template.
    */
    fn write_text(&mut self, text: &str) -> fmt::Result {
        self.write_str(text)
    }

    /**
    Write a hole with a matching value.

    This method is called for any [`Part::hole`] in the template without special formatting requirements where a matching property exists.
    */
    fn write_hole_value(&mut self, label: &str, value: Value) -> fmt::Result {
        let _ = label;
        self.write_fmt(format_args!("{}", value))
    }

    /**
    Write a hole with a matching value and formatter.

    This method is called for any [`Part::hole`] in the template with special formatting requirements where a matching property exists.
    */
    fn write_hole_fmt(&mut self, label: &str, value: Value, formatter: Formatter) -> fmt::Result {
        let _ = label;
        self.write_fmt(format_args!("{}", formatter.apply(value)))
    }

    /**
    Write a hole without a matching value.

    This method is called for any [`Part::hole`] in the template where a matching property doesn't exist.
    */
    fn write_hole_label(&mut self, label: &str) -> fmt::Result {
        self.write_fmt(format_args!("{{{}}}", label))
    }
}

impl<'a, W: Write + ?Sized> Write for &'a mut W {
    fn write_text(&mut self, text: &str) -> fmt::Result {
        (**self).write_text(text)
    }

    fn write_hole_value(&mut self, label: &str, value: Value) -> fmt::Result {
        (**self).write_hole_value(label, value)
    }

    fn write_hole_fmt(&mut self, label: &str, value: Value, formatter: Formatter) -> fmt::Result {
        (**self).write_hole_fmt(label, value, formatter)
    }

    fn write_hole_label(&mut self, label: &str) -> fmt::Result {
        (**self).write_hole_label(label)
    }
}

#[cfg(feature = "alloc")]
impl Write for alloc::string::String {}

impl<'a> Write for fmt::Formatter<'a> {
    fn write_hole_value(&mut self, _: &str, value: Value) -> fmt::Result {
        fmt::Display::fmt(&value, self)
    }

    fn write_hole_fmt(&mut self, _: &str, value: Value, formatter: Formatter) -> fmt::Result {
        formatter.fmt(value, self)
    }
}

impl<'a, P: Props> fmt::Display for Render<'a, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.write(f)
    }
}

impl<'a, P: Props> fmt::Debug for Render<'a, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use fmt::Write as _;

        struct EscapeDebug<W>(W);

        impl<W: fmt::Write> fmt::Write for EscapeDebug<W> {
            fn write_str(&mut self, s: &str) -> fmt::Result {
                for c in s.escape_debug() {
                    self.0.write_char(c)?;
                }

                Ok(())
            }
        }

        f.write_char('"')?;
        write!(EscapeDebug(&mut *f), "{}", self)?;
        f.write_char('"')
    }
}

/**
An individual token in a [`Template`].
*/
#[derive(Debug, Clone)]
pub struct Part<'a>(PartKind<'a>);

impl<'a> fmt::Display for Part<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.write(true, f, Empty)
    }
}

impl Part<'static> {
    /**
    Create a token for a fragment of literal text.
    */
    pub const fn text(text: &'static str) -> Self {
        Self::text_str(Str::new(text))
    }

    /**
    Create a token for a hole to interpolate a [`Value`] into.
    */
    pub const fn hole(label: &'static str) -> Self {
        Self::hole_str(Str::new(label))
    }
}

impl<'a> Part<'a> {
    /**
    Create a token for a borrowed fragment of literal text.

    The [`Part::text`] method should be preferred where possible.
    */
    pub const fn text_ref(text: &'a str) -> Self {
        Self::text_str(Str::new_ref(text))
    }

    /**
    Create a token for a hole with a borrowed label to interpolate a [`Value`] into.

    The [`Part::hole`] method should be preferred where possible.
    */
    pub const fn hole_ref(label: &'a str) -> Self {
        Self::hole_str(Str::new_ref(label))
    }

    /**
    Create a token for a fragment of literal text from a [`Str`] instead of `&str`.

    This method allows creating parts from potentially owned or borrowed string values.
    */
    pub const fn text_str(text: Str<'a>) -> Self {
        Part(PartKind::Text {
            // Assume the input needs to be escaped when formatting
            needs_escaping: true,
            value: text,
        })
    }

    /**
    Create a token for a hole with a label from a [`Str`] instead of `&str` to interpolate a [`Value`] into.

    This method allows creating parts from potentially owned or borrowed string values.
    */
    pub const fn hole_str(label: Str<'a>) -> Self {
        Part(PartKind::Hole {
            label,
            formatter: None,
        })
    }

    /**
    Try get the value of the part as a literal text fragment.
    */
    pub const fn as_text(&'_ self) -> Option<&'_ Str<'a>> {
        match self.0 {
            PartKind::Text {
                ref value,
                needs_escaping: _,
            } => Some(value),
            _ => None,
        }
    }

    /**
    Try get the label of the part.

    If the part is a [`Part::hole`] this method will return `Some`. Otherwise it will return `None.
    */
    pub const fn label(&'_ self) -> Option<&'_ Str<'a>> {
        match self.0 {
            PartKind::Hole {
                ref label,
                formatter: _,
            } => Some(label),
            _ => None,
        }
    }

    /**
    Try get the formatter of the part.

    If the part is a [`Part::hole`] and a formatter has been set through [`Part::with_formatter`] then this method will return `Some`. Otherwise it will return `None.
    */
    pub const fn formatter(&self) -> Option<&Formatter> {
        match self.0 {
            PartKind::Hole { ref formatter, .. } => formatter.as_ref(),
            _ => None,
        }
    }

    /**
    Get a new part, borrowing data from this one.
    */
    pub fn by_ref<'b>(&'b self) -> Part<'b> {
        match self.0 {
            PartKind::Text {
                ref value,
                needs_escaping,
            } => Part(PartKind::Text {
                value: value.by_ref(),
                needs_escaping,
            }),
            PartKind::Hole {
                ref label,
                ref formatter,
            } => Part(PartKind::Hole {
                label: label.by_ref(),
                formatter: formatter.clone(),
            }),
        }
    }

    /**
    Set a formatter for a value interpolated into this part to use.

    This method only applies to [`Part::hole`]s. It's a no-op in other cases.
    */
    pub const fn with_formatter(mut self, formatter: Formatter) -> Self {
        if let PartKind::Hole {
            formatter: ref mut slot,
            ..
        } = self.0
        {
            *slot = Some(formatter);
        }

        self
    }

    /**
    Mark whether the part should check for, and escape any `{` or `}` characters when formatting, without checking if escaping is actually necessary.

    This method only applies to [`Part::text`]s. It's a no-op in other cases.

    It is only valid to call this method on a text part with `false` if the text part does not contain any `{` or `}` characters.

    This method is not unsafe. There are no memory safety properties tied to the validity of templates. Code that uses parts may panic or produce unexpected results if given an invalid template.
    */
    pub const fn with_needs_escaping_raw(mut self, needs_escaping: bool) -> Self {
        if let PartKind::Text {
            needs_escaping: ref mut slot,
            ..
        } = self.0
        {
            *slot = needs_escaping;
        }

        self
    }

    /**
    Format the part into the given `writer`, filling any holes with values from `props`.

    If `escape` is `true` then any text parts with `{` or `}` characters will be escaped as `{{` and `}}`.
    */
    fn write(&self, escape: bool, mut writer: impl Write, props: impl Props) -> fmt::Result {
        match self.0 {
            PartKind::Text {
                ref value,
                needs_escaping,
            } => {
                if escape && needs_escaping {
                    escape_text(writer, value.get())
                } else {
                    writer.write_text(value.get())
                }
            }
            PartKind::Hole {
                ref label,
                ref formatter,
                ..
            } => {
                let label = label.get();

                if let Some(value) = props.get(label) {
                    if let Some(formatter) = formatter {
                        writer.write_hole_fmt(label, value, formatter.clone())
                    } else {
                        writer.write_hole_value(label, value)
                    }
                } else {
                    writer.write_hole_label(label)
                }
            }
        }
    }
}

/**
Write `text` to `writer`, escaping any `{` or `}` characters.
*/
fn escape_text(mut writer: impl Write, text: &str) -> fmt::Result {
    let mut from = 0;
    let mut to = 0;

    let raw = text.as_bytes();

    while to < text.len() {
        let b = raw[to];

        if let b'{' | b'}' = b {
            writer.write_text(&text[from..to])?;
            writer.write_text(&text[to..to + 1])?;
            from = to;
        }

        to += 1;
    }

    writer.write_text(&text[from..])?;

    Ok(())
}

// Work-around for const-fn in traits
// Mirrors trait fns in `macro_hooks`
#[doc(hidden)]
impl Part<'static> {
    pub const fn __private_interpolated(self) -> Self {
        self
    }

    pub const fn __private_uninterpolated(self) -> Self {
        self
    }

    pub const fn __private_captured(self) -> Self {
        self
    }

    pub const fn __private_uncaptured(self) -> Self {
        self
    }

    pub const fn __private_fmt_as_default(self) -> Self {
        self
    }

    pub const fn __private_fmt_as(self, formatter: Formatter) -> Self {
        self.with_formatter(formatter)
    }
}

/**
A specialized formatter for a [`Value`] interpolated into a [`Part::hole`].

This type supports formatting values using standard Rust flags like padding and precision.
*/
#[derive(Clone)]
pub struct Formatter {
    fmt: fn(Value, &mut fmt::Formatter) -> fmt::Result,
}

impl fmt::Debug for Formatter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Formatter").finish_non_exhaustive()
    }
}

impl Formatter {
    /**
    Create a formatter from the given function.

    It's the responsibility of the function to actually write the value into the formatter.
    */
    pub const fn new(fmt: fn(Value, &mut fmt::Formatter) -> fmt::Result) -> Self {
        Formatter { fmt }
    }

    /**
    Invoke the formatter on a given value.
    */
    pub fn fmt(&self, value: Value, f: &mut fmt::Formatter) -> fmt::Result {
        (self.fmt)(value, f)
    }

    /**
    Get a lazily formatted value that will apply the formatter.
    */
    pub fn apply<'b>(&'b self, value: Value<'b>) -> impl fmt::Display + 'b {
        struct FormatValue<'a> {
            value: Value<'a>,
            fmt: fn(Value, &mut fmt::Formatter) -> fmt::Result,
        }

        impl<'a> fmt::Display for FormatValue<'a> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                (self.fmt)(self.value.by_ref(), f)
            }
        }

        FormatValue {
            value,
            fmt: self.fmt,
        }
    }
}

#[derive(Debug, Clone)]
enum PartKind<'a> {
    Text {
        value: Str<'a>,
        needs_escaping: bool,
    },
    Hole {
        label: Str<'a>,
        formatter: Option<Formatter>,
    },
}

#[cfg(feature = "alloc")]
mod alloc_support {
    use super::*;

    use alloc::vec::Vec;

    impl Template<'static> {
        /**
        Create a template from a set of owned parts.
        */
        pub fn new_owned(parts: impl Into<Box<[Part<'static>]>>) -> Self {
            let parts = parts.into();

            Template {
                kind: TemplateKind::Owned(parts),
            }
        }
    }

    impl<'a> Template<'a> {
        /**
        Get a new template from this one, converting its parts into owned data.

        If the template already contains owned data then this method will simply clone it.
        */
        pub fn to_owned(&self) -> Template<'static> {
            match self.kind {
                TemplateKind::Owned(ref parts) => Template::new_owned(parts.clone()),
                ref parts => {
                    let mut dst = Vec::new();

                    for part in parts.parts() {
                        dst.push(part.to_owned());
                    }

                    Template::new_owned(dst)
                }
            }
        }
    }

    impl Part<'static> {
        /**
        Create a token for an owned fragment of literal text.
        */
        pub fn text_owned(text: impl Into<Box<str>>) -> Self {
            Part(PartKind::Text {
                value: Str::new_owned(text),
                needs_escaping: true,
            })
        }

        /**
        Create a token for a hole with an owned label.
        */
        pub fn hole_owned(label: impl Into<Box<str>>) -> Self {
            Part(PartKind::Hole {
                label: Str::new_owned(label),
                formatter: None,
            })
        }
    }

    impl<'a> Part<'a> {
        fn to_owned(&self) -> Part<'static> {
            match self.0 {
                PartKind::Text {
                    ref value,
                    needs_escaping,
                } => Part(PartKind::Text {
                    value: value.to_owned(),
                    needs_escaping,
                }),
                PartKind::Hole {
                    ref label,
                    ref formatter,
                    ..
                } => Part(PartKind::Hole {
                    label: label.to_owned(),
                    formatter: formatter.clone(),
                }),
            }
        }
    }
}

#[cfg(feature = "sval")]
impl<'k> sval::Value for Template<'k> {
    fn stream<'sval, S: sval::Stream<'sval> + ?Sized>(&'sval self, stream: &mut S) -> sval::Result {
        use sval_ref::ValueRef as _;

        self.stream_ref(stream)
    }
}

#[cfg(feature = "sval")]
impl<'k> sval_ref::ValueRef<'k> for Template<'k> {
    fn stream_ref<S: sval::Stream<'k> + ?Sized>(&self, stream: &mut S) -> sval::Result {
        if let Some(v) = self.as_literal() {
            sval_ref::stream_ref(stream, v)
        } else {
            // NOTE: This could borrow
            sval::stream_display(stream, self)
        }
    }
}

#[cfg(feature = "serde")]
impl<'k> serde::Serialize for Template<'k> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

#[cfg(feature = "sval")]
impl<'k, P: Props> sval::Value for Render<'k, P> {
    fn stream<'sval, S: sval::Stream<'sval> + ?Sized>(&'sval self, stream: &mut S) -> sval::Result {
        use sval_ref::ValueRef as _;

        self.stream_ref(stream)
    }
}

#[cfg(feature = "sval")]
impl<'k, P: Props> sval_ref::ValueRef<'k> for Render<'k, P> {
    fn stream_ref<S: sval::Stream<'k> + ?Sized>(&self, stream: &mut S) -> sval::Result {
        if let Some(v) = self.as_literal() {
            sval_ref::stream_ref(stream, v)
        } else {
            // NOTE: These could be improved to borrow in more cases
            sval::stream_display(stream, self)
        }
    }
}

#[cfg(feature = "serde")]
impl<'k, P: Props> serde::Serialize for Render<'k, P> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::hash::{BuildHasher, BuildHasherDefault, DefaultHasher};

    fn hash(h: &(impl Hash + ?Sized)) -> u64 {
        BuildHasherDefault::<DefaultHasher>::default().hash_one(h)
    }

    fn parts_to_string(parts: &[Part]) -> String {
        use std::fmt::Write as _;

        let mut buf = String::new();

        for part in parts {
            write!(&mut buf, "{part}").unwrap();
        }

        buf
    }

    #[test]
    fn literal() {
        let tpl = Template::literal("text");

        assert_eq!("text", tpl.as_literal().unwrap().get_static().unwrap());
    }

    #[test]
    fn eq() {
        for (a, b, expected) in [
            (&[Part::text("")] as &[_], &[Part::text("")] as &[_], true),
            (&[Part::text("")] as &[_], &[] as &[_], true),
            (&[Part::text("a")], &[Part::text("a")], true),
            (
                &[Part::text("a"), Part::text("b")],
                &[Part::text("ab")],
                true,
            ),
            (&[Part::hole("a")], &[Part::hole("a")], true),
            (
                &[Part::hole("a"), Part::hole("b")],
                &[Part::hole("ab")],
                false,
            ),
            (&[Part::text("a")], &[Part::text("b")], false),
            (&[Part::hole("a")], &[Part::hole("b")], false),
            (&[Part::text("a")], &[Part::hole("a")], false),
            (&[Part::text("{a}")], &[Part::hole("a")], false),
            (&[Part::text(""), Part::hole("a")], &[Part::hole("a")], true),
            (&[Part::hole("a"), Part::text("")], &[Part::hole("a")], true),
            (
                &[
                    Part::text("a"),
                    Part::text("b"),
                    Part::hole("c"),
                    Part::text(""),
                    Part::text("de"),
                ],
                &[
                    Part::text(""),
                    Part::text("ab"),
                    Part::hole("c"),
                    Part::text("de"),
                    Part::text(""),
                ],
                true,
            ),
        ] {
            let a = Template::new_ref(&a);
            let b = Template::new_ref(&b);

            let ah = hash(&a);
            let bh = hash(&b);

            assert_eq!(a, a, "{:?} == {:?}", a.kind, a.kind);
            assert_eq!(b, b, "{:?} == {:?}", b.kind, b.kind);

            assert_eq!(expected, a == b, "{:?} == {:?}", a.kind, b.kind);
            assert_eq!(expected, b == a, "{:?} == {:?}", b.kind, a.kind);

            assert_eq!(expected, ah == bh, "h({:?}) == h({:?})", a.kind, b.kind);

            assert_eq!(
                expected,
                a.to_string() == b.to_string(),
                "{:?}.to_string() == {:?}.to_string()",
                a.kind,
                b.kind
            );

            assert_eq!(
                a.to_string(),
                parts_to_string(a.kind.parts()),
                "{:?}.parts() == {:?}.parts()",
                a.kind,
                a.kind
            );
            assert_eq!(
                b.to_string(),
                parts_to_string(b.kind.parts()),
                "{:?}.parts() == {:?}.parts()",
                b.kind,
                b.kind
            );
        }
    }

    #[test]
    fn render() {
        for (case, debug, display, render_empty, render_interpolated) in [
            (
                Template::literal("text"),
                "\"text\"",
                "text",
                "text",
                "text",
            ),
            (
                Template::new({
                    const PARTS: &'static [Part<'static>] = &[Part::hole("greet")];

                    PARTS
                }),
                "\"{greet}\"",
                "{greet}",
                "{greet}",
                "user",
            ),
            (
                Template::new({
                    const PARTS: &'static [Part<'static>] =
                        &[Part::text("{user:"), Part::hole("greet"), Part::text("}")];

                    PARTS
                }),
                "\"{{user:{greet}}}\"",
                "{{user:{greet}}}",
                "{user:{greet}}",
                "{user:user}",
            ),
            (
                Template::new({
                    const PARTS: &'static [Part<'static>] = &[
                        Part::text("user is \""),
                        Part::hole("greet"),
                        Part::text("\""),
                    ];

                    PARTS
                }),
                "\"user is \\\"{greet}\\\"\"",
                "user is \"{greet}\"",
                "user is \"{greet}\"",
                "user is \"user\"",
            ),
            (
                Template::new({
                    const PARTS: &'static [Part<'static>] =
                        &[Part::text("{}").with_needs_escaping_raw(false)];

                    PARTS
                }),
                "\"{}\"",
                "{}",
                "{}",
                "{}",
            ),
            (
                Template::new({
                    const PARTS: &'static [Part<'static>] =
                        &[Part::text("Hello, "), Part::hole("greet"), Part::text("!")];

                    PARTS
                }),
                "\"Hello, {greet}!\"",
                "Hello, {greet}!",
                "Hello, {greet}!",
                "Hello, user!",
            ),
            (
                Template::new({
                    const PARTS: &'static [Part<'static>] =
                        &[Part::text("Hello"), Part::hole(""), Part::text("!")];

                    PARTS
                }),
                "\"Hello{}!\"",
                "Hello{}!",
                "Hello{}!",
                "Hello{}!",
            ),
        ] {
            assert_eq!(debug, format!("{case:?}"), "{case:?}");
            assert_eq!(debug, format!("{:?}", case.to_string()), "{case:?}");
            assert_eq!(display, case.to_string(), "{case:?}");
            assert_eq!(render_empty, case.render(Empty).to_string(), "{case:?}");
            assert_eq!(
                render_interpolated,
                case.render([("greet", "user")]).to_string(),
                "{case:?}"
            );
        }
    }

    #[test]
    fn to_value() {
        for (case, expected_display, expected_str) in [
            (Template::literal("text"), "text", Some("text")),
            (
                Template::new({
                    const PARTS: &'static [Part<'static>] =
                        &[Part::text("Hello, "), Part::hole("greet"), Part::text("!")];

                    PARTS
                }),
                "Hello, {greet}!",
                None,
            ),
        ] {
            let value = Value::from_any(&case);

            assert_eq!(expected_display, value.to_string());

            let s = value.cast::<Str>().map(|s| s.to_string());

            assert_eq!(expected_str, s.as_deref());
        }
    }

    #[cfg(feature = "sval")]
    #[test]
    fn stream() {
        sval_test::assert_tokens(
            &Template::new_ref(&[Part::text("Hello, "), Part::hole("greet"), Part::text("!")]),
            &[
                sval_test::Token::TextBegin(None),
                sval_test::Token::TextFragmentComputed("Hello, ".to_owned()),
                sval_test::Token::TextFragmentComputed("{".to_owned()),
                sval_test::Token::TextFragmentComputed("greet".to_owned()),
                sval_test::Token::TextFragmentComputed("}".to_owned()),
                sval_test::Token::TextFragmentComputed("!".to_owned()),
                sval_test::Token::TextEnd,
            ],
        );

        sval_test::assert_tokens(
            &Template::literal("Hello!"),
            &[
                sval_test::Token::TextBegin(Some(6)),
                sval_test::Token::TextFragment("Hello!"),
                sval_test::Token::TextEnd,
            ],
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serialize() {
        serde_test::assert_ser_tokens(
            &Template::new_ref(&[Part::text("Hello, "), Part::hole("greet"), Part::text("!")]),
            &[serde_test::Token::Str("Hello, {greet}!")],
        );
    }
}
