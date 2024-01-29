use proc_macro2::TokenStream;
use quote::{quote, ToTokens};

pub fn fold_quote<F, I, T>(input: impl Iterator<Item = I>, f: F) -> TokenStream
where
    F: Fn(I) -> T,
    T: ToTokens,
{
    input.fold(quote! {}, |acc, x| {
        let y = f(x);
        quote! { #acc #y }
    })
}

pub fn is_unit(v: &syn::Variant) -> bool {
    match v.fields {
        syn::Fields::Unit => true,
        _ => false,
    }
}
