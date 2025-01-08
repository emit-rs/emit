use std::collections::BTreeMap;

use proc_macro2::TokenStream;

use syn::{parse::Parse, punctuated::Punctuated, spanned::Spanned, Expr, ExprPath, FieldValue};

use crate::{fmt, props::Props, util::FieldValueKey};

pub fn parse2<A: Parse>(
    input: TokenStream,
    fn_name: impl Fn(&FieldValue) -> TokenStream,
    captured: bool,
) -> Result<(A, Option<Template>, Props), syn::Error> {
    let template =
        fv_template::Template::parse2(input).map_err(|e| syn::Error::new(e.span(), e))?;

    // Parse args from the field values before the template
    let args = {
        let args = template
            .before_literal_field_values()
            .cloned()
            .collect::<Punctuated<FieldValue, Token![,]>>();
        syn::parse2(quote!(#args))?
    };

    // Any field-values that aren't part of the template
    let mut extra_field_values: BTreeMap<_, _> = template
        .after_literal_field_values()
        .map(|fv| Ok((fv.key_name()?, fv)))
        .collect::<Result<_, syn::Error>>()?;

    let mut props = Props::new();

    // Push the field-values that appear in the template
    for fv in template.literal_field_values() {
        let k = fv.key_name()?;

        // If the hole has a corresponding field-value outside the template
        // then it will be used as the source for the value and attributes
        // In this case, it's expected that the field-value in the template is
        // just a single identifier
        match extra_field_values.remove(&k) {
            Some(extra_fv) => {
                if let Expr::Path(ExprPath { ref path, .. }) = fv.expr {
                    // Make sure the field-value in the template is just a plain identifier
                    if !fv.attrs.is_empty() {
                        return Err(syn::Error::new(fv.span(), "keys that exist in the template and extra pairs can only use attributes on the extra pair"));
                    }

                    assert_eq!(
                        path.get_ident().map(|ident| ident.to_string()).as_ref(),
                        Some(&k),
                        "the key name and path don't match"
                    );
                } else {
                    return Err(syn::Error::new(
                        extra_fv.span(),
                        "keys that exist in the template and extra pairs can only use identifiers",
                    ));
                }

                props.push(extra_fv, fn_name(extra_fv), true, captured)?;
            }
            None => {
                props.push(fv, fn_name(fv), true, captured)?;
            }
        }
    }

    // Push any remaining extra field-values
    // This won't include any field values that also appear in the template
    for (_, fv) in extra_field_values {
        props.push(fv, fn_name(fv), false, captured)?;
    }

    // A runtime representation of the template
    let (template_parts_tokens, template_literal_tokens) = {
        let mut template_visitor = TemplateVisitor {
            props: &props,
            parts: Ok(Vec::new()),
            literal: String::new(),
        };
        template.visit_literal(&mut template_visitor);
        let template_parts = template_visitor.parts?;
        let literal = template_visitor.literal;

        let parts_tokens = {
            quote!({
                const __TPL_PARTS: &[emit::template::Part] = &[
                    #(#template_parts),*
                ];

                __TPL_PARTS
            })
        };

        let literal_tokens = quote!(#literal);

        (parts_tokens, literal_tokens)
    };

    let template = if template.has_literal() {
        Some(Template {
            template_parts_tokens,
            template_literal_tokens,
        })
    } else {
        None
    };

    Ok((args, template, props))
}

pub struct Template {
    template_parts_tokens: TokenStream,
    template_literal_tokens: TokenStream,
}

impl Template {
    pub fn template_literal_tokens(&self) -> TokenStream {
        self.template_literal_tokens.clone()
    }

    pub fn template_tokens(&self) -> TokenStream {
        let template_parts = &self.template_parts_tokens;

        quote!(emit::Template::new_ref(#template_parts))
    }
}

struct TemplateVisitor<'a> {
    props: &'a Props,
    parts: syn::Result<Vec<TokenStream>>,
    literal: String,
}

impl<'a> fv_template::LiteralVisitor for TemplateVisitor<'a> {
    fn visit_hole(&mut self, hole: &FieldValue) {
        let Ok(ref mut parts) = self.parts else {
            return;
        };

        if let Err(e) = (|| {
            let label = hole.key_name()?;
            let hole = hole.key_expr()?;

            let field = self.props.get(&label).expect("missing prop");

            debug_assert!(field.interpolated);

            self.literal.push_str("{");
            self.literal.push_str(&label);
            self.literal.push_str("}");

            let hole_tokens =
                fmt::template_hole_with_hook(&field.attrs, &hole, true, field.captured)?;
            match field.cfg_attr {
                Some(ref cfg_attr) => parts.push(quote!(#cfg_attr { #hole_tokens })),
                _ => parts.push(quote!(#hole_tokens)),
            }

            Ok::<(), syn::Error>(())
        })() {
            self.parts = Err(e);
        }
    }

    fn visit_text(&mut self, text: &str) {
        let Ok(ref mut parts) = self.parts else {
            return;
        };

        self.literal.push_str(text);

        parts.push(quote!(emit::template::Part::text(#text)));
    }
}
