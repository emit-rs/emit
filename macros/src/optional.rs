use crate::hook;
use proc_macro2::TokenStream;
use syn::{parse::Parse, punctuated::Punctuated, token::Comma, Expr, Ident};

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
        name: "optional",
        target: "values in `emit` macros",
        args: opts.args,
        expr: opts.expr,
        predicate: |ident: &str| {
            ident.starts_with("__private_capture") || ident.starts_with("__private_captured")
        },
        to: move |_: &Args, ident: &Ident, args: &Punctuated<Expr, Comma>| {
            if ident.to_string().starts_with("__private_captured") {
                return None;
            }

            let mut optional_args = Punctuated::<Expr, Comma>::new();

            optional_args.push(parse_quote_spanned!(ident.span()=> |v| v.#ident(#args)));

            let optional_ident = Ident::new("__private_optional", ident.span());

            Some((quote!(#optional_ident), quote!(#optional_args)))
        },
    })
}
