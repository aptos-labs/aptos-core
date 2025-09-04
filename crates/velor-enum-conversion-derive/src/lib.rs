// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput};

#[proc_macro_derive(EnumConversion)]
pub fn derive_enum_converters(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);

    // Get the name of the enum type
    let enum_name = &input.ident;
    let generics = &input.generics;

    // Check if the input is an enum
    if let Data::Enum(enum_data) = &input.data {
        let result: Result<TokenStream, String> = enum_data
            .variants
            .iter()
            .map(|variant| {
                let variant_name = &variant.ident;
                if variant.fields.len() != 1 {
                    return Err("enum variant must have exactly one field".into());
                }
                if let syn::Fields::Unnamed(fields) = &variant.fields {
                    if let Some(struct_name) = fields.unnamed.first() {
                        let expanded = quote! {
                            impl #generics From<#struct_name> for #enum_name #generics {
                                fn from(struct_value: #struct_name) -> Self {
                                    Self::#variant_name(struct_value)
                                }
                            }

                            impl #generics TryFrom<#enum_name #generics> for #struct_name {
                                type Error = anyhow::Error;

                                fn try_from(msg: #enum_name #generics) -> Result<Self, Self::Error> {
                                    match msg {
                                        #enum_name::#variant_name(m) => Ok(m),
                                        _ => Err(anyhow::anyhow!("invalid message type")),
                                    }
                                }
                            }
                        };
                        return Ok(Into::<TokenStream>::into(expanded));
                    }
                }
                Err(format!("missing unnamed field in variant {}", variant_name))
            })
            .collect();

        return match result {
            Ok(token_stream) => token_stream,
            Err(error_message) => syn::Error::new_spanned(input, error_message)
                .to_compile_error()
                .into(),
        };
    }

    // If the input is not a valid struct, return a compilation error
    let error_message: &str = "This derive macro only supports enum types";
    syn::Error::new_spanned(input, error_message)
        .to_compile_error()
        .into()
}
