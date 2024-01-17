//! Derive macro for `AbstractDomain`
//!
//! Currently we can only derive for structs.
//! For tuple structs, the derived join joins each field;
//! for structs with named fields, the derived join joins each field with #[join] attribute.

use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{self, parse_macro_input, DeriveInput, Fields};

/// Given a field name, generates TokenStream of
/// `join_result = JoinResult::combine(join_result, self.field_name.join(&other.field_name));`
fn gen_join_field(field: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    quote! {
        join_result = JoinResult::combine(join_result, self.#field.join(&other.#field));
    }
}

#[proc_macro_derive(AbstractDomain, attributes(join))]
pub fn abstract_domain_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    // statements for joining fields
    let join_fields: Vec<_> = if let syn::Data::Struct(data_struct) = &input.data {
        match &data_struct.fields {
            Fields::Named(fields_named) => fields_named
                .named
                .iter()
                .filter_map(|field| {
                    field
                        .attrs
                        .iter()
                        .find(|attr| attr.path().is_ident("join"))
                        .map(|_| {
                            let field_name =
                                field.ident.as_ref().expect("field name").to_token_stream();
                            gen_join_field(field_name)
                        })
                })
                .collect(),
            Fields::Unnamed(fields_unnamed) => fields_unnamed
                .unnamed
                .iter()
                .enumerate()
                .map(|(idx, _)| {
                    let field_index = syn::Index::from(idx).to_token_stream();
                    gen_join_field(field_index)
                })
                .collect(),
            Fields::Unit => Vec::new(),
        }
    } else {
        panic!("AbstractDomain is only implemented for structs");
    };
    let expanded = quote! {
        impl AbstractDomain for #name {
            fn join(&mut self, other: &Self) -> JoinResult {
                let mut join_result = JoinResult::Unchanged;
                #(#join_fields)*
                join_result
            }
        }
    };
    expanded.into()
}
