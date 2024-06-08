use proc_macro2::{Ident, TokenStream};
use syn::{spanned::Spanned, Expr, ExprLit, ExprPath, FieldValue, Lit};

use crate::util::{print_list, FieldValueKey};

/**
An argument represented as a field-value input to a macro.

Arguments are set from a collection of field-values using either the `set_from_parse2` or `set_from_field_values` functions.
*/
pub struct Arg<T> {
    key: &'static str,
    set: Box<dyn FnMut(&FieldValue) -> Result<T, syn::Error>>,
    value: Option<T>,
}

impl Arg<bool> {
    pub fn bool(key: &'static str) -> Self {
        Arg::new(key, move |fv| {
            if let Expr::Lit(ExprLit {
                lit: Lit::Bool(ref l),
                ..
            }) = fv.expr
            {
                Ok(l.value)
            } else {
                Err(syn::Error::new(
                    fv.expr.span(),
                    format_args!("`{}` requires a boolean value", key),
                ))
            }
        })
    }
}

impl Arg<String> {
    pub fn str(key: &'static str) -> Self {
        Arg::new(key, move |fv| {
            if let Expr::Lit(ExprLit {
                lit: Lit::Str(ref l),
                ..
            }) = fv.expr
            {
                Ok(l.value())
            } else {
                Err(syn::Error::new(
                    fv.expr.span(),
                    format_args!("`{}` requires a string value", key),
                ))
            }
        })
    }
}

impl Arg<Ident> {
    pub fn ident(key: &'static str) -> Self {
        Arg::new(key, move |fv| {
            if let Expr::Path(ExprPath { ref path, .. }) = fv.expr {
                path.get_ident().cloned().ok_or_else(|| {
                    syn::Error::new(
                        fv.expr.span(),
                        format_args!("`{}` requires an identifier value", key),
                    )
                })
            } else {
                Err(syn::Error::new(
                    fv.expr.span(),
                    format_args!("`{}` requires a string value", key),
                ))
            }
        })
    }
}

impl Arg<TokenStream> {
    pub fn token_stream(
        key: &'static str,
        to_tokens: impl FnMut(&FieldValue) -> Result<TokenStream, syn::Error> + 'static,
    ) -> Self {
        Arg::new(key, to_tokens)
    }

    pub fn take_when(self) -> TokenStream {
        self.take()
            .map(|tokens| quote!(Some(#tokens)))
            .unwrap_or_else(|| quote!(None::<emit::empty::Empty>))
    }

    pub fn take_rt(self) -> Result<TokenStream, syn::Error> {
        #[cfg(feature = "implicit_rt")]
        {
            Ok(self
                .take()
                .unwrap_or_else(|| quote!(emit::runtime::shared())))
        }
        #[cfg(not(feature = "implicit_rt"))]
        {
            use proc_macro2::Span;

            self.take().ok_or_else(|| syn::Error::new(Span::call_site(), "a runtime must be specified by the `rt` parameter unless the `implicit_rt` feature of `emit` is enabled"))
        }
    }
}

impl<T> Arg<T> {
    pub fn new(
        key: &'static str,
        to_custom: impl FnMut(&FieldValue) -> Result<T, syn::Error> + 'static,
    ) -> Self {
        Arg {
            key,
            set: Box::new(to_custom),
            value: None,
        }
    }

    pub fn peek(&self) -> Option<&T> {
        self.value.as_ref()
    }

    pub fn take(self) -> Option<T> {
        self.value
    }
}

impl<T: Default> Arg<T> {
    pub fn take_or_default(self) -> T {
        self.take().unwrap_or_default()
    }
}

pub trait ArgDef {
    fn key(&self) -> &str;
    fn set(&mut self, fv: &FieldValue) -> Result<(), syn::Error>;
}

impl<T> ArgDef for Arg<T> {
    fn key(&self) -> &str {
        self.key
    }

    fn set(&mut self, fv: &FieldValue) -> Result<(), syn::Error> {
        if self.value.is_some() {
            return Err(syn::Error::new(
                fv.span(),
                format_args!("a value for `{}` has already been specified", self.key),
            ));
        }

        self.value = Some((self.set)(fv)?);
        Ok(())
    }
}

pub fn set_from_field_values<'a, const N: usize>(
    field_values: impl Iterator<Item = &'a FieldValue> + 'a,
    mut args: [&mut dyn ArgDef; N],
) -> Result<(), syn::Error> {
    'fields: for fv in field_values {
        let key_name = fv.key_name();

        for arg in &mut args {
            if arg.key() == key_name {
                arg.set(fv)?;
                continue 'fields;
            }
        }

        return Err(syn::Error::new(
            fv.span(),
            format_args!(
                "unknown argument `{}`; available arguments are {}",
                key_name,
                print_list(|| args.iter().map(|arg| arg.key()))
            ),
        ));
    }

    Ok(())
}
