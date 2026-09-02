// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Derives `MoveValueView` for a struct or enum whose fields are themselves
//! `MoveValueView`.

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, punctuated::Punctuated, token::Comma, Data, DeriveInput, Fields, Index,
    Variant,
};

#[proc_macro_derive(MoveValueView)]
pub fn derive_move_value_view(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    // The generated body visits every field, so each generic type parameter
    // must itself be viewable.
    let mut generics = input.generics.clone();
    for param in generics.type_params_mut() {
        param
            .bounds
            .push(syn::parse_quote!(::move_value_view::MoveValueView));
    }
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let body = match &input.data {
        Data::Struct(data) => struct_visit_body(&data.fields),
        Data::Enum(data) => enum_visit_body(&data.variants),
        Data::Union(_) => {
            return syn::Error::new_spanned(name, "`MoveValueView` cannot be derived for a union")
                .to_compile_error()
                .into()
        },
    };

    quote! {
        impl #impl_generics ::move_value_view::MoveValueView for #name #ty_generics #where_clause {
            fn visit<V: ::move_value_view::MoveValueVisitor>(
                &self,
                visitor: V,
            ) -> ::std::result::Result<V::Ok, V::Error> {
                #body
            }
        }
    }
    .into()
}

fn struct_visit_body(fields: &Fields) -> proc_macro2::TokenStream {
    let count = fields.len();
    let accesses: Vec<_> = match fields {
        Fields::Named(named) => named
            .named
            .iter()
            .map(|f| {
                let ident = f.ident.as_ref().expect("a named field has an identifier");
                quote!(&self.#ident)
            })
            .collect(),
        Fields::Unnamed(unnamed) => (0..unnamed.unnamed.len())
            .map(|i| {
                let index = Index::from(i);
                quote!(&self.#index)
            })
            .collect(),
        Fields::Unit => Vec::new(),
    };
    quote! {
        let mut fields = ::move_value_view::MoveValueVisitor::visit_struct(visitor, #count)?;
        #(::move_value_view::FieldVisitor::field(&mut fields, #accesses)?;)*
        ::move_value_view::FieldVisitor::end(fields)
    }
}

fn enum_visit_body(variants: &Punctuated<Variant, Comma>) -> proc_macro2::TokenStream {
    let arms = variants.iter().enumerate().map(|(index, variant)| {
        let index = index as u32;
        let name = &variant.ident;
        let count = variant.fields.len();
        let binds = variant_bindings(&variant.fields);
        let pattern = match &variant.fields {
            Fields::Named(_) => quote!(Self::#name { #(#binds),* }),
            Fields::Unnamed(_) => quote!(Self::#name(#(#binds),*)),
            Fields::Unit => quote!(Self::#name),
        };
        quote! {
            #pattern => {
                let mut fields = ::move_value_view::MoveValueVisitor::visit_variant(
                    visitor, #index, #count,
                )?;
                #(::move_value_view::FieldVisitor::field(&mut fields, #binds)?;)*
                ::move_value_view::FieldVisitor::end(fields)
            }
        }
    });
    quote! { match self { #(#arms),* } }
}

fn variant_bindings(fields: &Fields) -> Vec<proc_macro2::TokenStream> {
    match fields {
        Fields::Named(named) => named
            .named
            .iter()
            .map(|f| {
                let ident = f.ident.as_ref().expect("a named field has an identifier");
                quote!(#ident)
            })
            .collect(),
        Fields::Unnamed(unnamed) => (0..unnamed.unnamed.len())
            .map(|i| {
                let ident = syn::Ident::new(&format!("field{i}"), proc_macro2::Span::call_site());
                quote!(#ident)
            })
            .collect(),
        Fields::Unit => Vec::new(),
    }
}
