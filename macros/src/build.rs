use proc_macro2::TokenStream;
use syn::{parse::Parse, spanned::Spanned, FieldValue};

use crate::{args, props::Props, template};

pub struct ExpandPropsTokens {
    pub input: TokenStream,
}

pub fn expand_props_tokens(opts: ExpandPropsTokens) -> Result<TokenStream, syn::Error> {
    let props = syn::parse2::<Props>(opts.input)?;

    Ok(props.props_tokens())
}

pub struct ExpandTemplateTokens {
    pub input: TokenStream,
}

pub struct TemplateArgs {}

impl Parse for TemplateArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        args::set_from_field_values(
            input.parse_terminated(FieldValue::parse, Token![,])?.iter(),
            [],
        )?;

        Ok(TemplateArgs {})
    }
}

pub struct PartsArgs {}

impl Parse for PartsArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        args::set_from_field_values(
            input.parse_terminated(FieldValue::parse, Token![,])?.iter(),
            [],
        )?;

        Ok(PartsArgs {})
    }
}

pub fn expand_template_parts_tokens(opts: ExpandTemplateTokens) -> Result<TokenStream, syn::Error> {
    let span = opts.input.span();

    let (_, template, props) = template::parse2::<PartsArgs>(opts.input, false)?;

    let template =
        template.ok_or_else(|| syn::Error::new(span, "missing template string literal"))?;

    validate_props(&props)?;

    Ok(template.template_parts_tokens())
}

pub fn expand_template_tokens(opts: ExpandTemplateTokens) -> Result<TokenStream, syn::Error> {
    let span = opts.input.span();

    let (_, template, props) = template::parse2::<TemplateArgs>(opts.input, false)?;

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
