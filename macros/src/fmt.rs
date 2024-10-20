use proc_macro2::TokenStream;
use syn::{
    parse::Parse, punctuated::Punctuated, spanned::Spanned, token::Comma, Attribute, Expr, ExprLit,
    FieldValue, Ident, LitStr,
};

use crate::{
    args::{self, Arg},
    hook, key,
};

pub fn template_hole_with_hook(
    attrs: &[Attribute],
    hole: &ExprLit,
    interpolated: bool,
    captured: bool,
) -> syn::Result<TokenStream> {
    let interpolated_expr = if interpolated {
        quote!(.__private_interpolated())
    } else {
        quote!(.__private_uninterpolated())
    };

    let captured_expr = if captured {
        quote!(.__private_captured())
    } else {
        quote!(.__private_uncaptured())
    };

    let hole = key::key_with_hook(&[], hole);

    hook::eval_hooks(
        &attrs,
        syn::parse_quote_spanned!(hole.span()=>
            #[allow(unused_imports)]
            {
                use emit::__private::{__PrivateFmtHook as _, __PrivateInterpolatedHook as _};
                emit::template::Part::hole_str(#hole).__private_fmt_as_default()#interpolated_expr #captured_expr
            }
        ),
    )
}

pub struct Args {
    /*
    NOTE: Also update docs in _Control Parameters_ for this macro when adding new args
    */
    pub flags: String,
}

impl Parse for Args {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // Accept a standalone string as a shorthand for the flags argument
        if input.peek(LitStr) {
            let flags: LitStr = input.parse()?;

            return Ok(Args {
                flags: flags.value(),
            });
        }

        let mut flags = Arg::str("flags");

        args::set_from_field_values(
            input.parse_terminated(FieldValue::parse, Token![,])?.iter(),
            [&mut flags],
        )?;

        Ok(Args {
            flags: flags.take_or_default(),
        })
    }
}

impl Args {
    fn to_format_args(&self) -> String {
        if self.flags.is_empty() {
            "{}".to_owned()
        } else {
            // `?b` -> `{:?b}`
            format!("{{:{}}}", self.flags)
        }
    }
}

pub struct RenameHookTokens {
    pub args: TokenStream,
    pub expr: TokenStream,
}

pub fn rename_hook_tokens(opts: RenameHookTokens) -> Result<TokenStream, syn::Error> {
    hook::rename_hook_tokens(hook::RenameHookTokens {
        name: "fmt",
        target: "values in templates or event macros",
        args: opts.args,
        expr: opts.expr,
        predicate: |ident: &str| {
            ident.starts_with("__private_fmt") || ident.starts_with("__private_interpolated")
        },
        to: move |args: &Args, ident: &Ident, _: &Punctuated<Expr, Comma>| {
            if ident.to_string().starts_with("__private_interpolated") {
                return None;
            }

            let fmt = args.to_format_args();

            let to_ident = quote!(__private_fmt_as);
            let to_arg = quote!(emit::template::Formatter::new(|v, f| {
                emit::__private::core::write!(f, #fmt, v)
            }));

            Some((to_ident, to_arg))
        },
    })
}
