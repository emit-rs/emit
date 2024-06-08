use proc_macro2::TokenStream;
use syn::{
    parse::Parse, punctuated::Punctuated, spanned::Spanned, token::Comma, Attribute, Expr, ExprLit,
    FieldValue, Ident, Lit, LitStr,
};

use crate::{
    args::{self, Arg},
    hook,
};

pub fn key_with_hook(attrs: &[Attribute], key_expr: &ExprLit) -> TokenStream {
    quote_spanned!(key_expr.span()=>
        #(#attrs)*
        #[allow(unused_imports)]
        {
            use emit::__private::__PrivateKeyHook as _;
            emit::Str::new(#key_expr).__private_key_as_default()
        }
    )
}

pub struct Args {
    pub name: Name,
}

pub enum Name {
    Str(String),
    Any(TokenStream),
}

impl Parse for Args {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // Accept a standalone string as a shorthand for the key name
        let name = if input.peek(LitStr) {
            let value: LitStr = input.parse()?;

            Name::Str(value.value())
        } else {
            let mut name = Arg::new("name", |fv| {
                let expr = &fv.expr;

                if let Expr::Lit(ExprLit {
                    attrs: _,
                    lit: Lit::Str(lit),
                }) = expr
                {
                    Ok(Name::Str(lit.value()))
                } else {
                    Ok(Name::Any(quote_spanned!(expr.span()=> #expr)))
                }
            });

            args::set_from_field_values(
                input.parse_terminated(FieldValue::parse, Token![,])?.iter(),
                [&mut name],
            )?;

            name.take()
                .ok_or_else(|| syn::Error::new(input.span(), "the `name` argument is missing"))?
        };

        Ok(Args { name })
    }
}

pub struct RenameHookTokens {
    pub args: TokenStream,
    pub expr: TokenStream,
}

pub fn rename_hook_tokens(opts: RenameHookTokens) -> Result<TokenStream, syn::Error> {
    hook::rename_hook_tokens(hook::RenameHookTokens {
        name: "key",
        target: "values in templates or event macros",
        args: opts.args,
        expr: opts.expr,
        predicate: |ident: &str| ident.starts_with("__private_key"),
        to: move |args: &Args, _: &Ident, _: &Punctuated<Expr, Comma>| {
            let (to_ident, to_arg) = match args.name {
                Name::Str(ref name) => (quote!(__private_key_as_static), quote!(#name)),
                Name::Any(ref name) => (quote!(__private_key_as), name.clone()),
            };

            Some((to_ident, to_arg))
        },
    })
}
