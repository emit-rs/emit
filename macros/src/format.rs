use proc_macro2::TokenStream;
use syn::spanned::Spanned;

pub struct ExpandTokens {
    pub input: TokenStream,
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
        use syn::{FieldValue, parse::Parse};

        use crate::{args, template};

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

        let span = opts.input.span();

        let (_, template, props) =
            template::parse2::<Args>(opts.input, crate::capture::default_fn_name, true)?;

        let template =
            template.ok_or_else(|| syn::Error::new(span, "missing template string literal"))?;

        let template_tokens = template.template_tokens();

        props.match_bound_props_tokens(|props_tokens| {
            Ok(quote!(emit::__private::__private_format(
                #template_tokens,
                #props_tokens,
            )))
        })
    }
}
