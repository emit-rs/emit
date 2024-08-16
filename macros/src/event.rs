use proc_macro2::{Span, TokenStream};
use syn::{parse::Parse, spanned::Spanned, FieldValue, Ident};

use crate::{
    args::{self, Arg},
    props::Props,
    template,
};

pub struct ExpandTokens {
    pub level: Option<TokenStream>,
    pub input: TokenStream,
}

struct Args {
    mdl: args::MdlArg,
    props: args::PropsArg,
    extent: args::ExtentArg,
}

impl Parse for Args {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut mdl = Arg::new("mdl", |fv| {
            let expr = &fv.expr;

            Ok(args::MdlArg::new(quote_spanned!(expr.span()=> #expr)))
        });
        let mut extent = Arg::new("extent", |fv| {
            let expr = &fv.expr;

            Ok(args::ExtentArg::new(quote_spanned!(expr.span()=> #expr)))
        });
        let mut props = Arg::new("props", |fv| {
            let expr = &fv.expr;

            Ok(args::PropsArg::new(quote_spanned!(expr.span()=> #expr)))
        });

        args::set_from_field_values(
            input.parse_terminated(FieldValue::parse, Token![,])?.iter(),
            [&mut mdl, &mut extent, &mut props],
        )?;

        Ok(Args {
            mdl: mdl.take_or_default(),
            extent: extent.take_or_default(),
            props: props.take_or_default(),
        })
    }
}

pub fn expand_tokens(opts: ExpandTokens) -> Result<TokenStream, syn::Error> {
    let span = opts.input.span();

    let (args, template, mut props) = template::parse2::<Args>(opts.input, true)?;

    let template =
        template.ok_or_else(|| syn::Error::new(span, "missing template string literal"))?;

    push_evt_props(&mut props, opts.level)?;

    let extent_tokens = args.extent.to_tokens();
    let base_props_tokens = args.props.to_tokens();
    let template_tokens = template.template_tokens();
    let props_tokens = props.props_tokens();
    let mdl_tokens = args.mdl.to_tokens();

    Ok(
        quote!(emit::Event::new(#mdl_tokens, #template_tokens, #extent_tokens, emit::Props::and_props(&#base_props_tokens, #props_tokens))),
    )
}

pub fn push_evt_props(props: &mut Props, level: Option<TokenStream>) -> Result<(), syn::Error> {
    // Add the level as a property
    if let Some(level_value) = level {
        let level_ident = Ident::new(emit_core::well_known::KEY_LVL, Span::call_site());

        props.push(
            &syn::parse2::<FieldValue>(quote!(#level_ident: #level_value))?,
            false,
            true,
        )?;
    }

    Ok(())
}
