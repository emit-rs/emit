use std::{fmt, marker::PhantomData};

use proc_macro2::{Span, TokenStream};
use syn::{
    ext::IdentExt,
    parse::{self, Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    Attribute, Expr, ExprField, ExprLit, ExprParen, FieldValue, Ident, Lit, LitStr, MacroDelimiter,
    Member, Meta, MetaList,
};

pub trait FieldValueKey {
    fn key_ident(&self) -> Result<Ident, syn::Error>;

    fn key_expr(&self) -> Result<ExprLit, syn::Error> {
        let ident = self.key_ident()?;

        Ok(ExprLit {
            attrs: vec![],
            lit: Lit::Str(LitStr::new(&ident.unraw().to_string(), ident.span())),
        })
    }

    fn key_name(&self) -> Result<String, syn::Error> {
        let expr = self.key_expr()?;

        match expr.lit {
            Lit::Str(s) => Ok(s.value()),
            _ => Err(syn::Error::new(
                expr.span(),
                "key expressions must be string literals",
            )),
        }
    }
}

impl FieldValueKey for FieldValue {
    fn key_ident(&self) -> Result<Ident, syn::Error> {
        match self.member {
            Member::Named(ref member) => Ok(member.clone()),
            Member::Unnamed(_) => Err(syn::Error::new(
                self.span(),
                "field values must used named identifiers",
            )),
        }
    }
}

pub trait ExprIsLocalVariable {
    fn is_local_variable(&self) -> bool;
}

impl ExprIsLocalVariable for Expr {
    fn is_local_variable(&self) -> bool {
        match self {
            Expr::Path(_) => true,
            Expr::Field(ExprField { base, .. }) => base.is_local_variable(),
            Expr::Paren(ExprParen { expr, .. }) => expr.is_local_variable(),
            _ => false,
        }
    }
}

pub trait AttributeCfg {
    fn is_cfg(&self) -> bool;
    fn invert_cfg(&self) -> Option<Attribute>;
}

impl AttributeCfg for Attribute {
    fn is_cfg(&self) -> bool {
        if let Some(ident) = self.path().get_ident() {
            ident == "cfg"
        } else {
            false
        }
    }

    fn invert_cfg(&self) -> Option<Attribute> {
        match self.path().get_ident() {
            Some(ident) if ident == "cfg" => {
                let tokens = match &self.meta {
                    Meta::Path(meta) => quote!(not(#meta)),
                    Meta::List(meta) => {
                        let meta = &meta.tokens;
                        quote!(not(#meta))
                    }
                    Meta::NameValue(meta) => quote!(not(#meta)),
                };

                Some(Attribute {
                    pound_token: self.pound_token.clone(),
                    style: self.style.clone(),
                    bracket_token: self.bracket_token.clone(),
                    meta: Meta::List(MetaList {
                        path: self.path().clone(),
                        delimiter: MacroDelimiter::Paren(Default::default()),
                        tokens,
                    }),
                })
            }
            _ => None,
        }
    }
}

pub fn maybe_cfg(cfg_attr: Option<&Attribute>, span: Span, wrap: TokenStream) -> TokenStream {
    match cfg_attr {
        Some(cfg_attr) => quote_spanned!(span=>
            #cfg_attr
            {
                #wrap
            }
        ),
        None => wrap,
    }
}

pub fn parse_comma_separated2<T: Parse>(
    tokens: TokenStream,
) -> Result<Punctuated<T, Token![,]>, syn::Error> {
    struct ParsePunctuated<T> {
        value: Punctuated<T, Token![,]>,
    }

    impl<T: Parse> Parse for ParsePunctuated<T> {
        fn parse(input: ParseStream) -> parse::Result<Self> {
            Ok(ParsePunctuated {
                value: input.parse_terminated(T::parse, Token![,])?,
            })
        }
    }

    Ok(syn::parse2::<ParsePunctuated<T>>(tokens)?.value)
}

pub trait ResultToTokens {
    fn unwrap_or_compile_error(self) -> proc_macro::TokenStream;
}

impl ResultToTokens for Result<TokenStream, syn::Error> {
    fn unwrap_or_compile_error(self) -> proc_macro::TokenStream {
        proc_macro::TokenStream::from(match self {
            Ok(item) => item,
            Err(err) => err.into_compile_error(),
        })
    }
}

pub fn print_list<'a, I: Iterator<Item = &'a str> + 'a>(
    list: impl Fn() -> I + 'a,
) -> impl fmt::Display + 'a {
    struct PrintList<F, I>(F, PhantomData<I>);

    impl<'a, F: Fn() -> I + 'a, I: Iterator<Item = &'a str>> fmt::Display for PrintList<F, I> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let mut first = true;

            for arg in (self.0)() {
                if !first {
                    write!(f, ", ")?;
                }

                first = false;

                write!(f, "`{}`", arg)?;
            }

            Ok(())
        }
    }

    PrintList(list, PhantomData)
}

pub trait ToRefTokens {
    fn to_ref_tokens(&self) -> TokenStream;
}

impl ToRefTokens for TokenStream {
    fn to_ref_tokens(&self) -> TokenStream {
        quote!(&(#self))
    }
}

pub trait ToOptionTokens {
    fn to_option_tokens(&self, none_ty_hint: TokenStream) -> TokenStream;
}

impl ToOptionTokens for Option<TokenStream> {
    fn to_option_tokens(&self, none_ty_hint: TokenStream) -> TokenStream {
        match self {
            Some(ref tokens) => quote!(emit::__private::core::option::Option::Some(#tokens)),
            None => quote!(emit::__private::core::option::Option::None::<#none_ty_hint>),
        }
    }
}

impl ToOptionTokens for TokenStream {
    fn to_option_tokens(&self, _: TokenStream) -> TokenStream {
        quote!(emit::__private::core::option::Option::Some(#self))
    }
}
