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

use core::{cmp, fmt, slice};

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
pub struct Template<'a>(TemplateKind<'a>);

#[derive(Clone)]
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
        fmt::Debug::fmt(&self.render(Empty), f)
    }
}

impl<'a> fmt::Display for Template<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.render(Empty), f)
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
        Template(TemplateKind::Parts(parts))
    }

    /**
    Create a template from a string literal with no holes.
    */
    pub const fn literal(text: &'static str) -> Self {
        Template(TemplateKind::Literal([Part::text(text)]))
    }
}

impl<'a> Template<'a> {
    /**
    Create a template from a borrowed set of tokens.

    The [`Template::new`] method should be preferred where possible.
    */
    pub const fn new_ref(parts: &'a [Part<'a>]) -> Self {
        Template(TemplateKind::Parts(parts))
    }

    /**
    Create a template from a string literal with no holes.

    The [`Template::literal`] method should be preferred where possible.
    */
    pub const fn literal_ref(text: &'a str) -> Self {
        Template(TemplateKind::Literal([Part::text_ref(text)]))
    }

    /**
    Get a new template, borrowing data from this one.
    */
    pub fn by_ref<'b>(&'b self) -> Template<'b> {
        match self.0 {
            TemplateKind::Literal([ref part]) => Template(TemplateKind::Literal([part.by_ref()])),
            TemplateKind::Parts(parts) => Template(TemplateKind::Parts(parts)),
            #[cfg(feature = "alloc")]
            TemplateKind::Owned(ref parts) => Template(TemplateKind::Parts(parts)),
        }
    }

    /**
    Try get the value of the template as a string literal.

    If the template only has a single token, and that token is text, then this method will return `Some`. Otherwise this method will return `None`.
    */
    pub fn as_literal(&'_ self) -> Option<&'_ Str<'a>> {
        match self.0.parts() {
            [part] => part.as_text(),
            _ => None,
        }
    }

    /**
    Iterate over the parts of the template.
    */
    pub fn parts(&'_ self) -> Parts<'_, 'a> {
        Parts(self.0.parts().iter())
    }

    /**
    Lazily render the template, using the given properties for interpolation.
    */
    pub fn render<'b, P>(&'b self, props: P) -> Render<'b, P> {
        Render {
            tpl: self.by_ref(),
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
    fn to_value(&self) -> Value {
        if let Some(tpl) = self.as_literal() {
            Value::from_any(tpl)
        } else {
            Value::from_display(self)
        }
    }
}

impl<'a, 'b> PartialEq<Template<'b>> for Template<'a> {
    fn eq(&self, other: &Template<'b>) -> bool {
        // Optimize for the case where both templates are just text literals
        if let (Some(a), Some(b)) = (self.as_literal(), other.as_literal()) {
            return a == b;
        }

        let mut ai = 0;
        let mut ati = 0;
        let mut bi = 0;
        let mut bti = 0;

        let a = self.0.parts();
        let b = other.0.parts();

        while ai < a.len() && bi < b.len() {
            let ap = &a[ai];
            let bp = &b[bi];

            match (&ap.0, &bp.0) {
                (PartKind::Text { value: ref a }, PartKind::Text { value: ref b }) => {
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
                (PartKind::Hole { label: ref a, .. }, PartKind::Hole { label: ref b, .. }) => {
                    if a != b {
                        return false;
                    }

                    ai += 1;
                    bi += 1;

                    continue;
                }
                _ => return false,
            }
        }

        // If there's any data left then it would have to be empty text
        for part in a[ai..].iter().chain(b[bi..].iter()) {
            let PartKind::Text { ref value } = part.0 else {
                return false;
            };

            if !value.get().is_empty() {
                return false;
            }
        }

        // If all data was processed then the templates are equal
        true
    }
}

impl<'a> Eq for Template<'a> {}

/**
The result of calling [`Template::render`].

The template can be converted to text either using the [`fmt::Display`] implementation of `Render`, or by calling [`Render::write`] with an instance of [`Write`].
*/
pub struct Render<'a, P> {
    tpl: Template<'a>,
    props: P,
}

impl<'a, P> Render<'a, P> {
    /**
    Set the properties to interpolate.
    */
    pub fn with_props<U>(self, props: U) -> Render<'a, U> {
        Render {
            tpl: self.tpl,
            props,
        }
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
        for part in self.tpl.0.parts() {
            part.write(&mut writer, &self.props)?;
        }

