use proc_macro2::{Span, TokenStream};
use syn::{parse::Parse, spanned::Spanned, FieldValue, Ident};

use crate::{
    args::{self, Arg},
    module::module_tokens,
    props::Props,
    template,
};

pub struct ExpandTokens {
    pub level: Option<TokenStream>,
    pub input: TokenStream,
}

struct Args {
    module: TokenStream,
    props: TokenStream,
    extent: TokenStream,
}

impl Parse for Args {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut module = Arg::token_stream("module", |fv| {
            let expr = &fv.expr;

            Ok(quote_spanned!(expr.span()=> #expr))
        });
        let mut extent = Arg::token_stream("extent", |fv| {
            let expr = &fv.expr;

            Ok(quote_spanned!(expr.span()=> #expr))
        });

        let mut props = Arg::token_stream("props", |fv| {
            let expr = &fv.expr;

            Ok(quote_spanned!(expr.span()=> #expr))
        });

        args::set_from_field_values(
            input.parse_terminated(FieldValue::parse, Token![,])?.iter(),
            [&mut module, &mut extent, &mut props],
        )?;

        Ok(Args {
            module: module.take().unwrap_or_else(|| module_tokens()),
            extent: extent.take().unwrap_or_else(|| quote!(emit::empty::Empty)),
            props: props.take().unwrap_or_else(|| quote!(emit::empty::Empty)),
        })
    }
}

pub fn expand_tokens(opts: ExpandTokens) -> Result<TokenStream, syn::Error> {
    let span = opts.input.span();

    let (args, template, mut props) = template::parse2::<Args>(opts.input, true)?;

    let template =
        template.ok_or_else(|| syn::Error::new(span, "missing template string literal"))?;

    push_event_props(&mut props, opts.level)?;

    let extent_tokens = args.extent;
    let base_props_tokens = args.props;
    let template_tokens = template.template_tokens();
    let props_tokens = props.props_tokens();
    let module_tokens = args.module;

    Ok(
        quote!(emit::Event::new(#module_tokens, #extent_tokens, #template_tokens, emit::Props::and_props(&#base_props_tokens, #props_tokens))),
    )
}

pub fn push_event_props(props: &mut Props, level: Option<TokenStream>) -> Result<(), syn::Error> {
    // Add the level as a property
    if let Some(level_value) = level {
        let level_ident = Ident::new(emit_core::well_known::KEY_LVL, Span::call_site());

        props.push(
            &syn::parse2::<FieldValue>(quote!(#level_ident: emit::Level::#level_value))?,
            false,
            true,
        )?;
    }

    Ok(())
}
