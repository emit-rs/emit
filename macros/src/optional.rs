use crate::{
    args::{self, Arg},
    hook,
};
use proc_macro2::TokenStream;
use syn::{parse::Parse, punctuated::Punctuated, token::Comma, Expr, FieldValue, Ident};

pub struct Args {
    /*
    NOTE: Also update docs in _Control Parameters_ for this macro when adding new args
    */
    is_ref: bool,
}

impl Parse for Args {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut is_ref = Arg::bool("is_ref");

        args::set_from_field_values(
            input.parse_terminated(FieldValue::parse, Token![,])?.iter(),
            [&mut is_ref],
        )?;

        Ok(Args {
            is_ref: is_ref.take_or_default(),
        })
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
        to: move |Args { is_ref }: &Args, ident: &Ident, args: &Punctuated<Expr, Comma>| {
            if ident.to_string().starts_with("__private_captured") {
                return None;
            }

            let ident = Ident::new(
                &ident.to_string().replace(
                    "some",
                    if *is_ref {
                        "option_by_value"
                    } else {
                        "option_by_ref"
                    },
                ),
                ident.span(),
            );

            Some((quote!(#ident), quote!(#args)))
        },
    })
}