        Ok(())
    }
}

impl<'a, P: Props> ToValue for Render<'a, P> {
    fn to_value(&self) -> Value {
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

        f.write_char('"')?;
        write!(WriteEscaped(&mut *f), "{}", self)?;
        f.write_char('"')
    }
}

struct WriteEscaped<W>(W);

impl<W: fmt::Write> WriteEscaped<W> {
    pub fn write_fmt(&mut self, args: fmt::Arguments) -> fmt::Result {
        self.0.write_fmt(args)
    }
}

impl<W: fmt::Write> fmt::Write for WriteEscaped<W> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.escape_debug() {
            self.0.write_char(c)?;
        }

        Ok(())
    }
}

/**
An individual token in a [`Template`].
*/
#[derive(Clone)]
pub struct Part<'a>(PartKind<'a>);

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
        Part(PartKind::Text { value: text })
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
            PartKind::Text { ref value, .. } => Some(value),
            _ => None,
        }
    }

    /**
    Try get the label of the part.

    If the part is a [`Part::hole`] this method will return `Some`. Otherwise it will return `None.
    */
    pub const fn label(&'_ self) -> Option<&'_ Str<'a>> {
        match self.0 {
            PartKind::Hole { ref label, .. } => Some(label),
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
            PartKind::Text { ref value } => Part(PartKind::Text {
                value: value.by_ref(),
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

    fn write(&self, mut writer: impl Write, props: impl Props) -> fmt::Result {
        match self.0 {
            PartKind::Text { ref value, .. } => writer.write_text(value.get()),
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

#[derive(Clone)]
enum PartKind<'a> {
    Text {
        value: Str<'a>,
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

            Template(TemplateKind::Owned(parts))
        }
    }

    impl<'a> Template<'a> {
        /**
        Get a new template from this one, converting its parts into owned data.

        If the template already contains owned data then this method will simply clone it.
        */
        pub fn to_owned(&self) -> Template<'static> {
            match self.0 {
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
                PartKind::Text { ref value, .. } => Part(PartKind::Text {
                    value: value.to_owned(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal() {
        let tpl = Template::literal("text");

        assert_eq!("text", tpl.as_literal().unwrap().get_static().unwrap());
    }

    #[test]
    fn eq() {
        let a = [
            Part::text("a"),
            Part::text("b"),
            Part::hole("c"),
            Part::text(""),
            Part::text("de"),
        ];
        let a = Template::new_ref(&a);

        let b = [
            Part::text(""),
            Part::text("ab"),
            Part::hole("c"),
            Part::text("de"),
            Part::text(""),
        ];
        let b = Template::new_ref(&b);

        assert_eq!(a, b);
    }

    #[test]
    fn render() {
        for (case, raw, interpolated) in [
            (Template::literal("text"), "text", "text"),
            (
                Template::new({
                    const PARTS: &'static [Part<'static>] = &[Part::hole("greet")];

                    PARTS
                }),
                "{greet}",
                "user",
            ),
            (
                Template::new({
                    const PARTS: &'static [Part<'static>] =
                        &[Part::text("Hello, "), Part::hole("greet"), Part::text("!")];

                    PARTS
                }),
                "Hello, {greet}!",
                "Hello, user!",
            ),
            (
                Template::new({
                    const PARTS: &'static [Part<'static>] =
                        &[Part::text("Hello"), Part::hole(""), Part::text("!")];

                    PARTS
                }),
                "Hello{}!",
                "Hello{}!",
            ),
        ] {
            assert_eq!(raw, case.render(Empty).to_string());
            assert_eq!(interpolated, case.render([("greet", "user")]).to_string());
        }
    }

    #[test]
    fn to_value() {
        let tpl = Template::literal("text");

        let value = Value::from_any(&tpl);

        assert_eq!("text", value.to_string());

        let s = value.cast::<Str>().unwrap();

        assert_eq!("text", s);

        let tpl = Template::new({
            const PARTS: &'static [Part<'static>] =
                &[Part::text("Hello, "), Part::hole("greet"), Part::text("!")];

            PARTS
        });

        let value = Value::from_any(&tpl);

        assert_eq!("Hello, {greet}!", value.to_string());
        assert!(value.cast::<Str>().is_none());
    }
}
