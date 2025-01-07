use proc_macro2::{Ident, Span, TokenStream};
use syn::{
    parse::Parse, spanned::Spanned, Block, Expr, ExprAsync, ExprBlock, FieldValue, Item, ItemFn,
    Signature, Stmt,
};

use crate::{
    args::{self, Arg},
    capture,
    props::{check_evt_props, Props},
    template::{self, Template},
    util::{ToOptionTokens, ToRefTokens},
};

pub struct ExpandTokens {
    pub level: Option<TokenStream>,
    pub item: TokenStream,
    pub input: TokenStream,
}

struct Args {
    /*
    NOTE: Also update docs in _Control Parameters_ for this macro when adding new args
    */
    rt: args::RtArg,
    mdl: args::MdlArg,
    when: args::WhenArg,
    guard: Option<Ident>,
    setup: Option<TokenStream>,
    ok_lvl: Option<TokenStream>,
    err_lvl: Option<TokenStream>,
    panic_lvl: Option<TokenStream>,
    err: Option<TokenStream>,
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
        let mut panic_lvl = Arg::token_stream("panic_lvl", |fv| {
            let expr = &fv.expr;

            Ok(quote_spanned!(expr.span()=> #expr))
        });
        let mut err = Arg::token_stream("err", |fv| {
            let expr = &fv.expr;

            Ok(quote_spanned!(expr.span()=> #expr))
        });
        let mut setup = Arg::token_stream("setup", |fv| {
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
                &mut panic_lvl,
                &mut setup,
                &mut err,
            ],
        )?;

        if let Some(guard) = guard.peek() {
            if ok_lvl.peek().is_some() || err_lvl.peek().is_some() || err.peek().is_some() {
                return Err(syn::Error::new(guard.span(), "the `guard` control parameter is incompatible with `ok_lvl`, `err_lvl`, `panic_lvl`, or `err`"));
            }
        }

        Ok(Args {
            rt: rt.take_or_default(),
            mdl: mdl.take_or_default(),
            when: when.take_or_default(),
            ok_lvl: ok_lvl.take_if_std()?,
            err_lvl: err_lvl.take_if_std()?,
            panic_lvl: panic_lvl.take_if_std()?,
            err: err.take_if_std()?,
            setup: setup.take(),
            guard: guard.take(),
        })
    }
}

pub fn expand_tokens(opts: ExpandTokens) -> Result<TokenStream, syn::Error> {
    let span = opts.input.span();

    let (args, template, ctxt_props) =
        template::parse2::<Args>(opts.input, capture::default_fn_name, true)?;

    let template =
        template.ok_or_else(|| syn::Error::new(span, "missing template string literal"))?;

    check_evt_props(&ctxt_props)?;

    let span_guard = args
        .guard
        .unwrap_or_else(|| Ident::new("__span", Span::call_site()));

    let default_lvl_tokens = opts.level;
    let panic_lvl_tokens = args.panic_lvl;
    let ok_lvl_tokens = args.ok_lvl;
    let err_lvl_tokens = args.err_lvl;
    let err_tokens = args.err;
    let setup_tokens = args.setup;

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
            sig: Signature {
                asyncness: None, ..
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
                setup_tokens,
                quote!(#block),
                default_lvl_tokens,
                panic_lvl_tokens,
                ok_lvl_tokens,
                err_lvl_tokens,
                err_tokens,
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
                setup_tokens,
                quote!(#block),
                default_lvl_tokens,
                panic_lvl_tokens,
                ok_lvl_tokens,
                err_lvl_tokens,
                err_tokens,
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
                &rt_tokens,
                &mdl_tokens,
                &when_tokens,
                &template,
                &ctxt_props,
                &span_guard,
                setup_tokens,
                quote!(#block),
                default_lvl_tokens,
                panic_lvl_tokens,
                ok_lvl_tokens,
                err_lvl_tokens,
                err_tokens,
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
                setup_tokens,
                quote!(#block),
                default_lvl_tokens,
                panic_lvl_tokens,
                ok_lvl_tokens,
                err_lvl_tokens,
                err_tokens,
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
    setup_tokens: Option<TokenStream>,
    body_tokens: TokenStream,
    default_lvl_tokens: Option<TokenStream>,
    panic_lvl_tokens: Option<TokenStream>,
    ok_lvl_tokens: Option<TokenStream>,
    err_lvl_tokens: Option<TokenStream>,
    err_tokens: Option<TokenStream>,
) -> Result<TokenStream, syn::Error> {
    let ctxt_props_tokens = ctxt_props.props_tokens();
    let template_tokens = template.template_tokens().to_ref_tokens();
    let span_name_tokens = template.template_literal_tokens();

    let Completion {
        body_tokens,
        default_lvl_tokens,
        default_completion_tokens,
    } = if use_result_completion(&ok_lvl_tokens, &err_lvl_tokens, &err_tokens) {
        // Wrap the body in a closure so we can rely on code running afterwards
        // without control flow statements like `return` getting in the way
        //
        // We can't use a drop guard here because we need to match on the result
        //
        // We also need to specify the return type, otherwise inference seems to fail.
        // We might be able to avoid this in the future
        let body_tokens = quote!((move || {
            #body_tokens
        })());

        result_completion(
            body_tokens,
            rt_tokens,
            &template_tokens,
            span_guard,
            default_lvl_tokens,
            panic_lvl_tokens,
            ok_lvl_tokens,
            err_lvl_tokens,
            err_tokens,
        )?
    } else {
        completion(
            body_tokens,
            rt_tokens,
            &template_tokens,
            default_lvl_tokens,
            panic_lvl_tokens,
        )?
    };

    let setup_tokens = setup_tokens.map(|setup| quote!(let __setup = (#setup)();));

    Ok(quote!({
        #setup_tokens

        let (__span_guard, __ctxt) = emit::__private::__private_begin_span(
            #rt_tokens,
            #mdl_tokens,
            #span_name_tokens,
            #default_lvl_tokens,
            #when_tokens,
            #ctxt_props_tokens,
            #default_completion_tokens,
        );

        __ctxt.call(move || {
            let #span_guard = __span_guard;

            #body_tokens
        })
    }))
}

fn inject_async(
    rt_tokens: &TokenStream,
    mdl_tokens: &TokenStream,
    when_tokens: &TokenStream,
    template: &Template,
    ctxt_props: &Props,
    span_guard: &Ident,
    setup_tokens: Option<TokenStream>,
    body_tokens: TokenStream,
    default_lvl_tokens: Option<TokenStream>,
    panic_lvl_tokens: Option<TokenStream>,
    ok_lvl_tokens: Option<TokenStream>,
    err_lvl_tokens: Option<TokenStream>,
    err_tokens: Option<TokenStream>,
) -> Result<TokenStream, syn::Error> {
    let ctxt_props_tokens = ctxt_props.props_tokens();
    let template_tokens = template.template_tokens().to_ref_tokens();
    let span_name_tokens = template.template_literal_tokens();

    let Completion {
        body_tokens,
        default_lvl_tokens,
        default_completion_tokens,
    } = if use_result_completion(&ok_lvl_tokens, &err_lvl_tokens, &err_tokens) {
        // Like the sync case, ensure control flow doesn't interrupt
        // our matching of the result, and provide a concrete type
        // for inference
        let body_tokens = quote!(async move {
            #body_tokens
        }.await);

        result_completion(
            body_tokens,
            rt_tokens,
            &template_tokens,
            span_guard,
            default_lvl_tokens,
            panic_lvl_tokens,
            ok_lvl_tokens,
            err_lvl_tokens,
            err_tokens,
        )?
    } else {
        completion(
            body_tokens,
            rt_tokens,
            &template_tokens,
            default_lvl_tokens,
            panic_lvl_tokens,
        )?
    };

    let setup_tokens = setup_tokens.map(|setup| quote!(let __setup = (#setup)();));

    Ok(quote!({
        #setup_tokens

        let (mut __span_guard, __ctxt) = emit::__private::__private_begin_span(
            #rt_tokens,
            #mdl_tokens,
            #span_name_tokens,
            #default_lvl_tokens,
            #when_tokens,
            #ctxt_props_tokens,
            #default_completion_tokens,
        );

        __ctxt.in_future(async move {
            let #span_guard = __span_guard;
            #body_tokens
        }).await
    }))
}

struct Completion {
    body_tokens: TokenStream,
    default_lvl_tokens: TokenStream,
    default_completion_tokens: TokenStream,
}

fn use_result_completion(
    ok_lvl_tokens: &Option<TokenStream>,
    err_lvl_tokens: &Option<TokenStream>,
    err_tokens: &Option<TokenStream>,
) -> bool {
    ok_lvl_tokens.is_some() || err_lvl_tokens.is_some() || err_tokens.is_some()
}

fn result_completion(
    body_tokens: TokenStream,
    rt_tokens: &TokenStream,
    template_tokens: &TokenStream,
    span_guard: &Ident,
    default_lvl_tokens: Option<TokenStream>,
    panic_lvl_tokens: Option<TokenStream>,
    ok_lvl_tokens: Option<TokenStream>,
    err_lvl_tokens: Option<TokenStream>,
    err_tokens: Option<TokenStream>,
) -> Result<Completion, syn::Error> {
    // If the span is applied to a Result-returning function then wrap the body
    // We'll attach the error to the span if the call fails and set the appropriate level

    let ok_branch = {
        let lvl_tokens = ok_lvl_tokens
            .map(|lvl| lvl.to_ref_tokens())
            .or_else(|| default_lvl_tokens.as_ref().map(|lvl| lvl.to_ref_tokens()))
            .to_option_tokens(quote!(&emit::Level));

        quote!(
            Ok(__ok) => {
                #span_guard.complete_with(emit::span::completion::from_fn(|span| {
                    emit::__private::__private_complete_span_ok(
                        #rt_tokens,
                        span,
                        #template_tokens,
                        #lvl_tokens,
                    )
                }));

                Ok(__ok)
            }
        )
    };

    let err_branch = {
        // In the error case, we don't just defer to the default level
        // If none is set then we'll mark it as an error
        let lvl_tokens = err_lvl_tokens
            .map(|lvl| lvl.to_ref_tokens())
            .or_else(|| default_lvl_tokens.as_ref().map(|lvl| lvl.to_ref_tokens()))
            .unwrap_or_else(|| {
                let err_lvl = emit_core::well_known::LVL_ERROR;

                quote!(#err_lvl)
            });

        let err_tokens = err_tokens
            .map(|mapper| quote!((#mapper)(&__err)))
            .unwrap_or_else(|| quote!(&__err));

        quote!(
            Err(__err) => {
                #span_guard.complete_with(emit::span::completion::from_fn(|span| {
                    emit::__private::__private_complete_span_err(
                        #rt_tokens,
                        span,
                        #template_tokens,
                        #lvl_tokens,
                        #err_tokens,
                    )
                }));

                Err(__err)
            }
        )
    };

    let body_tokens = quote!(match #body_tokens {
        #ok_branch,
        #err_branch,
    });

    completion(
        body_tokens,
        rt_tokens,
        template_tokens,
        default_lvl_tokens,
        panic_lvl_tokens,
    )
}

fn completion(
    body_tokens: TokenStream,
    rt_tokens: &TokenStream,
    template_tokens: &TokenStream,
    default_lvl_tokens: Option<TokenStream>,
    panic_lvl_tokens: Option<TokenStream>,
) -> Result<Completion, syn::Error> {
    let lvl_tokens = default_lvl_tokens
        .map(|lvl| lvl.to_ref_tokens())
        .to_option_tokens(quote!(&emit::Level));

    let panic_lvl_tokens = panic_lvl_tokens
        .map(|lvl| lvl.to_ref_tokens())
        .to_option_tokens(quote!(&emit::Level));

    let default_completion_tokens = quote!(emit::span::completion::from_fn(|span| {
        emit::__private::__private_complete_span(
            #rt_tokens,
            span,
            #template_tokens,
            #lvl_tokens,
            #panic_lvl_tokens,
        )
    }));

    Ok(Completion {
        body_tokens,
        default_lvl_tokens: lvl_tokens,
        default_completion_tokens,
    })
}
