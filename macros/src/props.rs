use std::collections::BTreeMap;

use proc_macro2::{Span, TokenStream};
use syn::{Attribute, FieldValue, Ident, parse::Parse, spanned::Spanned};

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
                    quote_spanned!(kv.span()=> (#key_tokens, #value_expr)),
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

    pub fn direct_bound_props_tokens(&self) -> Result<TokenStream, syn::Error> {
        let mut err = None;

        let key_values = self.key_values.values().filter_map(|kv| {
            let key_value_tokens = match capture::eval_key_value_with_hook(
                &kv.attrs,
                &kv.fv,
                &kv.fn_name,
                kv.interpolated,
                kv.captured,
            ) {
                Ok(key_value_tokens) => key_value_tokens,
                Err(e) => {
                    err = Some(e);
                    return None;
                }
            };

            let key_value_tokens = maybe_cfg(kv.cfg_attr.as_ref(), kv.span(), key_value_tokens);

            Some(quote_spanned!(kv.span()=> #key_value_tokens))
        });

        let props_tokens =
            quote!(emit::__private::__PrivateMacroProps::from_array([#(#key_values),*]));

        match err {
            None => Ok(props_tokens),
            Some(err) => Err(err),
        }
    }

    pub fn raw_bound_props_tokens(&self) -> Result<TokenStream, syn::Error> {
        // Make sure no key-values carry attributes
        // This is a limitation imposed while this code is only used internally
        // If we expose some way for users to produce raw props we'll need to rethink this
        for (k, v) in &self.key_values {
            if v.attrs.len() != 0 {
                return Err(syn::Error::new(
                    v.span(),
                    format!("attributes on {k} are not supported when capturing directly"),
                ));
            }
        }

        match self.key_values.len() {
            0 => Ok(quote!(emit::Empty)),
            1 => capture::raw_key_value(&self.key_values.first_key_value().unwrap().1.fv),
            _ => {
                let mut err = None;

                let key_values = self.key_values.values().filter_map(|kv| {
                    match capture::raw_key_value(&kv.fv) {
                        Ok(key_value_tokens) => Some(key_value_tokens),
                        Err(e) => {
                            err = Some(e);
                            None
                        }
                    }
                });

                let props_tokens =
                    quote!(emit::__private::__PrivateTupleMacroProps::new((#(#key_values),*)));

                match err {
                    None => Ok(props_tokens),
                    Some(err) => Err(err),
                }
            }
        }
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
