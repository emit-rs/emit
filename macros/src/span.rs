use proc_macro2::{Ident, Span, TokenStream};
use syn::{
    parse::Parse, spanned::Spanned, Block, Expr, ExprAsync, ExprBlock, FieldValue, Item, ItemFn,
    Signature, Stmt,
};

use crate::{
    args::{self, Arg},
    event::push_evt_props,
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
    guard: Option<Ident>,
    ok_lvl: Option<TokenStream>,
    err_lvl: Option<TokenStream>,
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
        let mut ok_lvl = Arg::token_stream("ok_lvl", |fv| {
            let expr = &fv.expr;

            Ok(quote_spanned!(expr.span()=> #expr))
        });
        let mut err_lvl = Arg::token_stream("err_lvl", |fv| {
            let expr = &fv.expr;

            Ok(quote_spanned!(expr.span()=> #expr))
        });
        let mut guard = Arg::ident("guard");

        args::set_from_field_values(
            input.parse_terminated(FieldValue::parse, Token![,])?.iter(),
            [
                &mut module,
                &mut guard,
                &mut rt,
                &mut when,
                &mut ok_lvl,
                &mut err_lvl,
            ],
        )?;

        Ok(Args {
            rt: rt.take_rt_ref()?,
            module: module.take().unwrap_or_else(|| module_tokens()),
            when: when.take_some_or_empty_ref(),
            ok_lvl: ok_lvl.take_if_std()?,
            err_lvl: err_lvl.take_if_std()?,
            guard: guard.take(),
        })
    }
}

pub fn expand_tokens(opts: ExpandTokens) -> Result<TokenStream, syn::Error> {
    let span = opts.input.span();

    let (args, template, ctxt_props) = template::parse2::<Args>(opts.input, true)?;

    let template =
        template.ok_or_else(|| syn::Error::new(span, "missing template string literal"))?;

    let span_guard = args
        .guard
        .unwrap_or_else(|| Ident::new("__span", Span::call_site()));

    let module_tokens = args.module;

    let base_lvl = opts.level;
    let ok_lvl = args.ok_lvl;
    let err_lvl = args.err_lvl;

    let evt_props = Props::new();

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
                evt_props,
                &span_guard,
                quote!(#block),
                base_lvl,
                ok_lvl,
                err_lvl,
            )?)?;
        }
        // A synchronous block
        Stmt::Expr(Expr::Block(ExprBlock { block, .. }), _) => {
            *block = syn::parse2::<Block>(inject_sync(
                &args.rt,
                &module_tokens,
                &args.when,
                &template,
                &ctxt_props,
                evt_props,
                &span_guard,
                quote!(#block),
                base_lvl,
                ok_lvl,
                err_lvl,
            )?)?;
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
                evt_props,
                &span_guard,
                quote!(#block),
                base_lvl,
                ok_lvl,
                err_lvl,
            )?)?;
        }
        // An asynchronous block
        Stmt::Expr(Expr::Async(ExprAsync { block, .. }), _) => {
            *block = syn::parse2::<Block>(inject_async(
                &args.rt,
                &module_tokens,
                &args.when,
                &template,
                &ctxt_props,
                evt_props,
                &span_guard,
                quote!(#block),
                base_lvl,
                ok_lvl,
                err_lvl,
            )?)?;
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
    evt_props: Props,
    span_guard: &Ident,
    body: TokenStream,
    base_lvl: Option<TokenStream>,
    ok_lvl: Option<TokenStream>,
    err_lvl: Option<TokenStream>,
) -> Result<TokenStream, syn::Error> {
    let ctxt_props_tokens = ctxt_props.props_tokens();
    let template_tokens = template.template_tokens();
    let template_literal_tokens = template.template_literal_tokens();

    let Completion {
        body_tokens,
        evt_props_tokens,
        completion_tokens,
    } = completion(
        evt_props,
        body,
        base_lvl,
        ok_lvl,
        err_lvl,
        span_guard,
        rt_tokens,
        &template_tokens,
    )?;

    Ok(quote!({
        let (mut __ctxt, __span_guard) = emit::__private::__private_begin_span(
            #rt_tokens,
            #module_tokens,
            #when_tokens,
            #template_tokens,
            #ctxt_props_tokens,
            #evt_props_tokens,
            #template_literal_tokens,
            #completion_tokens,
        );
        let __ctxt_guard = __ctxt.enter();

        let #span_guard = __span_guard;

        #body_tokens
    }))
}

fn inject_async(
    rt_tokens: &TokenStream,
    module_tokens: &TokenStream,
    when_tokens: &TokenStream,
    template: &Template,
    ctxt_props: &Props,
    evt_props: Props,
    span_guard: &Ident,
    body: TokenStream,
    base_lvl: Option<TokenStream>,
    ok_lvl: Option<TokenStream>,
    err_lvl: Option<TokenStream>,
) -> Result<TokenStream, syn::Error> {
    let ctxt_props_tokens = ctxt_props.props_tokens();
    let template_tokens = template.template_tokens();
    let template_literal_tokens = template.template_literal_tokens();

    let Completion {
        body_tokens,
        evt_props_tokens,
        completion_tokens,
    } = completion(
        evt_props,
        quote!(async #body.await),
        base_lvl,
        ok_lvl,
        err_lvl,
        span_guard,
        rt_tokens,
        &template_tokens,
    )?;

    Ok(quote!({
        let (__ctxt, __span_guard) = emit::__private::__private_begin_span(
            #rt_tokens,
            #module_tokens,
            #when_tokens,
            #template_tokens,
            #ctxt_props_tokens,
            #evt_props_tokens,
            #template_literal_tokens,
            #completion_tokens,
        );

        __ctxt.in_future(async move {
            let #span_guard = __span_guard;

            #body_tokens
        }).await
    }))
}

struct Completion {
    body_tokens: TokenStream,
    evt_props_tokens: TokenStream,
    completion_tokens: TokenStream,
}

fn completion(
    mut evt_props: Props,
    body: TokenStream,
    base_lvl: Option<TokenStream>,
    ok_lvl: Option<TokenStream>,
    err_lvl: Option<TokenStream>,
    span_guard: &Ident,
    rt_tokens: &TokenStream,
    template_tokens: &TokenStream,
) -> Result<Completion, syn::Error> {
    let body_tokens = if ok_lvl.is_some() || err_lvl.is_some() {
        // Get the event tokens _before_ pushing the default level
        let evt_props_tokens = evt_props.props_tokens();

        let ok_branch = ok_lvl
            .map(|lvl| {
                // If a level is provided then complete using it
                quote!(
                    Ok(ok) => {
                        #span_guard.complete_with(|span| {
                            emit::__private::__private_complete_span_ok(
                                #rt_tokens,
                                span,
                                #template_tokens,
                                #evt_props_tokens,
                                &#lvl,
                            )
                        });

                        Ok(ok)
                    }
                )
            })
            .unwrap_or_else(|| {
                // Fall-through to the default completion
                quote!(
                    Ok(ok) => Ok(ok)
                )
            });

        let err_branch = err_lvl
            .map(|lvl| {
                // If a level is provided then complete using it
                quote!(
                    Err(err) => {
                        #span_guard.complete_with(|span| {
                            emit::__private::__private_complete_span_err(
                                #rt_tokens,
                                span,
                                #template_tokens,
                                #evt_props_tokens,
                                &#lvl,
                                &err,
                            )
                        });

                        Err(err)
                    }
                )
            })
            .unwrap_or_else(|| {
                // Fall-through to the default completion
                quote!(
                    Err(err) => Err(err)
                )
            });

        quote!(
            match #body {
                #ok_branch,
                #err_branch,
            }
        )
    } else {
        body
    };

    push_evt_props(&mut evt_props, base_lvl)?;
    let evt_props_tokens = evt_props.props_tokens();

    let completion_tokens = quote!(|span| {
        emit::__private::__private_complete_span(
            #rt_tokens,
            span,
            #template_tokens,
            #evt_props_tokens,
        )
    });

    Ok(Completion {
        body_tokens,
        evt_props_tokens,
        completion_tokens,
    })
}
