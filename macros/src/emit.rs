use proc_macro2::TokenStream;
use syn::{parse::Parse, spanned::Spanned, FieldValue};

use crate::{
    args::{self, Arg},
    event::push_evt_props,
    module::module_tokens,
    template,
};

pub struct ExpandTokens {
    pub level: Option<TokenStream>,
    pub input: TokenStream,
}

struct Args {
    rt: TokenStream,
    evt: Option<TokenStream>,
    mdl: TokenStream,
    props: TokenStream,
    extent: TokenStream,
    when: TokenStream,
}

impl Parse for Args {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut module = Arg::token_stream("mdl", |fv| {
            let expr = &fv.expr;

            Ok(quote_spanned!(expr.span()=> #expr))
        });
        let mut extent = Arg::token_stream("extent", |fv| {
            let expr = &fv.expr;

            Ok(quote_spanned!(expr.span()=> #expr))
        });
        let mut rt = Arg::token_stream("rt", |fv| {
            let expr = &fv.expr;

            Ok(quote_spanned!(expr.span()=> #expr))
        });
        let mut props = Arg::token_stream("props", |fv| {
            let expr = &fv.expr;

            Ok(quote_spanned!(expr.span()=> #expr))
        });
        let mut when = Arg::token_stream("when", |fv| {
            let expr = &fv.expr;

            Ok(quote_spanned!(expr.span()=> #expr))
        });
        let mut evt = Arg::token_stream("evt", |fv| {
            let expr = &fv.expr;

            Ok(quote_spanned!(expr.span()=> #expr))
        });

        args::set_from_field_values(
            input.parse_terminated(FieldValue::parse, Token![,])?.iter(),
            [
                &mut module,
                &mut extent,
                &mut props,
                &mut rt,
                &mut when,
                &mut evt,
            ],
        )?;

        if let Some(ref evt) = evt.peek() {
            if module.peek().is_some() || extent.peek().is_some() || props.peek().is_some() {
                return Err(syn::Error::new(evt.span(), "the `evt` argument cannot be set if any of `module`, `extent`, or `props` are also set"));
            }
        }

        Ok(Args {
            module: module.take().unwrap_or_else(|| module_tokens()),
            extent: extent.take().unwrap_or_else(|| quote!(emit::empty::Empty)),
            props: props
                .take_ref()
                .unwrap_or_else(|| quote!(emit::empty::Empty)),
            evt: evt.take_ref(),
            rt: rt.take_rt_ref()?,
            when: when.take_some_or_empty_ref(),
        })
    }
}

pub fn expand_tokens(opts: ExpandTokens) -> Result<TokenStream, syn::Error> {
    let span = opts.input.span();

    let (args, template, mut props) = template::parse2::<Args>(opts.input, true)?;

    push_evt_props(&mut props, opts.level)?;

    let props_match_input_tokens = props.match_input_tokens();
    let props_match_binding_tokens = props.match_binding_tokens();
    let props_tokens = props.match_bound_tokens();

    let rt_tokens = args.rt;
    let when_tokens = args.when;

    let emit_tokens = if let Some(event_tokens) = args.evt {
        // If the `event` parameter is present, then we can emit it without a template
        let template_tokens = template
            .map(|template| {
                let template_tokens = template.template_tokens();

                quote!(Some(#template_tokens))
            })
            .unwrap_or_else(|| quote!(None));

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
        let base_props_tokens = args.props;
        let extent_tokens = args.extent;
        let module_tokens = args.module;

        let template =
            template.ok_or_else(|| syn::Error::new(span, "missing template string literal"))?;
        let template_tokens = template.template_tokens();

        quote!(
            emit::__private::__private_emit(
                #rt_tokens,
                #module_tokens,
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
