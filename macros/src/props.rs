use std::collections::BTreeMap;

use proc_macro2::{Span, TokenStream};
use syn::{Attribute, FieldValue, Ident, parse::Parse, spanned::Spanned};

use crate::util::maybe_cfg_else;
use crate::{
    capture, hook,
    util::{AttributeCfg, ExprIsLocalVariable, FieldValueKey, maybe_cfg},
};

#[derive(Debug)]
pub struct Props {
    key_values: BTreeMap<String, KeyValue>,
    key_value_index: usize,
}

impl Parse for Props {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let fv = input.parse_terminated(FieldValue::parse, Token![,])?;

        let mut props = Props::new();

        for fv in fv {
            props.push(&fv, capture::default_fn_name(&fv), false, true)?;
        }

        Ok(props)
    }
}

#[derive(Debug)]
pub struct KeyValue {
    idx: usize,
    fv: FieldValue,
    fn_name: TokenStream,
    pub interpolated: bool,
    pub captured: bool,
    pub cfg_attr: Option<Attribute>,
    pub attrs: Vec<Attribute>,
}

impl KeyValue {
    pub fn span(&self) -> Span {
        self.fv.span()
    }

    pub fn hole_tokens(&self) -> Result<TokenStream, syn::Error> {
        let label = self.fv.key_ident()?;
        let attrs = &self.attrs;

        Ok(quote!(#(#attrs)* #label))
    }
}

impl Props {
    pub fn new() -> Self {
        Props {
            key_values: BTreeMap::new(),
            key_value_index: 0,
        }
    }

    pub fn match_bound_props_tokens(
        &self,
        match_arm: impl FnOnce(TokenStream) -> Result<TokenStream, syn::Error>,
    ) -> Result<TokenStream, syn::Error> {
        let mut match_input_tokens = Vec::new();
        let mut match_binding_tokens = Vec::new();
        let mut match_bound_tokens = Vec::new();

        for kv in self.key_values.values() {
            let match_bound_ident = Ident::new(&format!("__tmp{}", kv.idx), kv.span());

            // This is one of the few places we end up looking at the shape of an expression and deciding how to emit code for it.
            //
            // In the 2021 edition and prior, lifetimes of temporaries created in a `match expr` would be extended to the end of the
            // `match`. In the 2024 edition, that doesn't happen anymore. So to keep the semantics that you can capture a value in scope
            // by reference, and supply temporaries inline, we check whether the field value is a local like `x: a.b.c` and take a reference
            // inside the `match expr`, or a complex expression like `x: a.b()`, where it's taken by value in the `match expr`.
            let (kv_match_input_tokens, kv_match_bound_tokens) = if kv.fv.expr.is_local_variable() {
                let key_value_tokens = maybe_cfg(
                    kv.cfg_attr.as_ref(),
                    kv.span(),
                    capture::eval_key_value_with_hook(
                        &kv.attrs,
                        &kv.fv,
                        &kv.fn_name,
                        kv.interpolated,
                        kv.captured,
                    )?,
                );

                // If the expression is a local variable then it's an already in-scope identifier
                // We take the expression by reference in a `match`

                let cfg_attr = &kv.cfg_attr;
                let kv_match_input_tokens = quote_spanned!(kv.span()=>#key_value_tokens);
                let kv_match_bound_tokens = quote_spanned!(kv.span()=>#cfg_attr (#match_bound_ident.0, #match_bound_ident.1));

                (kv_match_input_tokens, kv_match_bound_tokens)
            } else {
                let cfg_attr = &kv.cfg_attr;

                // If the expression is not a local variable then it's constructed for the call
                // We take the expression by value in a `match`

                let key_tokens =
                    capture::eval_key_with_hook(&kv.attrs, &kv.fv, kv.interpolated, kv.captured)?;
                let value_expr = &kv.fv.expr;

                let kv_match_input_tokens = maybe_cfg(
                    kv.cfg_attr.as_ref(),
                    kv.span(),
                    quote_spanned!(kv.span()=> {(#key_tokens, #value_expr)}),
                );

                let bound_value_tokens = capture::value_with_hook(
                    &syn::parse_quote_spanned!(kv.fv.span()=>#match_bound_ident.1),
                    &kv.fn_name,
                    kv.interpolated,
                    kv.captured,
                );
                let bound_value_tokens = hook::eval_hooks(
                    &kv.attrs,
                    syn::parse_quote_spanned!(kv.span()=>#bound_value_tokens),
                )?;

                let kv_match_bound_tokens =
                    quote!(#cfg_attr (#match_bound_ident.0, #bound_value_tokens));

                (kv_match_input_tokens, kv_match_bound_tokens)
            };

            match_input_tokens.push(kv_match_input_tokens);
            match_binding_tokens.push(quote_spanned!(kv.span()=> #match_bound_ident));

            // If there's a #[cfg] then also push its reverse
            // This is to give a dummy value to the pattern binding since they don't support attributes
            if let Some(cfg_attr) = &kv.cfg_attr {
                let cfg_attr = cfg_attr
                    .invert_cfg()
                    .ok_or_else(|| syn::Error::new(cfg_attr.span(), "attribute is not a #[cfg]"))?;

                match_input_tokens.push(quote_spanned!(kv.span()=> #cfg_attr ()));
            }

            match_bound_tokens.push(kv_match_bound_tokens);
        }

        let props_tokens =
            quote!(emit::__private::__PrivateMacroProps::from_array([#(#match_bound_tokens),*]));
        let body_tokens = match_arm(props_tokens)?;

        Ok(quote!({
            match (#(#match_input_tokens),*) {
                (#(#match_binding_tokens),*) => #body_tokens,
            }
        }))
    }

    pub fn gen_bound_props_tokens(&self) -> Result<TokenStream, syn::Error> {
        let mut struct_decl_tys = Vec::new();
        let mut struct_decl_fvs = Vec::new();
        let mut struct_decl_markers = Vec::new();

        let mut impl_decl_tys = Vec::new();
        let mut impl_struct_tys = Vec::new();
        let mut impl_for_each = Vec::new();
        let mut impl_to_value = Vec::new();

        let mut let_bindings = Vec::new();

        let mut new_decl_args = Vec::new();
        let mut new_fvs = Vec::new();
        let mut new_ctor_args = Vec::new();

        for kv in self.key_values.values() {
            let input_field = kv.fv.key_ident()?;

            let input_ident = Ident::new(&format!("__i{}", kv.idx), kv.span());
            let fn_ident = Ident::new(&format!("__f{}", kv.idx), kv.span());

            let input_ty = Ident::new(&format!("__I{}", kv.idx), kv.span());
            let fn_ty = Ident::new(&format!("__F{}", kv.idx), kv.span());

            let cfg_attr = kv.cfg_attr.as_ref();
            let invert_cfg_attr = cfg_attr.and_then(|cfg_attr| cfg_attr.invert_cfg());

            struct_decl_tys.push(quote!(#input_ty));
            struct_decl_tys.push(quote!(#fn_ty));

            impl_decl_tys.push(quote!(#input_ty));
            impl_decl_tys
                .push(quote!(#fn_ty: Fn(&#input_ty) -> (emit::Str<'_>, emit::__private::core::option::Option<emit::Value<'_>>)));
            impl_struct_tys.push(quote!(#input_ty));
            impl_struct_tys.push(quote!(#fn_ty));

            struct_decl_fvs.push(quote!(#cfg_attr pub #input_field: #input_ty));
            if let Some(invert_cfg_attr) = &invert_cfg_attr {
                struct_decl_fvs.push(quote!(#invert_cfg_attr #input_field: #input_ty));
            }

            struct_decl_fvs.push(quote!(#fn_ident: #fn_ty));
            struct_decl_markers.push(quote!(#input_ty));
            struct_decl_markers.push(quote!(#fn_ty));

            let value = &kv.fv.expr;
            let value = maybe_cfg(cfg_attr, kv.span(), quote!({#value}));

            let_bindings.push(quote!(let #input_ident = { #value }));

            let key_tokens =
                capture::eval_key_with_hook(&kv.attrs, &kv.fv, kv.interpolated, kv.captured)?;

            let value_tokens = capture::eval_value_with_hook(
                &kv.attrs,
                &syn::parse_quote_spanned!(kv.fv.span()=>(*#input_ident)),
                &kv.fn_name,
                kv.interpolated,
                kv.captured,
            )?;

            let fn_body = quote!((#key_tokens, #value_tokens));
            let fn_body = maybe_cfg_else(
                cfg_attr,
                kv.span(),
                fn_body,
                quote!(emit::__private::core::unreachable!()),
            )?;

            new_decl_args.push(quote!(#fn_ident: #fn_ty));
            new_decl_args.push(quote!(#input_ident: #input_ty));

            new_fvs.push(quote!(#fn_ident));
            new_fvs.push(quote!(#input_field: #input_ident));

            new_ctor_args
                .push(quote!((&#input_ident).__private_infer_input(|#input_ident| #fn_body)));
            new_ctor_args.push(quote!(#input_ident));

            impl_for_each.push(maybe_cfg_else(
                cfg_attr,
                kv.span(),
                quote!(
                    {
                        match (self.#fn_ident)(&self.#input_field) {
                            (k, emit::__private::core::option::Option::Some(v)) => for_each(k, v)?,
                            _ => (),
                        }
                    }
                ),
                quote!({ let _ = self.#input_field; }),
            )?);

            impl_to_value.push(maybe_cfg_else(
                cfg_attr,
                kv.span(),
                quote!({ (self.#fn_ident)(&self.#input_field).1.unwrap_or(emit::Value::null()) }),
                quote!({ emit::Value::null() }),
            )?);
        }

        struct_decl_fvs.push(quote!(__marker: emit::__private::core::marker::PhantomData<(#(#struct_decl_markers,)*)>));
        new_fvs.push(quote!(__marker: emit::__private::core::marker::PhantomData));

        let single_impls = if self.key_values.len() == 1 {
            Some(quote!(
                impl<#(#impl_decl_tys,)*> emit::value::ToValue for __PrivateMacroGenProps<#(#impl_struct_tys,)*> {
                    fn to_value(&self) -> emit::Value<'_> {
                        #(#impl_to_value)*
                    }
                }
            ))
        } else {
            None
        };

        Ok(quote!({
            #[allow(unused_imports)]
            use emit::__private::__PrivateInferInput;

            mod __private_macro_gen_props {
                pub(super) struct __PrivateMacroGenProps<#(#struct_decl_tys,)*> {
                    #(#struct_decl_fvs,)*
                }

                impl<#(#impl_decl_tys,)*> __PrivateMacroGenProps<#(#impl_struct_tys,)*> {
                    pub(super) fn __new(
                        #(#new_decl_args,)*
                    ) -> Self {
                        __PrivateMacroGenProps {
                            #(#new_fvs,)*
                        }
                    }
                }

                #single_impls

                impl<#(#impl_decl_tys,)*> emit::Props for __PrivateMacroGenProps<#(#impl_struct_tys,)*> {
                    fn for_each<
                        'kv,
                        F: emit::__private::core::ops::FnMut(emit::Str<'kv>, emit::Value<'kv>) -> emit::__private::core::ops::ControlFlow<()>,
                    >(&'kv self, mut for_each: F) -> emit::__private::core::ops::ControlFlow<()> {
                        #(#impl_for_each)*

                        emit::__private::core::ops::ControlFlow::Continue(())
                    }

                    fn is_unique(&self) -> bool {
                        true
                    }
                }
            }

            #(#let_bindings;)*

            __private_macro_gen_props::__PrivateMacroGenProps::__new(#(#new_ctor_args,)*)
        }))
    }

    pub fn get(&self, label: &str) -> Option<&KeyValue> {
        self.key_values.get(label)
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = (&'a str, &'a KeyValue)> + 'a {
        self.key_values.iter().map(|(k, v)| (&**k, v))
    }

    pub fn push(
        &mut self,
        fv: &FieldValue,
        fn_name: TokenStream,
        interpolated: bool,
        captured: bool,
    ) -> Result<(), syn::Error> {
        let mut attrs = vec![];
        let mut cfg_attr = None;

        for attr in &fv.attrs {
            if attr.is_cfg() {
                if cfg_attr.is_some() {
                    return Err(syn::Error::new(
                        attr.span(),
                        "only a single #[cfg] is supported on key-value pairs",
                    ));
                }

                cfg_attr = Some(attr.clone());
            } else {
                attrs.push(attr.clone());
            }
        }

        let idx = self.key_value_index;
        self.key_value_index += 1;

        if fv.colon_token.is_some() && !captured {
            return Err(syn::Error::new(
                fv.span(),
                "uncaptured key values must be plain identifiers",
            ));
        }

        // Make sure keys aren't duplicated
        let previous = self.key_values.insert(
            fv.key_name()?,
            KeyValue {
                idx,
                fv: fv.clone(),
                fn_name,
                cfg_attr,
                attrs,
                captured,
                interpolated,
            },
        );

        if previous.is_some() {
            return Err(syn::Error::new(fv.span(), "keys cannot be duplicated"));
        }

        Ok(())
    }
}

/**
Check properties for reserved keys used by event metadata.
*/
pub fn check_evt_props(props: &Props) -> Result<(), syn::Error> {
    for (k, v) in &props.key_values {
        match &**k {
            emit_core::well_known::KEY_MDL => {
                return Err(syn::Error::new(
                    v.span(),
                    "specify the module using the `mdl` control parameter before the template",
                ));
            }
            emit_core::well_known::KEY_TPL => {
                return Err(syn::Error::new(
                    v.span(),
                    "the template is specified as a string literal before properties",
                ));
            }
            emit_core::well_known::KEY_MSG => {
                return Err(syn::Error::new(
                    v.span(),
                    "the message is specified as a string literal template before properties",
                ));
            }
            emit_core::well_known::KEY_TS => {
                return Err(syn::Error::new(
                    v.span(),
                    "specify the timestamp using the `extent` control parameter before the template",
                ));
            }
            emit_core::well_known::KEY_TS_START => {
                return Err(syn::Error::new(
                    v.span(),
                    "specify the start timestamp using the `extent` control parameter before the template",
                ));
            }
            _ => (),
        }
    }

    Ok(())
}

/**
Check properties for reserved keys used by event metadata.
*/
pub fn check_span_props(props: &Props) -> Result<(), syn::Error> {
    for (k, v) in &props.key_values {
        match &**k {
            emit_core::well_known::KEY_EVT_KIND => {
                return Err(syn::Error::new(
                    v.span(),
                    "the `evt_kind` property is always given the value `\"span\"`",
                ));
            }
            emit_core::well_known::KEY_SPAN_NAME => {
                return Err(syn::Error::new(
                    v.span(),
                    "specify the span name using the `name` control parameter before the template",
                ));
            }
            _ => (),
        }
    }

    Ok(())
}

/**
Push common properties for events.
*/
pub fn push_evt_props(props: &mut Props, level: Option<TokenStream>) -> Result<(), syn::Error> {
    // Add the level as a property
    if let Some(level_value) = level {
        let level_ident = Ident::new(emit_core::well_known::KEY_LVL, Span::call_site());

        let fv = syn::parse2::<FieldValue>(quote!(#level_ident: #level_value))?;

        props.push(&fv, capture::default_fn_name(&fv), false, true)?;
    }

    Ok(())
}
