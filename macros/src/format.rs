use proc_macro2::TokenStream;
use syn::{parse::Parse, spanned::Spanned, FieldValue};

use crate::args;

pub struct ExpandTokens {
    pub input: TokenStream,
}

struct Args {}

impl Parse for Args {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        args::set_from_field_values(
            input.parse_terminated(FieldValue::parse, Token![,])?.iter(),
            [],
        )?;

        Ok(Args {})
    }
}

pub fn expand_tokens(opts: ExpandTokens) -> Result<TokenStream, syn::Error> {
    #[cfg(not(feature = "std"))]
    {
        Err(syn::Error::new(
            opts.input.span(),
            "the `format` macro is only available when the `std` Cargo feature is enabled",
        ))
    }
    #[cfg(feature = "std")]
    {
        use crate::{template, util::ToRefTokens};

        let span = opts.input.span();

        let (_, template, props) =
            template::parse2::<Args>(opts.input, crate::capture::default_fn_name, true)?;

        let template =
            template.ok_or_else(|| syn::Error::new(span, "missing template string literal"))?;

        let props_match_input_tokens = props.match_input_tokens();
        let props_match_binding_tokens = props.match_binding_tokens();
        let props_tokens = props.match_bound_tokens().to_ref_tokens();

        let template_tokens = template.template_tokens();

        Ok(quote!({
            match (#(#props_match_input_tokens),*) {
                (#(#props_match_binding_tokens),*) => {
                    emit::__private::__private_format(
                        #template_tokens,
                        #props_tokens,
                    )
                }
            }
        }))
    }
}
