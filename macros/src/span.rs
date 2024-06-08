use proc_macro2::{Ident, Span, TokenStream};
use syn::{
    parse::Parse, spanned::Spanned, Block, Expr, ExprAsync, ExprBlock, FieldValue, Item, ItemFn,
    Signature, Stmt,
};

use crate::{
    args::{self, Arg},
    event::push_event_props,
    module::module_tokens,
    props::Props,
    template::{self, Template},
};

pub struct ExpandTokens {
    pub level: Option<TokenStream>,
    pub item: TokenStream,
    pub input: TokenStream,
}

struct Args {
    rt: TokenStream,
    module: TokenStream,
    when: TokenStream,
    arg: Option<Ident>,
}

impl Parse for Args {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut rt = Arg::token_stream("rt", |fv| {
            let expr = &fv.expr;

            Ok(quote_spanned!(expr.span()=> #expr))
        });
        let mut module = Arg::token_stream("module", |fv| {
            let expr = &fv.expr;

            Ok(quote_spanned!(expr.span()=> #expr))
        });
        let mut when = Arg::token_stream("when", |fv| {
            let expr = &fv.expr;

            Ok(quote_spanned!(expr.span()=> #expr))
        });
        let mut arg = Arg::ident("arg");

        args::set_from_field_values(
            input.parse_terminated(FieldValue::parse, Token![,])?.iter(),
            [&mut module, &mut arg, &mut rt, &mut when],
        )?;

        Ok(Args {
            rt: rt.take_rt()?,
            module: module.take().unwrap_or_else(|| module_tokens()),
            when: when.take_when(),
            arg: arg.take(),
        })
    }
}

pub fn expand_tokens(opts: ExpandTokens) -> Result<TokenStream, syn::Error> {
    let span = opts.input.span();

    let (args, template, ctxt_props) = template::parse2::<Args>(opts.input, true)?;

    let template =
        template.ok_or_else(|| syn::Error::new(span, "missing template string literal"))?;

    let mut evt_props = Props::new();
    push_event_props(&mut evt_props, opts.level)?;

    let span_arg = args
        .arg
        .unwrap_or_else(|| Ident::new("__span", Span::call_site()));

    let module_tokens = args.module;

    let mut item = syn::parse2::<Stmt>(opts.item)?;
    match &mut item {
        // A synchronous function
        Stmt::Item(Item::Fn(ItemFn {
            block,
            sig: Signature {
                asyncness: None, ..
            },
            ..
        })) => {
            **block = syn::parse2::<Block>(inject_sync(
                &args.rt,
                &module_tokens,
                &args.when,
                &template,
                &ctxt_props,
                &evt_props,
                &span_arg,
                quote!(#block),
            ))?;
        }
        // A synchronous block
        Stmt::Expr(Expr::Block(ExprBlock { block, .. }), _) => {
            *block = syn::parse2::<Block>(inject_sync(
                &args.rt,
                &module_tokens,
                &args.when,
                &template,
                &ctxt_props,
                &evt_props,
                &span_arg,
                quote!(#block),
            ))?;
        }
        // An asynchronous function
        Stmt::Item(Item::Fn(ItemFn {
            block,
            sig: Signature {
                asyncness: Some(_), ..
            },
            ..
        })) => {
            **block = syn::parse2::<Block>(inject_async(
                &args.rt,
                &module_tokens,
                &args.when,
                &template,
                &ctxt_props,
                &evt_props,
                &span_arg,
                quote!(#block),
            ))?;
        }
        // An asynchronous block
        Stmt::Expr(Expr::Async(ExprAsync { block, .. }), _) => {
            *block = syn::parse2::<Block>(inject_async(
                &args.rt,
                &module_tokens,
                &args.when,
                &template,
                &ctxt_props,
                &evt_props,
                &span_arg,
                quote!(#block),
            ))?;
        }
        _ => return Err(syn::Error::new(item.span(), "unrecognized item type")),
    }

    Ok(quote!(#item))
}

fn inject_sync(
    rt_tokens: &TokenStream,
    module_tokens: &TokenStream,
    when_tokens: &TokenStream,
    template: &Template,
    ctxt_props: &Props,
    evt_props: &Props,
    span_arg: &Ident,
    body: TokenStream,
) -> TokenStream {
    let ctxt_props_tokens = ctxt_props.props_tokens();
    let evt_props_tokens = evt_props.props_tokens();
    let template_tokens = template.template_tokens();
    let template_literal_tokens = template.template_literal_tokens();

    quote!({
        let (mut __ctxt, __span_arg) = emit::__private::__private_begin_span(
            #rt_tokens,
            #module_tokens,
            #when_tokens,
            #template_tokens,
            #ctxt_props_tokens,
            #evt_props_tokens,
            #template_literal_tokens,
            |span| {
                emit::__private::__private_complete_span(
                    #rt_tokens,
                    span,
                    #template_tokens,
                    #evt_props_tokens,
                )
            }
        );
        let __ctxt_guard = __ctxt.enter();

        let #span_arg = __span_arg;

        #body
    })
}

fn inject_async(
    rt_tokens: &TokenStream,
    module_tokens: &TokenStream,
    when_tokens: &TokenStream,
    template: &Template,
    ctxt_props: &Props,
    evt_props: &Props,
    span_arg: &Ident,
    body: TokenStream,
) -> TokenStream {
    let ctxt_props_tokens = ctxt_props.props_tokens();
    let evt_props_tokens = evt_props.props_tokens();
    let template_tokens = template.template_tokens();
    let template_literal_tokens = template.template_literal_tokens();

    quote!({
        let (__ctxt, __span_arg) = emit::__private::__private_begin_span(
            #rt_tokens,
            #module_tokens,
            #when_tokens,
            #template_tokens,
            #ctxt_props_tokens,
            #evt_props_tokens,
            #template_literal_tokens,
            |span| {
                emit::__private::__private_complete_span(
                    #rt_tokens,
                    span,
                    #template_tokens,
                    #evt_props_tokens,
                )
            }
        );

        __ctxt.in_future(async move {
            let #span_arg = __span_arg;

            async #body.await
        }).await
    })
}
