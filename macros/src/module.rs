use proc_macro2::TokenStream;

pub(crate) fn module_tokens() -> TokenStream {
    quote!(emit::module!())
}
