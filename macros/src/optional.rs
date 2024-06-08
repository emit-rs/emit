use crate::hook;
use proc_macro2::TokenStream;
use syn::{parse::Parse, punctuated::Punctuated, token::Comma, Expr, Ident};

pub struct Args;

impl Parse for Args {
    fn parse(_: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Args)
    }
}

pub struct RenameHookTokens {
    pub args: TokenStream,
    pub expr: TokenStream,
}

pub fn rename_hook_tokens(opts: RenameHookTokens) -> Result<TokenStream, syn::Error> {
    hook::rename_hook_tokens(hook::RenameHookTokens {
        name: "optional",
        target: "values in `emit` macros",
        args: opts.args,
        expr: opts.expr,
        predicate: |ident: &str| {
            ident.starts_with("__private_optional") || ident.starts_with("__private_captured")
        },
        to: move |_: &Args, ident: &Ident, args: &Punctuated<Expr, Comma>| {
            if ident.to_string().starts_with("__private_captured") {
                return None;
            }

            let ident = Ident::new(&ident.to_string().replace("some", "option"), ident.span());

            Some((quote!(#ident), quote!(#args)))
        },
    })
}
