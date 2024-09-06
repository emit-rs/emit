use proc_macro2::TokenStream;
use syn::{parse::Parse, spanned::Spanned, FieldValue};

use crate::{
    args::{self, Arg},
    props::{check_evt_props, push_evt_props},
    template,
    util::{ToOptionTokens, ToRefTokens},
};

pub struct ExpandTokens {
    pub level: Option<TokenStream>,
    pub input: TokenStream,
}

struct Args {
    rt: args::RtArg,
    evt: Option<TokenStream>,
    mdl: args::MdlArg,
    props: args::PropsArg,
    extent: args::ExtentArg,
    when: args::WhenArg,
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
        let mut rt = Arg::new("rt", |fv| {
            let expr = &fv.expr;

            Ok(args::RtArg::new(quote_spanned!(expr.span()=> #expr)))
        });
        let mut props = Arg::new("props", |fv| {
            let expr = &fv.expr;

            Ok(args::PropsArg::new(quote_spanned!(expr.span()=> #expr)))
        });
        let mut when = Arg::new("when", |fv| {
            let expr = &fv.expr;

            Ok(args::WhenArg::new(quote_spanned!(expr.span()=> #expr)))
        });
        let mut evt = Arg::token_stream("evt", |fv| {
            let expr = &fv.expr;

            Ok(quote_spanned!(expr.span()=> #expr))
        });

        args::set_from_field_values(
            input.parse_terminated(FieldValue::parse, Token![,])?.iter(),
            [
                &mut mdl,
                &mut extent,
                &mut props,
                &mut rt,
                &mut when,
                &mut evt,
            ],
        )?;

        if let Some(ref evt) = evt.peek() {
            if mdl.peek().is_some() || extent.peek().is_some() || props.peek().is_some() {
                return Err(syn::Error::new(evt.span(), "the `evt` argument cannot be set if any of `mdl`, `extent`, or `props` are also set"));
            }
        }

        Ok(Args {
            mdl: mdl.take_or_default(),
            extent: extent.take_or_default(),
            props: props.take_or_default(),
            evt: evt.take(),
            rt: rt.take_or_default(),
            when: when.take_or_default(),
        })
    }
}

pub fn expand_tokens(opts: ExpandTokens) -> Result<TokenStream, syn::Error> {
    let span = opts.input.span();

    let (args, template, mut props) = template::parse2::<Args>(opts.input, true)?;

    check_evt_props(&props)?;
    push_evt_props(&mut props, opts.level)?;

    let props_match_input_tokens = props.match_input_tokens();
    let props_match_binding_tokens = props.match_binding_tokens();
    let props_tokens = props.match_bound_tokens();

    let rt_tokens = args.rt.to_tokens()?.to_ref_tokens();
    let when_tokens = args
        .when
        .to_tokens()
        .map(|when| when.to_ref_tokens())
        .to_option_tokens(quote!(&emit::Empty));

    let emit_tokens = if let Some(event_tokens) = args.evt {
        // If the `event` parameter is present, then we can emit it without a template
        let template_tokens = template
            .map(|template| template.template_tokens().to_ref_tokens())
            .to_option_tokens(quote!(&emit::Template));
        let event_tokens = event_tokens.to_ref_tokens();

        quote!(
            emit::__private::__private_emit_event(
                #rt_tokens,
                #when_tokens,
                #event_tokens,
                #template_tokens,
                #props_tokens,
            );
        )
    } else {
        let base_props_tokens = args.props.to_tokens().to_ref_tokens();
        let extent_tokens = args.extent.to_tokens().to_ref_tokens();
        let mdl_tokens = args.mdl.to_tokens().to_ref_tokens();

        let template =
            template.ok_or_else(|| syn::Error::new(span, "missing template string literal"))?;
        let template_tokens = template.template_tokens().to_ref_tokens();

        quote!(
            emit::__private::__private_emit(
                #rt_tokens,
                #mdl_tokens,
                #when_tokens,
                #extent_tokens,
                #template_tokens,
                #base_props_tokens,
                #props_tokens,
            );
        )
    };

    Ok(quote!({
        match (#(#props_match_input_tokens),*) {
            (#(#props_match_binding_tokens),*) => {
                #emit_tokens
            }
        }
    }))
}
