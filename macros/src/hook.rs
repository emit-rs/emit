use std::{collections::HashMap, fmt::Write, sync::OnceLock};

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{
    parse::Parse,
    punctuated::Punctuated,
    spanned::Spanned,
    token::Comma,
    visit_mut::{self, VisitMut},
    Attribute, Expr, ExprMethodCall, Ident, Meta, MetaList,
};

use crate::util::parse_comma_separated2;

static HOOKS: OnceLock<
    HashMap<&'static str, fn(TokenStream, TokenStream) -> syn::Result<TokenStream>>,
> = OnceLock::new();

pub(crate) fn get(
    name: &str,
) -> Option<impl Fn(TokenStream, TokenStream) -> syn::Result<TokenStream>> {
    HOOKS.get_or_init(crate::hooks).get(name)
}

pub struct RenameHookTokens<P, T> {
    pub args: TokenStream,
    pub expr: TokenStream,
    pub predicate: P,
    pub to: T,
    pub name: &'static str,
    pub target: &'static str,
}

pub fn rename_hook_tokens<A: Parse>(
    opts: RenameHookTokens<
        impl Fn(&str) -> bool,
        impl Fn(&A, &Ident, &Punctuated<Expr, Comma>) -> Option<(TokenStream, TokenStream)>,
    >,
) -> Result<TokenStream, syn::Error> {
    let mut hook = syn::parse2::<Hook>(opts.expr)?;
    let mut visitor = RenameVisitor {
        scratch: String::new(),
        predicate: opts.predicate,
        to: opts.to,
        args: syn::parse2::<A>(opts.args)?,
        applied: false,
    };

    visitor.visit_expr_mut(&mut hook.expr);

    if !visitor.applied {
        Err(syn::Error::new(
            hook.expr.span(),
            format_args!(
                "`{}` isn't valid here; it can only be applied to {}",
                opts.name, opts.target
            ),
        ))
    } else {
        Ok(hook.to_token_stream())
    }
}

struct RenameVisitor<P, A, T> {
    scratch: String,
    predicate: P,
    args: A,
    to: T,
    applied: bool,
}

impl<P, A, T> VisitMut for RenameVisitor<P, A, T>
where
    P: Fn(&str) -> bool,
    T: Fn(&A, &Ident, &Punctuated<Expr, Comma>) -> Option<(TokenStream, TokenStream)>,
{
    fn visit_expr_method_call_mut(&mut self, i: &mut ExprMethodCall) {
        let ExprMethodCall { method, args, .. } = i;

        self.scratch.clear();
        write!(&mut self.scratch, "{}", method).expect("infallible write to string");

        if (self.predicate)(&self.scratch) {
            self.applied = true;

            if let Some((to_ident_tokens, to_arg_tokens)) = (self.to)(&self.args, &method, &args) {
                *method = syn::parse2(to_ident_tokens).expect("invalid ident");
                *args = parse_comma_separated2(to_arg_tokens).expect("invalid args");
            }
        }

        visit_mut::visit_expr_method_call_mut(self, i)
    }
}

/**
An expression with an optional trailing comma.

When reformatting the expression, the comma is discarded.
*/
struct Hook {
    expr: Expr,
}

impl Parse for Hook {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut items = input.parse_terminated(Expr::parse, Token![,])?;

        let expr = items
            .pop()
            .ok_or_else(|| syn::Error::new(input.span(), "missing expression"))?
            .into_value();

        if !items.is_empty() {
            return Err(syn::Error::new(
                input.span(),
                "expected a single expression",
            ));
        }

        Ok(Hook { expr })
    }
}

impl ToTokens for Hook {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Hook { expr } = self;

        tokens.extend(quote!(#expr));
    }
}

pub(crate) fn eval_hooks(attrs: &[Attribute], expr: Expr) -> syn::Result<TokenStream> {
    let mut unapplied = Vec::new();
    let mut expr = quote!(#expr);

    for attr in attrs {
        if attr.path().segments.len() == 2 {
            let root = attr.path().segments.first().unwrap();
            let name = attr.path().segments.last().unwrap();

            if root.ident == "emit" {
                let args = match &attr.meta {
                    Meta::List(MetaList { ref tokens, .. }) => Some(tokens),
                    _ => None,
                };

                if let Some(eval) = get(&name.ident.to_string()) {
                    expr = eval(quote!(#args), expr)?;
                    continue;
                }
            }
        }

        unapplied.push(attr.clone());
    }

    Ok(quote_spanned!(expr.span()=> #(#unapplied)* #expr))
}
