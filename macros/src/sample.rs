use proc_macro2::{Ident, Span, TokenStream};

use syn::{parse::Parse, spanned::Spanned, Expr, FieldValue};

use crate::{
    args::{self, Arg},
    util::ToRefTokens,
};

pub struct ExpandTokens {
    pub agg: Option<TokenStream>,
    pub input: TokenStream,
}

pub struct Args {
    /*
    NOTE: Also update docs in _Control Parameters_ for this macro when adding new args
    */
    rt: args::RtArg,
    sampler: Option<TokenStream>,
    mdl: args::MdlArg,
    props: args::PropsArg,
    extent: args::ExtentArg,
    value: MetricValueArg,
    name: Option<TokenStream>,
    agg: Option<TokenStream>,
}

struct MetricValueArg(Expr);

impl MetricValueArg {
    pub fn new(value: Expr) -> Self {
        MetricValueArg(value)
    }

    pub fn ident(&self) -> Option<&Ident> {
        let Expr::Path(ref path) = self.0 else {
            return None;
        };

        path.path.get_ident()
    }

    pub fn infer_name(&self) -> syn::Result<TokenStream> {
        let inferred = self
            .ident()
            .ok_or_else(|| {
                let msg = format!("either `name` needs to be specified, or `value` must be an identifier to infer `name` from, like: `let my_metric = {}; emit::sample!(value: my_metric);`", self.to_tokens());

                syn::Error::new(self.span(), msg)
            })?
            .to_string();

        Ok(quote_spanned!(self.span()=> #inferred))
    }

    fn span(&self) -> Span {
        self.0.span()
    }

    pub fn to_tokens(&self) -> TokenStream {
        let expr = &self.0;

        quote_spanned!(expr.span()=> #expr)
    }
}

impl Parse for Args {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut rt = Arg::new("rt", |fv| {
            let expr = &fv.expr;

            Ok(args::RtArg::new(quote_spanned!(expr.span()=> #expr)))
        });
        let mut sampler = Arg::token_stream("sampler", |fv| {
            let expr = &fv.expr;

            Ok(quote_spanned!(expr.span()=> #expr))
        });
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
        let mut value = Arg::new("value", |fv| Ok(MetricValueArg::new(fv.expr.clone())));
        let mut name = Arg::token_stream("name", |fv| {
            let expr = &fv.expr;

            Ok(quote_spanned!(expr.span()=> #expr))
        });
        let mut agg = Arg::token_stream("agg", |fv| {
            let expr = &fv.expr;

            Ok(quote_spanned!(expr.span()=> #expr))
        });

        args::set_from_field_values(
            input.parse_terminated(FieldValue::parse, Token![,])?.iter(),
            [
                &mut rt,
                &mut sampler,
                &mut mdl,
                &mut extent,
                &mut props,
                &mut value,
                &mut name,
                &mut agg,
            ],
        )?;

        let rt = rt.take_or_default();
        let sampler = sampler.take();
        let mdl = mdl.take_or_default();
        let props = props.take_or_default();
        let extent = extent.take_or_default();

        let agg = agg.take();

        let value = value.take().ok_or_else(|| {
            syn::Error::new(Span::call_site(), "the `value` parameter is required")
        })?;

        let name = name.take();

        Ok(Args {
            rt,
            sampler,
            mdl,
            props,
            extent,
            value,
            name,
            agg,
        })
    }
}

pub fn expand_tokens(opts: ExpandTokens) -> Result<TokenStream, syn::Error> {
    let args = syn::parse2::<Args>(opts.input)?;

    let sampler_tokens = match args.sampler {
        Some(sampler) => sampler,
        None => {
            let rt = args.rt.to_tokens()?.to_ref_tokens();

            quote!(emit::__private::__private_default_sampler(#rt))
        }
    };

    let extent_tokens = args.extent.to_tokens().to_ref_tokens();
    let props_tokens = args.props.to_tokens().to_ref_tokens();
    let mdl_tokens = args.mdl.to_tokens();
    let name = if let Some(name) = args.name {
        name
    } else {
        args.value.infer_name()?
    };
    let value = args.value.to_tokens().to_ref_tokens();

    let agg = args.agg.or(opts.agg).unwrap_or_else(|| {
        let agg = emit_core::well_known::METRIC_AGG_LAST;

        quote!(#agg)
    });

    Ok(
        quote!(emit::__private::__private_sample(#sampler_tokens, #mdl_tokens, #extent_tokens, #props_tokens, #name, #agg, #value)),
    )
}

pub fn expand_new_tokens(opts: ExpandTokens) -> Result<TokenStream, syn::Error> {
    let args = syn::parse2::<Args>(opts.input)?;

    args::ensure_missing("rt", args.rt.take().map(|arg| arg.span()))?;
    args::ensure_missing("rt", args.sampler.map(|arg| arg.span()))?;

    let extent_tokens = args.extent.to_tokens().to_ref_tokens();
    let props_tokens = args.props.to_tokens().to_ref_tokens();
    let mdl_tokens = args.mdl.to_tokens();
    let name = if let Some(name) = args.name {
        name
    } else {
        args.value.infer_name()?
    };
    let value = args.value.to_tokens().to_ref_tokens();

    let agg = args.agg.or(opts.agg).unwrap_or_else(|| {
        let agg = emit_core::well_known::METRIC_AGG_LAST;

        quote!(#agg)
    });

    Ok(
        quote!(emit::__private::__private_new_sample(#mdl_tokens, #extent_tokens, #props_tokens, #name, #agg, #value)),
    )
}
