use proc_macro2::TokenStream;
use syn::{parse::Parse, spanned::Spanned, FieldValue};

use crate::{
    args, capture,
    props::{push_evt_props, Props},
    template::{self, Template},
    util::{ToOptionTokens, ToRefTokens},
};

pub struct ExpandTokens {
    pub input: TokenStream,
}

struct StandaloneProps {
    fvs: Vec<FieldValue>,
}

impl Parse for StandaloneProps {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(StandaloneProps {
            fvs: input
                .parse_terminated(FieldValue::parse, Token![,])?
                .iter()
                .cloned()
                .collect(),
        })
    }
}

pub fn expand_tokens(opts: ExpandTokens) -> Result<TokenStream, syn::Error> {
    let (standalone, template, mut props) =
        template::parse2::<StandaloneProps>(opts.input, dbg_capture_fn, true)?;

    let template = match template {
        None => {
            for fv in standalone.fvs {
                props.push(&fv, dbg_capture_fn(&fv), false, true)?;
            }

            check_dbg_props(&props)?;

            compute_template(&props)?
        }
        Some(template) => {
            // If a template is supplied, ensure there are no properties before it
            if standalone.fvs.len() > 0 {
                return Err(syn::Error::new(
                    template.template_literal_tokens().span(),
                    "specify the template string literal before any properties",
                ));
            }

            check_dbg_props(&props)?;

            template
        }
    };

    push_loc_props(&mut props)?;
    push_evt_props(&mut props, Some(quote!(emit::Level::Debug)))?;

    let props_match_input_tokens = props.match_input_tokens();
    let props_match_binding_tokens = props.match_binding_tokens();
    let props_tokens = props.match_bound_tokens();

    let rt_tokens = args::RtArg::default().to_tokens()?.to_ref_tokens();
    let when_tokens = None::<TokenStream>.to_option_tokens(quote!(&emit::Empty));

    let base_props_tokens = quote!(&emit::Empty);
    let extent_tokens = quote!(&emit::Empty);
    let mdl_tokens = args::MdlArg::default().to_tokens().to_ref_tokens();

    let template_tokens = template.template_tokens().to_ref_tokens();

    let emit_tokens = quote!(
        emit::__private::__private_emit(
            #rt_tokens,
            #mdl_tokens,
            #when_tokens,
            #extent_tokens,
            #template_tokens,
            #base_props_tokens,
            #props_tokens,
        );
    );

    Ok(quote!({
        match (#(#props_match_input_tokens),*) {
            (#(#props_match_binding_tokens),*) => {
                #emit_tokens
            }
        }
    }))
}

fn push_loc_props(props: &mut Props) -> Result<(), syn::Error> {
    let fv = syn::parse2::<FieldValue>(quote!(file: file!()))?;
    props.push(&fv, capture::default_fn_name(&fv), false, true)?;

    let fv = syn::parse2::<FieldValue>(quote!(line: line!()))?;
    props.push(&fv, capture::default_fn_name(&fv), false, true)?;

    Ok(())
}

fn compute_template(props: &Props) -> Result<Template, syn::Error> {
    let mut literal = String::new();

    let mut first = true;
    for (name, key_value) in props.iter() {
        if !first {
            literal.push_str(", ");
        }
        first = false;

        literal.push_str(name);
        literal.push_str(" = {");
        literal.push_str(&key_value.hole_tokens().to_string());
        literal.push_str("}");
    }

    if !first {
        literal.push_str(" ");
    }

    literal.push_str("at {file}:{line}");

    let (pre, template, _) =
        template::parse2::<StandaloneProps>(quote!(#literal), dbg_capture_fn, true)?;

    debug_assert_eq!(0, pre.fvs.len());

    Ok(template.expect("missing template"))
}

fn dbg_capture_fn(fv: &FieldValue) -> TokenStream {
    quote_spanned!(fv.span()=> __private_capture_anon_as_debug)
}

fn check_dbg_props(props: &Props) -> Result<(), syn::Error> {
    for (k, v) in props.iter() {
        match k {
            "line" | "file" => {
                return Err(syn::Error::new(
                    v.span(),
                    format!("`{k}` can't be used as a key in the `dbg` macro"),
                ))
            }
            _ => (),
        }
    }

    Ok(())
}
