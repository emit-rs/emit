use proc_macro2::{Ident, Span, TokenStream};
use syn::{
    parse::Parse, spanned::Spanned, Block, Expr, ExprAsync, ExprBlock, FieldValue, Item, ItemFn,
    ReturnType, Signature, Stmt,
};

use crate::{
    args::{self, Arg},
    props::push_evt_props,
    props::Props,
    template::{self, Template},
    util::{ToOptionTokens, ToRefTokens},
};

pub struct ExpandTokens {
    pub level: Option<TokenStream>,
    pub item: TokenStream,
    pub input: TokenStream,
}

struct Args {
    rt: args::RtArg,
    mdl: args::MdlArg,
    when: args::WhenArg,
    guard: Option<Ident>,
    ok_lvl: Option<TokenStream>,
    err_lvl: Option<TokenStream>,
}

impl Parse for Args {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut rt = Arg::new("rt", |fv| {
            let expr = &fv.expr;

            Ok(args::RtArg::new(quote_spanned!(expr.span()=> #expr)))
        });
        let mut mdl = Arg::new("mdl", |fv| {
            let expr = &fv.expr;

            Ok(args::MdlArg::new(quote_spanned!(expr.span()=> #expr)))
        });
        let mut when = Arg::new("when", |fv| {
            let expr = &fv.expr;

            Ok(args::WhenArg::new(quote_spanned!(expr.span()=> #expr)))
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
                &mut mdl,
                &mut guard,
                &mut rt,
                &mut when,
                &mut ok_lvl,
                &mut err_lvl,
            ],
        )?;

        Ok(Args {
            rt: rt.take_or_default(),
            mdl: mdl.take_or_default(),
            when: when.take_or_default(),
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

    let default_lvl_tokens = opts.level;
    let ok_lvl_tokens = args.ok_lvl;
    let err_lvl_tokens = args.err_lvl;

    let rt_tokens = args.rt.to_tokens()?.to_ref_tokens();
    let mdl_tokens = args.mdl.to_tokens();
    let when_tokens = args
        .when
        .to_tokens()
        .map(|when| when.to_ref_tokens())
        .to_option_tokens(quote!(&emit::Empty));

    let mut item = syn::parse2::<Stmt>(opts.item)?;
    match &mut item {
        // A synchronous function
        Stmt::Item(Item::Fn(ItemFn {
            block,
            sig:
                Signature {
                    asyncness: None,
                    output,
                    ..
                },
            ..
        })) => {
            **block = syn::parse2::<Block>(inject_sync(
                &rt_tokens,
                &mdl_tokens,
                &when_tokens,
                &template,
                &ctxt_props,
                &span_guard,
                quote!(#block),
                body_ret_ty_tokens(output),
                default_lvl_tokens,
                ok_lvl_tokens,
                err_lvl_tokens,
            )?)?;
        }
        // A synchronous block
        Stmt::Expr(Expr::Block(ExprBlock { block, .. }), _) => {
            *block = syn::parse2::<Block>(inject_sync(
                &rt_tokens,
                &mdl_tokens,
                &when_tokens,
                &template,
                &ctxt_props,
                &span_guard,
                quote!(#block),
                body_ret_ty_tokens(&ReturnType::Default),
                default_lvl_tokens,
                ok_lvl_tokens,
                err_lvl_tokens,
            )?)?;
        }
        // An asynchronous function
        Stmt::Item(Item::Fn(ItemFn {
            block,
            sig:
                Signature {
                    asyncness: Some(_),
                    output,
                    ..
                },
            ..
        })) => {
            **block = syn::parse2::<Block>(inject_async(
                &rt_tokens,
                &mdl_tokens,
                &when_tokens,
                &template,
                &ctxt_props,
                &span_guard,
                quote!(#block),
                body_ret_ty_tokens(output),
                default_lvl_tokens,
                ok_lvl_tokens,
                err_lvl_tokens,
            )?)?;
        }
        // An asynchronous block
        Stmt::Expr(Expr::Async(ExprAsync { block, .. }), _) => {
            *block = syn::parse2::<Block>(inject_async(
                &rt_tokens,
                &mdl_tokens,
                &when_tokens,
                &template,
                &ctxt_props,
                &span_guard,
                quote!(#block),
                body_ret_ty_tokens(&ReturnType::Default),
                default_lvl_tokens,
                ok_lvl_tokens,
                err_lvl_tokens,
            )?)?;
        }
        _ => return Err(syn::Error::new(item.span(), "unrecognized item type")),
    }

    Ok(quote!(#item))
}

fn inject_sync(
    rt_tokens: &TokenStream,
    mdl_tokens: &TokenStream,
    when_tokens: &TokenStream,
    template: &Template,
    ctxt_props: &Props,
    span_guard: &Ident,
    body_tokens: TokenStream,
    body_ret_ty_tokens: TokenStream,
    default_lvl_tokens: Option<TokenStream>,
    ok_lvl_tokens: Option<TokenStream>,
    err_lvl_tokens: Option<TokenStream>,
) -> Result<TokenStream, syn::Error> {
    let ctxt_props_tokens = ctxt_props.props_tokens();
    let template_tokens = template.template_tokens().to_ref_tokens();
    let template_literal_tokens = template.template_literal_tokens();

    // Capture control flow statements within the body
    let body_tokens = quote!((|| {
        let __r: #body_ret_ty_tokens = #body_tokens;
        __r
    })());

    let Completion {
        body_tokens,
        span_evt_props_tokens,
        default_completion_tokens,
    } = completion(
        body_tokens,
        default_lvl_tokens,
        ok_lvl_tokens,
        err_lvl_tokens,
        span_guard,
        rt_tokens,
        &template_tokens,
    )?;

    Ok(quote!({
        let (mut __ctxt, __span_guard) = emit::__private::__private_begin_span(
            #rt_tokens,
            #mdl_tokens,
            #template_literal_tokens,
            #template_tokens,
            #when_tokens,
            #ctxt_props_tokens,
            #span_evt_props_tokens,
            #default_completion_tokens,
        );
        let __ctxt_guard = __ctxt.enter();

        let #span_guard = __span_guard;

        #body_tokens
    }))
}

fn inject_async(
    rt_tokens: &TokenStream,
    mdl_tokens: &TokenStream,
    when_tokens: &TokenStream,
    template: &Template,
    ctxt_props: &Props,
    span_guard: &Ident,
    body_tokens: TokenStream,
    body_ret_ty_tokens: TokenStream,
    default_lvl_tokens: Option<TokenStream>,
    ok_lvl_tokens: Option<TokenStream>,
    err_lvl_tokens: Option<TokenStream>,
) -> Result<TokenStream, syn::Error> {
    let ctxt_props_tokens = ctxt_props.props_tokens();
    let template_tokens = template.template_tokens().to_ref_tokens();
    let template_literal_tokens = template.template_literal_tokens();

    let body_tokens = quote!(async {
        let __r: #body_ret_ty_tokens = #body_tokens;
        __r
    }.await);

    let Completion {
        body_tokens,
        span_evt_props_tokens,
        default_completion_tokens,
    } = completion(
        body_tokens,
        default_lvl_tokens,
        ok_lvl_tokens,
        err_lvl_tokens,
        span_guard,
        rt_tokens,
        &template_tokens,
    )?;

    Ok(quote!({
        let (__ctxt, __span_guard) = emit::__private::__private_begin_span(
            #rt_tokens,
            #mdl_tokens,
            #template_literal_tokens,
            #template_tokens,
            #when_tokens,
            #ctxt_props_tokens,
            #span_evt_props_tokens,
            #default_completion_tokens,
        );

        __ctxt.in_future(async move {
            let #span_guard = __span_guard;

            #body_tokens
        }).await
    }))
}

fn body_ret_ty_tokens(output: &ReturnType) -> TokenStream {
    match output {
        ReturnType::Type(_, ty) => quote!(#ty),
        _ => quote!(_),
    }
}

struct Completion {
    body_tokens: TokenStream,
    span_evt_props_tokens: TokenStream,
    default_completion_tokens: TokenStream,
}

fn completion(
    body_tokens: TokenStream,
    default_lvl_tokens: Option<TokenStream>,
    ok_lvl_tokens: Option<TokenStream>,
    err_lvl_tokens: Option<TokenStream>,
    span_guard: &Ident,
    rt_tokens: &TokenStream,
    template_tokens: &TokenStream,
) -> Result<Completion, syn::Error> {
    let body_tokens = if ok_lvl_tokens.is_some() || err_lvl_tokens.is_some() {
        // If the span is applied to a Result-returning function then wrap the body
        // We'll attach the error to the span if the call fails and set the appropriate level

        let ok_branch = {
            let mut evt_props = Props::new();
            push_evt_props(
                &mut evt_props,
                ok_lvl_tokens.or_else(|| default_lvl_tokens.clone()),
            )?;
            let span_evt_props_tokens = evt_props.props_tokens();

            quote!(
                Ok(ok) => {
                    #span_guard.complete_with(|span| {
                        emit::__private::__private_complete_span(
                            #rt_tokens,
                            span,
                            #template_tokens,
                            #span_evt_props_tokens,
                        )
                    });

                    Ok(())
                }
            )
        };

        let err_branch = {
            let err_ident = Ident::new(emit_core::well_known::KEY_ERR, Span::call_site());

            let mut evt_props = Props::new();
            push_evt_props(
                &mut evt_props,
                err_lvl_tokens.or_else(|| default_lvl_tokens.clone()),
            )?;
            evt_props.push(&syn::parse2::<FieldValue>(quote!(#err_ident))?, false, true)?;
            let span_evt_props_tokens = evt_props.props_tokens();

            quote!(
                Err(#err_ident) => {
                    #span_guard.complete_with(|span| {
                        emit::__private::__private_complete_span(
                            #rt_tokens,
                            span,
                            #template_tokens,
                            #span_evt_props_tokens,
                        )
                    });

                    Err(#err_ident)
                }
            )
        };

        quote!(match #body_tokens {
            #ok_branch,
            #err_branch,
        })
    } else {
        body_tokens
    };

    let mut evt_props = Props::new();
    push_evt_props(&mut evt_props, default_lvl_tokens)?;
    let span_evt_props_tokens = evt_props.props_tokens();

    let default_completion_tokens = quote!(|span| {
        emit::__private::__private_complete_span(
            #rt_tokens,
            span,
            #template_tokens,
            #span_evt_props_tokens,
        )
    });

    Ok(Completion {
        body_tokens,
        span_evt_props_tokens,
        default_completion_tokens,
    })
}
