use proc_macro2::{Ident, Span, TokenStream};

use syn::{parse::Parse, spanned::Spanned, Expr, FieldValue};

use crate::{
    args::{self, Arg},
    util::ToRefTokens,
};

pub struct ExpandTokens {
    pub input: TokenStream,
}

pub struct SampleArgs {
    mdl: args::MdlArg,
    props: args::PropsArg,
    extent: args::ExtentArg,
    metric_value: MetricValueArg,
    metric_name: TokenStream,
    metric_agg: TokenStream,
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

    fn span(&self) -> Span {
        self.0.span()
    }

    pub fn to_tokens(&self) -> TokenStream {
        let expr = &self.0;

        quote_spanned!(expr.span()=> #expr)
    }
}

impl Parse for SampleArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let span = input.span();

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
        let mut metric_value = Arg::new("metric_value", |fv| {
            Ok(MetricValueArg::new(fv.expr.clone()))
        });
        let mut metric_name = Arg::token_stream("metric_name", |fv| {
            let expr = &fv.expr;

            Ok(quote_spanned!(expr.span()=> #expr))
        });
        let mut metric_agg = Arg::token_stream("metric_agg", |fv| {
            let expr = &fv.expr;

            Ok(quote_spanned!(expr.span()=> #expr))
        });

        args::set_from_field_values(
            input.parse_terminated(FieldValue::parse, Token![,])?.iter(),
            [
                &mut mdl,
                &mut extent,
                &mut props,
                &mut metric_value,
                &mut metric_name,
                &mut metric_agg,
            ],
        )?;

        let mdl = mdl.take_or_default();
        let props = props.take_or_default();
        let extent = extent.take_or_default();

        let metric_agg = metric_agg.take().unwrap_or_else(|| {
            let agg = emit_core::well_known::METRIC_AGG_LAST;

            quote!(#agg)
        });

        let metric_value = metric_value
            .take()
            .ok_or_else(|| syn::Error::new(span, "the `metric_value` parameter is required"))?;

        let metric_name = match metric_name.take() {
            Some(metric_name) => metric_name,
            None => {
                let inferred = metric_value
                    .ident()
                    .ok_or_else(|| {
                        let msg = format!("either `metric_name` needs to be specified, or `metric_value` must be an identifier to infer `metric_name` from, like: `let my_metric = {}; emit::sample!(metric_value: my_metric);`", metric_value.to_tokens());

                        syn::Error::new(metric_value.span(), msg)
                    })?
                    .to_string();

                quote_spanned!(metric_value.span()=> #inferred)
            }
        };

        Ok(SampleArgs {
            mdl,
            props,
            extent,
            metric_value,
            metric_name,
            metric_agg,
        })
    }
}

pub fn expand_tokens(opts: ExpandTokens) -> Result<TokenStream, syn::Error> {
    let args = syn::parse2::<SampleArgs>(opts.input)?;

    let extent_tokens = args.extent.to_tokens().to_ref_tokens();
    let props_tokens = args.props.to_tokens().to_ref_tokens();
    let mdl_tokens = args.mdl.to_tokens();
    let metric_name = args.metric_name;
    let metric_value = args.metric_value.to_tokens().to_ref_tokens();
    let metric_agg = args.metric_agg;

    Ok(
        quote!(emit::__private::__private_new_sample(#mdl_tokens, #extent_tokens, #props_tokens, #metric_name, #metric_agg, #metric_value)),
    )
}
