use proc_macro2::TokenStream;
use syn::{parse::Parse, spanned::Spanned, FieldValue};

use crate::args::{self, Arg};

pub struct ExpandTokens {
    pub input: TokenStream,
}

struct Args {
    rt: TokenStream,
}

impl Parse for Args {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut rt = Arg::token_stream("rt", |fv| {
            let expr = &fv.expr;

            Ok(quote_spanned!(expr.span()=> #expr))
        });

        args::set_from_field_values(
            input.parse_terminated(FieldValue::parse, Token![,])?.iter(),
            [&mut rt],
        )?;

        Ok(Args { rt: rt.take_rt()? })
    }
}

pub fn expand_now_tokens(opts: ExpandTokens) -> Result<TokenStream, syn::Error> {
    let args = syn::parse2::<Args>(opts.input)?;

    let rt_tokens = args.rt;

    Ok(quote!(emit::__private::__private_now(#rt_tokens)))
}

pub fn expand_start_timer_tokens(opts: ExpandTokens) -> Result<TokenStream, syn::Error> {
    let args = syn::parse2::<Args>(opts.input)?;

    let rt_tokens = args.rt;

    Ok(quote!(emit::__private::__private_start_timer(#rt_tokens)))
}
