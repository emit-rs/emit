use std::collections::BTreeMap;

use proc_macro2::{Span, TokenStream};
use syn::{parse::Parse, spanned::Spanned, Attribute, FieldValue, Ident};

use crate::{
    capture,
    util::{AttributeCfg, FieldValueKey},
};

#[derive(Debug)]
pub struct Props {
    match_value_tokens: Vec<TokenStream>,
    match_binding_tokens: Vec<TokenStream>,
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
    match_bound_tokens: TokenStream,
    direct_bound_tokens: TokenStream,
    label: Ident,
    span: Span,
    pub interpolated: bool,
    pub captured: bool,
    pub cfg_attr: Option<Attribute>,
    pub attrs: Vec<Attribute>,
}

impl KeyValue {
    pub fn span(&self) -> Span {
        self.span.clone()
    }

    pub fn hole_tokens(&self) -> TokenStream {
        let label = &self.label;
        let attrs = &self.attrs;

        quote!(#(#attrs)* #label)
    }
}

impl Props {
    pub fn new() -> Self {
        Props {
            match_value_tokens: Vec::new(),
            match_binding_tokens: Vec::new(),
            key_values: BTreeMap::new(),
            key_value_index: 0,
        }
    }

    pub fn match_input_tokens(&self) -> impl Iterator<Item = &TokenStream> {
        self.match_value_tokens.iter()
    }

    pub fn match_binding_tokens(&self) -> impl Iterator<Item = &TokenStream> {
        self.match_binding_tokens.iter()
    }

    pub fn match_bound_tokens(&self) -> TokenStream {
        Self::sorted_props_tokens(self.key_values.values().map(|kv| &kv.match_bound_tokens))
    }

    pub fn props_tokens(&self) -> TokenStream {
        Self::sorted_props_tokens(self.key_values.values().map(|kv| &kv.direct_bound_tokens))
    }

    fn sorted_props_tokens<'a>(
        key_values: impl Iterator<Item = &'a TokenStream> + 'a,
    ) -> TokenStream {
        quote!(emit::__private::__PrivateMacroProps::from_array([#(#key_values),*]))
    }

    fn next_match_binding_ident(&mut self, span: Span) -> Ident {
        let i = Ident::new(&format!("__tmp{}", self.key_value_index), span);
        self.key_value_index += 1;

        i
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

        let match_bound_ident = self.next_match_binding_ident(fv.span());

        let key_value_tokens = {
            let key_value_tokens =
                capture::key_value_with_hook(&attrs, &fv, fn_name, interpolated, captured)?;

            match cfg_attr {
                Some(ref cfg_attr) => quote_spanned!(fv.span()=>
                    #cfg_attr
                    {
                        #key_value_tokens
                    }
                ),
                None => key_value_tokens,
            }
        };

        self.match_value_tokens.push(key_value_tokens.clone());

        // If there's a #[cfg] then also push its reverse
        // This is to give a dummy value to the pattern binding since they don't support attributes
        if let Some(cfg_attr) = &cfg_attr {
            let cfg_attr = cfg_attr
                .invert_cfg()
                .ok_or_else(|| syn::Error::new(cfg_attr.span(), "attribute is not a #[cfg]"))?;

            self.match_value_tokens
                .push(quote_spanned!(fv.span()=> #cfg_attr ()));
        }

        self.match_binding_tokens
            .push(quote_spanned!(fv.span()=> #match_bound_ident));

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
                match_bound_tokens: quote_spanned!(fv.span()=> #cfg_attr (#match_bound_ident.0, #match_bound_ident.1)),
                direct_bound_tokens: quote_spanned!(fv.span()=> #key_value_tokens),
                span: fv.span(),
                label: fv.key_ident()?,
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
                ))
            }
            emit_core::well_known::KEY_TPL => {
                return Err(syn::Error::new(
                    v.span(),
                    "the template is specified as a string literal before properties",
                ))
            }
            emit_core::well_known::KEY_MSG => {
                return Err(syn::Error::new(
                    v.span(),
                    "the message is specified as a string literal template before properties",
                ))
            }
            emit_core::well_known::KEY_TS => {
                return Err(syn::Error::new(
                    v.span(),
                    "specify the timestamp using the `extent` control parameter before the template",
                ))
            }
            emit_core::well_known::KEY_TS_START => return Err(syn::Error::new(
                v.span(),
                "specify the start timestamp using the `extent` control parameter before the template",
            )),
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
