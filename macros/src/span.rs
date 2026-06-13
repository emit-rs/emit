use proc_macro2::{Ident, Span, TokenStream};
use syn::{
    Block, Expr, ExprAsync, ExprBlock, FieldValue, Item, ItemFn, Member, Signature, Stmt,
    parse::Parse, spanned::Spanned,
};

use crate::util::StmtFnName;
use crate::{
    args::{self, Arg},
    capture,
    props::{Props, check_evt_props, check_span_props},
    template::{self, Template},
    util::{ToOptionTokens, ToRefTokens},
};

use emit_core::well_known::KEY_SPAN_NAME;

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
    evt_props: Option<TokenStream>,
    fn_name: Option<Ident>,
    name: Option<TokenStream>,
    catch_unwind: Option<bool>,
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
        let mut evt_props = Arg::token_stream("evt_props", |fv| {
            let expr = &fv.expr;

            Ok(quote_spanned!(expr.span()=> #expr))
        });
        let mut fn_name = Arg::ident("fn_name");
        let mut name = Arg::token_stream("name", |fv| {
            let expr = &fv.expr;

            Ok(quote_spanned!(expr.span()=> #expr))
        });
        let mut catch_unwind = Arg::bool("catch_unwind");

        args::set_from_field_values(
            input.parse_terminated(FieldValue::parse, Token![,])?.iter(),
            [
                &mut mdl,
                &mut guard,
                &mut evt_props,
                &mut fn_name,
                &mut name,
                &mut rt,
                &mut when,
                &mut ok_lvl,
                &mut err_lvl,
                &mut panic_lvl,
                &mut catch_unwind,
                &mut setup,
                &mut err,
            ],
        )?;

        Ok(Args {
            rt: rt.take_or_default(),
            mdl: mdl.take_or_default(),
            when: when.take_or_default(),
            ok_lvl: ok_lvl.take_if_std()?,
            err_lvl: err_lvl.take_if_std()?,
            panic_lvl: panic_lvl.take_if_std()?,
            catch_unwind: catch_unwind.take(),
            err: err.take_if_std()?,
            setup: setup.take(),
            guard: guard.take(),
            evt_props: evt_props.take(),
            fn_name: fn_name.take(),
            name: name.take(),
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
    check_span_props(&ctxt_props)?;

    let mut macro_evt_props = Props::new();

    let span_guard = args.guard;

    let default_lvl_tokens = opts.level;
    let panic_lvl_tokens = args.panic_lvl;
    let ok_lvl_tokens = args.ok_lvl;
    let err_lvl_tokens = args.err_lvl;
    let err_tokens = args.err;
    let catch_unwind = args.catch_unwind.unwrap_or_default();
    let setup_tokens = args.setup;

    let user_evt_props_tokens = args.evt_props.unwrap_or_else(|| quote!(emit::Empty));

    let rt_tokens = args.rt.to_tokens()?.to_ref_tokens();
    let mdl_tokens = args.mdl.to_tokens();
    let when_tokens = args
        .when
        .to_tokens()
        .map(|when| when.to_ref_tokens())
        .to_option_tokens(quote!(&emit::Empty));

    let mut item = syn::parse2::<Stmt>(opts.item)?;

    let fn_name_tokens =
        if let Some(fn_name) = fn_name(args.fn_name.as_ref(), item.fn_name().as_ref())? {
            let fn_name_prop = fn_name.to_prop();

            macro_evt_props.push(
                &fn_name_prop,
                capture::default_fn_name(&fn_name_prop),
                false,
                true,
            )?;

            Some(fn_name.binding_tokens())
        } else {
            None
        };

    let span_name_prop = span_name_prop(args.name, &template);
    macro_evt_props.push(
        &span_name_prop,
        capture::default_fn_name(&span_name_prop),
        false,
        true,
    )?;

    check_evt_props(&macro_evt_props)?;
    check_span_props(&macro_evt_props)?;

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
                &user_evt_props_tokens,
                &macro_evt_props,
                fn_name_tokens,
                setup_tokens,
                span_guard,
                quote!(#block),
                default_lvl_tokens,
                panic_lvl_tokens,
                ok_lvl_tokens,
                err_lvl_tokens,
                err_tokens,
                catch_unwind,
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
                &user_evt_props_tokens,
                &macro_evt_props,
                fn_name_tokens,
                setup_tokens,
                span_guard,
                quote!(#block),
                default_lvl_tokens,
                panic_lvl_tokens,
                ok_lvl_tokens,
                err_lvl_tokens,
                err_tokens,
                catch_unwind,
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
                &user_evt_props_tokens,
                &macro_evt_props,
                fn_name_tokens,
                setup_tokens,
                span_guard,
                quote!(#block),
                default_lvl_tokens,
                panic_lvl_tokens,
                ok_lvl_tokens,
                err_lvl_tokens,
                err_tokens,
                catch_unwind,
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
                &user_evt_props_tokens,
                &macro_evt_props,
                fn_name_tokens,
                setup_tokens,
                span_guard,
                quote!(#block),
                default_lvl_tokens,
                panic_lvl_tokens,
                ok_lvl_tokens,
                err_lvl_tokens,
                err_tokens,
                catch_unwind,
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
    user_evt_props_tokens: &TokenStream,
    macro_evt_props: &Props,
    fn_name_tokens: Option<TokenStream>,
    setup_tokens: Option<TokenStream>,
    span_guard: Option<Ident>,
    body_tokens: TokenStream,
    default_lvl_tokens: Option<TokenStream>,
    panic_lvl_tokens: Option<TokenStream>,
    ok_lvl_tokens: Option<TokenStream>,
    err_lvl_tokens: Option<TokenStream>,
    err_tokens: Option<TokenStream>,
    catch_unwind: bool,
) -> Result<TokenStream, syn::Error> {
    let template_tokens = template.template_tokens();

    let SpanGuardBinding {
        span_guard_binding_tokens,
        span_guard_initial_ident,
        span_guard_completion_ident,
    } = span_guard_binding(
        span_guard,
        &ok_lvl_tokens,
        &err_lvl_tokens,
        &err_tokens,
        catch_unwind,
    );

    let body_tokens = if catch_unwind {
        quote!(emit::__private::__private_catch_unwind(move || { #body_tokens }))
    } else {
        body_tokens
    };

    let Completion {
        body_tokens,
        default_lvl_tokens,
        default_completion_tokens,
    } = if use_result_completion(&ok_lvl_tokens, &err_lvl_tokens, &err_tokens) {
        // Wrap the body in a closure so we can rely on code running afterwards
        // without control flow statements like `return` getting in the way
        //
        // We can't use a drop guard here because we need to match on the result
        let body_tokens = if catch_unwind {
            // We've already wrapped the body in a closure so don't
            // need to do it again
            body_tokens
        } else {
            quote!((move || {
                #body_tokens
            })())
        };

        result_completion(
            body_tokens,
            rt_tokens,
            &template_tokens,
            span_guard_completion_ident,
            default_lvl_tokens,
            panic_lvl_tokens,
            ok_lvl_tokens,
            err_lvl_tokens,
            err_tokens,
            catch_unwind,
        )?
    } else {
        completion(
            body_tokens,
            rt_tokens,
            &template_tokens,
            span_guard_completion_ident,
            default_lvl_tokens,
            panic_lvl_tokens,
            catch_unwind,
        )?
    };

    let setup_tokens = setup_tokens.map(|setup| quote!(let __setup = (#setup)();));

    let span_guard_tokens = span_guard_tokens(
        rt_tokens,
        mdl_tokens,
        when_tokens,
        ctxt_props,
        user_evt_props_tokens,
        macro_evt_props,
        &default_completion_tokens,
        &default_lvl_tokens,
    )?;

    Ok(quote!({
        #fn_name_tokens
        #setup_tokens

        let (mut #span_guard_initial_ident, __ctxt) = #span_guard_tokens;

        __ctxt.call(move || {
            #span_guard_initial_ident.start();
            #span_guard_binding_tokens

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
    user_evt_props_tokens: &TokenStream,
    macro_evt_props: &Props,
    fn_name_tokens: Option<TokenStream>,
    setup_tokens: Option<TokenStream>,
    span_guard: Option<Ident>,
    body_tokens: TokenStream,
    default_lvl_tokens: Option<TokenStream>,
    panic_lvl_tokens: Option<TokenStream>,
    ok_lvl_tokens: Option<TokenStream>,
    err_lvl_tokens: Option<TokenStream>,
    err_tokens: Option<TokenStream>,
    catch_unwind: bool,
) -> Result<TokenStream, syn::Error> {
    let template_tokens = template.template_tokens();

    let SpanGuardBinding {
        span_guard_binding_tokens,
        span_guard_initial_ident,
        span_guard_completion_ident,
    } = span_guard_binding(
        span_guard,
        &ok_lvl_tokens,
        &err_lvl_tokens,
        &err_tokens,
        catch_unwind,
    );

    let body_tokens = if catch_unwind {
        quote!(emit::__private::__private_catch_unwind_async(async move {
            #body_tokens
        }).await)
    } else {
        body_tokens
    };

    let Completion {
        body_tokens,
        default_lvl_tokens,
        default_completion_tokens,
    } = if use_result_completion(&ok_lvl_tokens, &err_lvl_tokens, &err_tokens) {
        // Like the sync case, ensure control flow doesn't interrupt
        // our matching of the result
        let body_tokens = if catch_unwind {
            // We've already wrapped the body in an async block so don't need
            // to do it again
            body_tokens
        } else {
            quote!(async move {
                #body_tokens
            }.await)
        };

        result_completion(
            body_tokens,
            rt_tokens,
            &template_tokens,
            span_guard_completion_ident,
            default_lvl_tokens,
            panic_lvl_tokens,
            ok_lvl_tokens,
            err_lvl_tokens,
            err_tokens,
            catch_unwind,
        )?
    } else {
        completion(
            body_tokens,
            rt_tokens,
            &template_tokens,
            span_guard_completion_ident,
            default_lvl_tokens,
            panic_lvl_tokens,
            catch_unwind,
        )?
    };

    let setup_tokens = setup_tokens.map(|setup| quote!(let __setup = (#setup)();));

    let span_guard_tokens = span_guard_tokens(
        rt_tokens,
        mdl_tokens,
        when_tokens,
        ctxt_props,
        user_evt_props_tokens,
        macro_evt_props,
        &default_completion_tokens,
        &default_lvl_tokens,
    )?;

    Ok(quote!({
        #fn_name_tokens
        #setup_tokens

        let (mut #span_guard_initial_ident, __ctxt) = #span_guard_tokens;

        __ctxt.in_future(async move {
            #span_guard_initial_ident.start();
            #span_guard_binding_tokens

            #body_tokens
        }).await
    }))
}

fn span_guard_tokens(
    rt_tokens: &TokenStream,
    mdl_tokens: &TokenStream,
    when_tokens: &TokenStream,
    ctxt_props: &Props,
    user_evt_props_tokens: &TokenStream,
    macro_evt_props: &Props,
    default_completion_tokens: &TokenStream,
    default_lvl_tokens: &TokenStream,
) -> Result<TokenStream, syn::Error> {
    // We use type-preserving props here because they may span across await points
    let macro_evt_props_tokens = macro_evt_props.gen_bound_props_tokens()?;

    let evt_props_tokens = quote!(emit::__private::__PrivateMacroExtendedProps::new(#user_evt_props_tokens, #macro_evt_props_tokens));

    ctxt_props.match_bound_props_tokens(|ctxt_props_tokens| {
        Ok(quote!(emit::__private::__private_begin_span(
            #rt_tokens,
            #mdl_tokens,
            #default_lvl_tokens,
            #when_tokens,
            #ctxt_props_tokens,
            #evt_props_tokens,
            #default_completion_tokens,
        )))
    })
}

struct SpanGuardBinding {
    span_guard_binding_tokens: TokenStream,
    span_guard_initial_ident: Ident,
    span_guard_completion_ident: Ident,
}

fn span_guard_binding(
    user_span_guard: Option<Ident>,
    ok_lvl_tokens: &Option<TokenStream>,
    err_lvl_tokens: &Option<TokenStream>,
    err_tokens: &Option<TokenStream>,
    catch_unwind: bool,
) -> SpanGuardBinding {
    let initial_span_guard = Ident::new("__span_guard", Span::call_site());

    let bind_by_ref =
        catch_unwind || use_result_completion(ok_lvl_tokens, err_lvl_tokens, err_tokens);

    if bind_by_ref {
        if let Some(user_span_guard) = user_span_guard {
            SpanGuardBinding {
                span_guard_binding_tokens: quote!(let mut #user_span_guard = &mut #initial_span_guard;),
                span_guard_initial_ident: initial_span_guard.clone(),
                span_guard_completion_ident: initial_span_guard,
            }
        } else {
            SpanGuardBinding {
                span_guard_binding_tokens: Default::default(),
                span_guard_initial_ident: initial_span_guard.clone(),
                span_guard_completion_ident: initial_span_guard,
            }
        }
    } else {
        if let Some(user_span_guard) = user_span_guard {
            SpanGuardBinding {
                span_guard_binding_tokens: quote!(let mut #user_span_guard = #initial_span_guard;),
                span_guard_initial_ident: initial_span_guard,
                span_guard_completion_ident: user_span_guard,
            }
        } else {
            SpanGuardBinding {
                span_guard_binding_tokens: Default::default(),
                span_guard_initial_ident: initial_span_guard.clone(),
                span_guard_completion_ident: initial_span_guard,
            }
        }
    }
}

struct FnName {
    fn_name_ident: Ident,
    fn_name_value: String,
}

impl FnName {
    fn binding_tokens(&self) -> TokenStream {
        let FnName {
            fn_name_ident,
            fn_name_value,
        } = self;

        quote!(let #fn_name_ident = #fn_name_value;)
    }

    fn to_prop(&self) -> FieldValue {
        let FnName { fn_name_ident, .. } = self;

        let span = fn_name_ident.span();

        FieldValue {
            attrs: vec![],
            // Bind as `x: x` instead of `x: "name"` so `x` doesn't trigger
            // unused warnings. The binding is assigned within the body of the span
            member: Member::Named(fn_name_ident.clone()),
            colon_token: Some(Token![:](span)),
            expr: parse_quote_spanned!(span=> #fn_name_ident),
        }
    }
}

fn fn_name(binding: Option<&Ident>, name: Option<&Ident>) -> Result<Option<FnName>, syn::Error> {
    match (binding, name) {
        (Some(binding), Some(name)) => Ok(Some(FnName {
            fn_name_ident: binding.clone(),
            fn_name_value: name.to_string(),
        })),
        (None, _) => Ok(None),
        (Some(binding), None) => Err(syn::Error::new(
            binding.span(),
            "cannot bind the name of an anonymous function",
        )),
    }
}

fn span_name_prop(name: Option<TokenStream>, template: &Template) -> FieldValue {
    let expr = name.unwrap_or_else(|| template.template_literal_tokens());

    FieldValue {
        attrs: vec![],
        member: Member::Named(Ident::new(KEY_SPAN_NAME, expr.span())),
        colon_token: Some(Token![:](expr.span())),
        expr: parse_quote_spanned!(expr.span()=> #expr),
    }
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
    span_guard: Ident,
    default_lvl_tokens: Option<TokenStream>,
    panic_lvl_tokens: Option<TokenStream>,
    ok_lvl_tokens: Option<TokenStream>,
    err_lvl_tokens: Option<TokenStream>,
    err_tokens: Option<TokenStream>,
    catch_unwind: bool,
) -> Result<Completion, syn::Error> {
    // If the span is applied to a Result-returning function then wrap the body
    // We'll attach the error to the span if the call fails and set the appropriate level

    let ok_branch = {
        let lvl_tokens = optional_lvl_tokens(ok_lvl_tokens.as_ref(), default_lvl_tokens.as_ref());

        quote!({
            #span_guard.complete_with(emit::__private::__private_complete_span_ok(
                #rt_tokens,
                #template_tokens,
                #lvl_tokens,
            ));

            emit::__private::core::result::Result::Ok(__ok)
        })
    };

    let err_branch = {
        // In the error case, we don't just defer to the default level
        // If none is set then we'll mark it as an error
        let lvl_tokens = lvl_tokens(
            err_lvl_tokens.as_ref(),
            default_lvl_tokens.as_ref(),
            emit_core::well_known::LVL_ERROR,
        );

        let err_tokens = err_tokens
            .map(|mapper| quote!((#mapper)(&__err)))
            .unwrap_or_else(|| quote!(&__err));

        quote!({
            #span_guard.complete_with(emit::__private::__private_complete_span_err(
                #rt_tokens,
                #template_tokens,
                #lvl_tokens,
                #err_tokens,
            ));

            emit::__private::core::result::Result::Err(__err)
        })
    };

    let panic_lvl_tokens = lvl_tokens(
        panic_lvl_tokens.as_ref(),
        default_lvl_tokens.as_ref(),
        emit_core::well_known::LVL_ERROR,
    );

    let body_tokens = if catch_unwind {
        quote!(match #body_tokens {
            emit::__private::core::result::Result::Ok(emit::__private::core::result::Result::Ok(__ok)) => #ok_branch,
            emit::__private::core::result::Result::Ok(emit::__private::core::result::Result::Err(__err)) => #err_branch,
            emit::__private::core::result::Result::Err(__panic) => {
                #span_guard.complete_with(emit::__private::__private_complete_span_panic(
                    #rt_tokens,
                    #template_tokens,
                    #panic_lvl_tokens,
                    &__panic,
                ));

                emit::__private::__private_resume_unwind(__panic)
            },
        })
    } else {
        quote!(match #body_tokens {
            emit::__private::core::result::Result::Ok(__ok) => #ok_branch,
            emit::__private::core::result::Result::Err(__err) => #err_branch,
        })
    };

    // Similar to the non-result `completion` variant
    let lvl_tokens = optional_lvl_tokens(default_lvl_tokens.as_ref(), default_lvl_tokens.as_ref());

    let default_completion_tokens = quote!(emit::__private::__private_complete_span(
        #rt_tokens,
        #template_tokens,
        #lvl_tokens,
        #panic_lvl_tokens,
    ));

    Ok(Completion {
        body_tokens,
        default_lvl_tokens: lvl_tokens,
        default_completion_tokens,
    })
}

fn completion(
    body_tokens: TokenStream,
    rt_tokens: &TokenStream,
    template_tokens: &TokenStream,
    span_guard: Ident,
    default_lvl_tokens: Option<TokenStream>,
    panic_lvl_tokens: Option<TokenStream>,
    catch_unwind: bool,
) -> Result<Completion, syn::Error> {
    let panic_lvl_tokens = lvl_tokens(
        panic_lvl_tokens.as_ref(),
        default_lvl_tokens.as_ref(),
        emit_core::well_known::LVL_ERROR,
    );
    let lvl_tokens = optional_lvl_tokens(default_lvl_tokens.as_ref(), default_lvl_tokens.as_ref());

    let default_completion_tokens = quote!(emit::__private::__private_complete_span(
        #rt_tokens,
        #template_tokens,
        #lvl_tokens,
        #panic_lvl_tokens,
    ));

    let body_tokens = if catch_unwind {
        quote!(match #body_tokens {
            emit::__private::core::result::Result::Ok(__r) => __r,
            emit::__private::core::result::Result::Err(__panic) => {
                #span_guard.complete_with(emit::__private::__private_complete_span_panic(
                    #rt_tokens,
                    #template_tokens,
                    #panic_lvl_tokens,
                    &__panic,
                ));

                emit::__private::__private_resume_unwind(__panic)
            },
        })
    } else {
        body_tokens
    };

    Ok(Completion {
        body_tokens,
        default_lvl_tokens: lvl_tokens,
        default_completion_tokens,
    })
}

fn optional_lvl_tokens(
    lvl_tokens: Option<&TokenStream>,
    default_lvl_tokens: Option<&TokenStream>,
) -> TokenStream {
    lvl_tokens
        .map(|lvl| lvl.to_ref_tokens())
        .or_else(|| default_lvl_tokens.as_ref().map(|lvl| lvl.to_ref_tokens()))
        .to_option_tokens(quote!(&emit::Level))
}

fn lvl_tokens(
    lvl_tokens: Option<&TokenStream>,
    default_lvl_tokens: Option<&TokenStream>,
    fallback_value: &str,
) -> TokenStream {
    lvl_tokens
        .map(|lvl| lvl.to_ref_tokens())
        .or_else(|| default_lvl_tokens.as_ref().map(|lvl| lvl.to_ref_tokens()))
        .unwrap_or_else(|| quote!(#fallback_value))
}

pub struct ExpandNewTokens {
    pub level: Option<TokenStream>,
    pub input: TokenStream,
}

/**
The `new_span!` macro.
*/
pub fn expand_new_tokens(opts: ExpandNewTokens) -> Result<TokenStream, syn::Error> {
    let span = opts.input.span();

    let (args, template, ctxt_props) =
        template::parse2::<Args>(opts.input, capture::default_fn_name, true)?;

    let template =
        template.ok_or_else(|| syn::Error::new(span, "missing template string literal"))?;

    check_evt_props(&ctxt_props)?;
    check_span_props(&ctxt_props)?;

    let Args {
        rt,
        mdl,
        when,
        guard,
        evt_props,
        setup,
        fn_name,
        name,
        ok_lvl,
        err_lvl,
        panic_lvl,
        catch_unwind,
        err,
    } = args;

    args::ensure_missing("guard", guard.map(|arg| arg.span()))?;
    args::ensure_missing("setup", setup.map(|arg| arg.span()))?;
    args::ensure_missing("ok_lvl", ok_lvl.map(|arg| arg.span()))?;
    args::ensure_missing("err_lvl", err_lvl.map(|arg| arg.span()))?;
    args::ensure_missing("catch_unwind", catch_unwind.map(|arg| arg.span()))?;
    args::ensure_missing("err", err.map(|arg| arg.span()))?;
    args::ensure_missing("fn_name", fn_name.map(|arg| arg.span()))?;

    let default_lvl_tokens = opts.level;
    let panic_lvl_tokens = panic_lvl;

    let rt_tokens = rt.to_tokens()?.to_ref_tokens();
    let mdl_tokens = mdl.to_tokens();
    let when_tokens = when
        .to_tokens()
        .map(|when| when.to_ref_tokens())
        .to_option_tokens(quote!(&emit::Empty));

    // TODO: Share more of the event props construction with `#[span]`
    let mut macro_evt_props = Props::new();
    let user_evt_props_tokens = evt_props.unwrap_or_else(|| quote!(emit::Empty));

    let span_name_prop = span_name_prop(name, &template);
    macro_evt_props.push(
        &span_name_prop,
        capture::default_fn_name(&span_name_prop),
        false,
        true,
    )?;

    check_evt_props(&macro_evt_props)?;
    check_span_props(&macro_evt_props)?;

    let macro_evt_props_tokens = macro_evt_props.gen_bound_props_tokens()?;

    let span_tokens = ctxt_props.match_bound_props_tokens(|ctxt_props_tokens| {
        let template_tokens = template.template_tokens();

        let evt_props_tokens = quote!(emit::__private::__PrivateMacroExtendedProps::new(#user_evt_props_tokens, #macro_evt_props_tokens));

        let panic_lvl_tokens = lvl_tokens(
            panic_lvl_tokens.as_ref(),
            default_lvl_tokens.as_ref(),
            emit_core::well_known::LVL_ERROR,
        );
        let lvl_tokens =
            optional_lvl_tokens(default_lvl_tokens.as_ref(), default_lvl_tokens.as_ref());

        Ok(quote!(
            emit::__private::__private_begin_span(
                #rt_tokens,
                #mdl_tokens,
                #lvl_tokens,
                #when_tokens,
                #ctxt_props_tokens,
                #evt_props_tokens,
                emit::__private::__private_complete_span(
                    #rt_tokens,
                    #template_tokens,
                    #lvl_tokens,
                    #panic_lvl_tokens,
                ),
            )
        ))
    })?;

    Ok(quote!(emit::__private::__must_use_span_guard(#span_tokens)))
}
