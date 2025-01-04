use proc_macro2::TokenStream;
use syn::{parse::Parse, spanned::Spanned, FieldValue, LitStr};

use crate::{
    args::{self, Arg},
    props::{push_evt_props, Props},
    template,
    util::ToRefTokens,
};

pub struct ExpandPropsTokens {
    pub input: TokenStream,
}

/**
The `props!` macro.
*/
pub fn expand_props_tokens(opts: ExpandPropsTokens) -> Result<TokenStream, syn::Error> {
    let props = syn::parse2::<Props>(opts.input)?;

    Ok(props.props_tokens())
}

pub struct ExpandTplTokens {
    pub input: TokenStream,
}

pub struct TplArgs {}

impl Parse for TplArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        args::set_from_field_values(
            input.parse_terminated(FieldValue::parse, Token![,])?.iter(),
            [],
        )?;

        Ok(TplArgs {})
    }
}

pub struct TplPartsArgs {}

impl Parse for TplPartsArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        args::set_from_field_values(
            input.parse_terminated(FieldValue::parse, Token![,])?.iter(),
            [],
        )?;

        Ok(TplPartsArgs {})
    }
}

/**
The `tpl_parts!` macro.
*/
pub fn expand_tpl_parts_tokens(opts: ExpandTplTokens) -> Result<TokenStream, syn::Error> {
    let span = opts.input.span();

    let (_, template, props) = template::parse2::<TplPartsArgs>(opts.input, false)?;

    let template =
        template.ok_or_else(|| syn::Error::new(span, "missing template string literal"))?;

    validate_props(&props)?;

    Ok(template.template_parts_tokens())
}

/**
The `tpl!` macro.
*/
pub fn expand_tpl_tokens(opts: ExpandTplTokens) -> Result<TokenStream, syn::Error> {
    let span = opts.input.span();

    let (_, template, props) = template::parse2::<TplArgs>(opts.input, false)?;

    let template =
        template.ok_or_else(|| syn::Error::new(span, "missing template string literal"))?;

    validate_props(&props)?;

    Ok(template.template_tokens())
}

fn validate_props(props: &Props) -> Result<(), syn::Error> {
    // Ensure that a standalone template only specifies identifiers
    for key_value in props.iter() {
        if !key_value.interpolated {
            return Err(syn::Error::new(
                key_value.span(),
                "key-values in raw templates must be in the template itself",
            ));
        }
    }

    Ok(())
}

pub struct ExpandPathTokens {
    pub input: TokenStream,
}

/**
The `path!` macro.
*/
pub fn expand_path_tokens(opts: ExpandPathTokens) -> Result<TokenStream, syn::Error> {
    let path = syn::parse2::<LitStr>(opts.input)?;
    let span = path.span();
    let path = path.value();

    if emit_core::path::is_valid_path(&path) {
        Ok(quote!(emit::Path::new_raw(#path)))
    } else {
        Err(syn::Error::new(span, "the value is not a valid path"))
    }
}

pub struct ExpandEvtTokens {
    pub level: Option<TokenStream>,
    pub input: TokenStream,
}

struct EvtArgs {
    mdl: args::MdlArg,
    props: args::PropsArg,
    extent: args::ExtentArg,
}

impl Parse for EvtArgs {
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

        Ok(EvtArgs {
            mdl: mdl.take_or_default(),
            extent: extent.take_or_default(),
            props: props.take_or_default(),
        })
    }
}

/**
The `evt!` macro.
*/
pub fn expand_evt_tokens(opts: ExpandEvtTokens) -> Result<TokenStream, syn::Error> {
    let span = opts.input.span();

    let (args, template, mut props) = template::parse2::<EvtArgs>(opts.input, true)?;

    let template =
        template.ok_or_else(|| syn::Error::new(span, "missing template string literal"))?;

    push_evt_props(&mut props, opts.level)?;

    let extent_tokens = args.extent.to_tokens().to_ref_tokens();
    let base_props_tokens = args.props.to_tokens().to_ref_tokens();
    let template_tokens = template.template_tokens().to_ref_tokens();
    let props_tokens = props.props_tokens().to_ref_tokens();
    let mdl_tokens = args.mdl.to_tokens().to_ref_tokens();

    Ok(
        quote!(emit::__private::__private_evt(#mdl_tokens, #template_tokens, #extent_tokens, #base_props_tokens, #props_tokens)),
    )
}
