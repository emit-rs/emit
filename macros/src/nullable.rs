use crate::hook;
use proc_macro2::TokenStream;
use syn::{Expr, Ident, parse::Parse, punctuated::Punctuated, token::Comma};

pub struct Args {
    /*
    NOTE: Also update docs in _Control Parameters_ for this macro when adding new args
    */
}

impl Parse for Args {
    fn parse(_: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Args {})
    }
}

pub struct RenameHookTokens {
    pub args: TokenStream,
    pub expr: TokenStream,
}

pub fn rename_hook_tokens(opts: RenameHookTokens) -> Result<TokenStream, syn::Error> {
    hook::rename_hook_tokens(hook::RenameHookTokens {
        name: "nullable",
        target: "values in `emit` macros",
        args: opts.args,
        expr: opts.expr,
        predicate: |ident: &str| {
            ident.starts_with("__private_capture")
        },
        to: move |_: &Args, ident: &Ident, args: &Punctuated<Expr, Comma>| {
            if ident == "__private_captured" {
                return None;
            }

            let mut nullable_args = Punctuated::<Expr, Comma>::new();

            nullable_args.push(parse_quote_spanned!(ident.span()=> |v| v.#ident(#args)));

            let nullable_ident = Ident::new("__private_nullable", ident.span());

            Some((quote!(#nullable_ident), quote!(#nullable_args)))
        },
    })
}
