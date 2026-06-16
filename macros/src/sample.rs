use proc_macro2::{Ident, Span, TokenStream};

use syn::{Expr, FieldValue, Member, parse::Parse, spanned::Spanned};

use emit_core::well_known::{
    KEY_METRIC_AGG, KEY_METRIC_DESCRIPTION, KEY_METRIC_NAME, KEY_METRIC_UNIT, KEY_METRIC_VALUE,
};

use crate::{
    args::{self, Arg},
    capture,
    props::Props,
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
    description: Option<TokenStream>,
    unit: Option<TokenStream>,
}

struct MetricValueArg(FieldValue);

impl MetricValueArg {
    pub fn new(value: FieldValue) -> Self {
        MetricValueArg(value)
    }

    pub fn ident(&self) -> Option<&Ident> {
        let Expr::Path(ref path) = self.0.expr else {
            return None;
        };

        path.path.get_ident()
    }

    pub fn infer_name(&self) -> syn::Result<TokenStream> {
        let inferred = self
            .ident()
            .ok_or_else(|| {
                let expr = &self.0.expr;
                let msg = format!("either `name` needs to be specified, or `value` must be an identifier to infer `name` from, like: `let my_metric = {}; emit::sample!(value: my_metric);`", quote!(#expr));

                syn::Error::new(self.span(), msg)
            })?
            .to_string();

        Ok(quote_spanned!(self.span()=> #inferred))
    }

    fn span(&self) -> Span {
        self.0.expr.span()
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
        let mut value = Arg::value("value", |fv| Ok(MetricValueArg::new(fv.clone())));
        let mut name = Arg::token_stream("name", |fv| {
            let expr = &fv.expr;

            Ok(quote_spanned!(expr.span()=> #expr))
        });
        let mut agg = Arg::token_stream("agg", |fv| {
            let expr = &fv.expr;

            Ok(quote_spanned!(expr.span()=> #expr))
        });
        let mut description = Arg::token_stream("description", |fv| {
            let expr = &fv.expr;

            Ok(quote_spanned!(expr.span()=> #expr))
        });
        let mut unit = Arg::token_stream("unit", |fv| {
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
                &mut description,
                &mut unit,
            ],
        )?;

        let rt = rt.take_or_default();
        let sampler = sampler.take();
        let mdl = mdl.take_or_default();
        let props = props.take_or_default();
        let extent = extent.take_or_default();

        let agg = agg.take();
        let description = description.take();
        let unit = unit.take();

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
            description,
            unit,
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
    let user_props_tokens = args.props.to_tokens().to_ref_tokens();
    let mdl_tokens = args.mdl.to_tokens();

    let macro_props = metric_props(
        args.name
            .map(Ok)
            .unwrap_or_else(|| args.value.infer_name())?,
        args.agg.or(opts.agg),
        args.description,
        args.unit,
        args.value.0,
    )?;

    macro_props.match_bound_props_tokens(|macro_props_tokens| {
        let props_tokens = quote!(emit::__private::__PrivateMacroExtendedProps::new(#user_props_tokens, #macro_props_tokens)).to_ref_tokens();

        Ok(quote!(emit::__private::__private_sample(#sampler_tokens, #mdl_tokens, #extent_tokens, #props_tokens)))
    })
}

pub fn expand_metric_tokens(opts: ExpandTokens) -> Result<TokenStream, syn::Error> {
    let args = syn::parse2::<Args>(opts.input)?;

    args::ensure_missing("rt", args.rt.take().map(|arg| arg.span()))?;
    args::ensure_missing("rt", args.sampler.map(|arg| arg.span()))?;

    let extent_tokens = args.extent.to_tokens().to_ref_tokens();
    let user_props_tokens = args.props.to_tokens();
    let mdl_tokens = args.mdl.to_tokens();

    let macro_props = metric_props(
        args.name
            .map(Ok)
            .unwrap_or_else(|| args.value.infer_name())?,
        args.agg.or(opts.agg),
        args.description,
        args.unit,
        args.value.0,
    )?;

    let macro_props_tokens = macro_props.gen_bound_props_tokens()?;
    let props_tokens = quote!(emit::__private::__PrivateMacroExtendedProps::new(#user_props_tokens, #macro_props_tokens));

    Ok(
        quote!(emit::__private::__must_use_metric(emit::__private::__private_metric(#mdl_tokens, #extent_tokens, #props_tokens))),
    )
}

fn metric_props(
    name: TokenStream,
    agg: Option<TokenStream>,
    description: Option<TokenStream>,
    unit: Option<TokenStream>,
    value: FieldValue,
) -> Result<Props, syn::Error> {
    let mut props = Props::new();

    let metric_name = metric_name_prop(name);
    props.push(
        &metric_name,
        capture::default_fn_name(&metric_name),
        false,
        true,
    )?;

    let metric_agg = metric_agg_prop(agg);
    props.push(
        &metric_agg,
        capture::default_fn_name(&metric_agg),
        false,
        true,
    )?;

    let metric_value = metric_value_prop(value);
    props.push(
        &metric_value,
        capture::default_fn_name(&metric_value),
        false,
        true,
    )?;

    if let Some(metric_description) = metric_description_prop(description) {
        props.push(
            &metric_description,
            capture::default_fn_name(&metric_description),
            false,
            true,
        )?;
    }

    if let Some(metric_unit) = metric_unit_prop(unit) {
        props.push(
            &metric_unit,
            capture::default_fn_name(&metric_unit),
            false,
            true,
        )?;
    }

    Ok(props)
}

fn metric_name_prop(name: TokenStream) -> FieldValue {
    let expr = name;

    FieldValue {
        attrs: vec![],
        member: Member::Named(Ident::new(KEY_METRIC_NAME, expr.span())),
        colon_token: Some(Token![:](expr.span())),
        expr: parse_quote_spanned!(expr.span()=> #expr),
    }
}

fn metric_agg_prop(agg: Option<TokenStream>) -> FieldValue {
    let expr = agg.unwrap_or_else(|| {
        let agg = emit_core::well_known::METRIC_AGG_LAST;

        quote!(#agg)
    });

    FieldValue {
        attrs: vec![],
        member: Member::Named(Ident::new(KEY_METRIC_AGG, expr.span())),
        colon_token: Some(Token![:](expr.span())),
        expr: parse_quote_spanned!(expr.span()=> #expr),
    }
}

fn metric_value_prop(value_prop: FieldValue) -> FieldValue {
    let span = value_prop.span();

    FieldValue {
        attrs: value_prop.attrs,
        member: Member::Named(Ident::new(KEY_METRIC_VALUE, span)),
        colon_token: Some(Token![:](span)),
        expr: value_prop.expr,
    }
}

fn metric_description_prop(description: Option<TokenStream>) -> Option<FieldValue> {
    let expr = description?;

    Some(FieldValue {
        attrs: vec![],
        member: Member::Named(Ident::new(KEY_METRIC_DESCRIPTION, expr.span())),
        colon_token: Some(Token![:](expr.span())),
        expr: parse_quote_spanned!(expr.span()=> #expr),
    })
}

fn metric_unit_prop(unit: Option<TokenStream>) -> Option<FieldValue> {
    let expr = unit?;

    Some(FieldValue {
        attrs: vec![],
        member: Member::Named(Ident::new(KEY_METRIC_UNIT, expr.span())),
        colon_token: Some(Token![:](expr.span())),
        expr: parse_quote_spanned!(expr.span()=> #expr),
    })
}
