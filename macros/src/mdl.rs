use proc_macro2::TokenStream;

pub(crate) fn mdl_tokens() -> TokenStream {
    quote!(emit::mdl!())
}
