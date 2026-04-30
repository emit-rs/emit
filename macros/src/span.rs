use proc_macro2::{Ident, Span, TokenStream};
use syn::{
    parse::Parse, spanned::Spanned, Block, Expr, ExprAsync, ExprBlock, FieldValue, Item, ItemFn,
    Member, Signature, Stmt,
};

use crate::util::StmtFnName;
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
    fn_name: Option<Ident>,
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
        let mut fn_name = Arg::ident("fn_name");
        let mut catch_unwind = Arg::bool("catch_unwind");

        args::set_from_field_values(
            input.parse_terminated(FieldValue::parse, Token![,])?.iter(),
            [
                &mut mdl,
                &mut guard,
                &mut fn_name,
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

        if let Some(guard) = guard.peek() {
            if ok_lvl.peek().is_some()
                || err_lvl.peek().is_some()
                || err.peek().is_some()
                || catch_unwind.peek().is_some()
            {
                return Err(syn::Error::new(guard.span(), "the `guard` control parameter is incompatible with `catch_unwind`, `ok_lvl`, `err_lvl`, `panic_lvl`, or `err`"));
            }
        }

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
            fn_name: fn_name.take(),
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

    let mut evt_props = Props::new();

    let span_guard = args
        .guard
        .unwrap_or_else(|| Ident::new("__span", Span::call_site()));

    let default_lvl_tokens = opts.level;
    let panic_lvl_tokens = args.panic_lvl;
    let ok_lvl_tokens = args.ok_lvl;
    let err_lvl_tokens = args.err_lvl;
    let err_tokens = args.err;
    let catch_unwind = args.catch_unwind.unwrap_or_default();
    let setup_tokens = args.setup;

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

            evt_props.push(
                &fn_name_prop,
                capture::default_fn_name(&fn_name_prop),
                false,
                true,
            )?;

            Some(fn_name.binding_tokens())
        } else {
            None
        };

    check_evt_props(&evt_props)?;

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
                &evt_props,
                fn_name_tokens,
                setup_tokens,
                &span_guard,
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
                &evt_props,
                fn_name_tokens,
                setup_tokens,
                &span_guard,
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
                &evt_props,
                fn_name_tokens,
                setup_tokens,
                &span_guard,
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
                &evt_props,
                fn_name_tokens,
                setup_tokens,
                &span_guard,
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
    evt_props: &Props,
    fn_name_tokens: Option<TokenStream>,
    setup_tokens: Option<TokenStream>,
    span_guard: &Ident,
    body_tokens: TokenStream,
    default_lvl_tokens: Option<TokenStream>,
    panic_lvl_tokens: Option<TokenStream>,
    ok_lvl_tokens: Option<TokenStream>,
    err_lvl_tokens: Option<TokenStream>,
    err_tokens: Option<TokenStream>,
    catch_unwind: bool,
) -> Result<TokenStream, syn::Error> {
    let template_tokens = template.template_tokens();
    let span_name_tokens = template.template_literal_tokens();

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
            span_guard,
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
            span_guard,
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
        evt_props,
        &span_name_tokens,
        &default_completion_tokens,
        &default_lvl_tokens,
    )?;

    Ok(quote!({
        #fn_name_tokens
        #setup_tokens

        let (mut __span_guard, __ctxt) = #span_guard_tokens;

        __ctxt.call(move || {
            __span_guard.start();

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
    evt_props: &Props,
    fn_name_tokens: Option<TokenStream>,
    setup_tokens: Option<TokenStream>,
    span_guard: &Ident,
    body_tokens: TokenStream,
    default_lvl_tokens: Option<TokenStream>,
    panic_lvl_tokens: Option<TokenStream>,
    ok_lvl_tokens: Option<TokenStream>,
    err_lvl_tokens: Option<TokenStream>,
    err_tokens: Option<TokenStream>,
    catch_unwind: bool,
) -> Result<TokenStream, syn::Error> {
    let template_tokens = template.template_tokens();
    let span_name_tokens = template.template_literal_tokens();

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
            span_guard,
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
            span_guard,
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
        evt_props,
        &span_name_tokens,
        &default_completion_tokens,
        &default_lvl_tokens,
    )?;

    Ok(quote!({
        #fn_name_tokens
        #setup_tokens

        let (mut __span_guard, __ctxt) = #span_guard_tokens;

        __ctxt.in_future(async move {
            __span_guard.start();

            let #span_guard = __span_guard;
            #body_tokens
        }).await
    }))
}

fn span_guard_tokens(
    rt_tokens: &TokenStream,
    mdl_tokens: &TokenStream,
    when_tokens: &TokenStream,
    ctxt_props: &Props,
    evt_props: &Props,
    span_name_tokens: &TokenStream,
    default_completion_tokens: &TokenStream,
    default_lvl_tokens: &TokenStream,
) -> Result<TokenStream, syn::Error> {
    let ctxt_props_match_input_tokens = ctxt_props.match_input_tokens();
    let ctxt_props_match_binding_tokens = ctxt_props.match_binding_tokens();
    let ctxt_props_tokens = ctxt_props.match_bound_tokens().to_ref_tokens();

    // We use type-preserving props here because they may span across await points
    let evt_props_tokens = evt_props.raw_props_tokens()?;

    Ok(quote!(match (#(#ctxt_props_match_input_tokens),*) {
        (#(#ctxt_props_match_binding_tokens),*) => {
            emit::__private::__private_begin_span(
                #rt_tokens,
                #mdl_tokens,
                #span_name_tokens,
                #default_lvl_tokens,
                #when_tokens,
                #ctxt_props_tokens,
                #evt_props_tokens,
                #default_completion_tokens,
            )
        }
    }))
}

struct FnName {
    ident: Ident,
    value: String,
}

impl FnName {
    fn binding_tokens(&self) -> TokenStream {
        let binding = &self.ident;
        let name = &self.value;

        quote!(let #binding = #name;)
    }

    fn to_prop(&self) -> FieldValue {
        let ident = &self.ident;
        let span = self.ident.span();

        FieldValue {
            attrs: vec![],
            // Bind as `x: x` instead of `x: "name"` so `x` doesn't trigger
            // unused warnings. The binding is assigned within the body of the span
            member: Member::Named(self.ident.clone()),
            colon_token: Some(Token![:](span)),
            expr: parse_quote_spanned!(span=> #ident),
        }
    }
}

fn fn_name(binding: Option<&Ident>, name: Option<&Ident>) -> Result<Option<FnName>, syn::Error> {
    match (binding, name) {
        (Some(binding), Some(name)) => Ok(Some(FnName {
            ident: binding.clone(),
            value: name.to_string(),
        })),
        (None, _) => Ok(None),
        (Some(binding), None) => Err(syn::Error::new(
            binding.span(),
            "cannot bind the name of an anonymous function",
        )),
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
    span_guard: &Ident,
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
    span_guard: &Ident,
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

    let Args {
        rt,
        mdl,
        when,
        guard,
        setup,
        fn_name,
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

    let ctxt_props_tokens = ctxt_props.props_tokens().to_ref_tokens();
    let template_tokens = template.template_tokens();
    let span_name_tokens = template.template_literal_tokens();

    let evt_props_tokens = quote!(emit::Empty);

    let panic_lvl_tokens = lvl_tokens(
        panic_lvl_tokens.as_ref(),
        default_lvl_tokens.as_ref(),
        emit_core::well_known::LVL_ERROR,
    );
    let lvl_tokens = optional_lvl_tokens(default_lvl_tokens.as_ref(), default_lvl_tokens.as_ref());

    Ok(quote!(
        emit::__private::__private_begin_span(
            #rt_tokens,
            #mdl_tokens,
            #span_name_tokens,
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
}
