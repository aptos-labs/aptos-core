// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! `#[checks]`: declares which specified checks each method of an `impl`
//! block evaluates, and generates a registry of them.
//!
//! ```ignore
//! #[checks(registry = IMPLEMENTED_CHECKS)]
//! impl Checker {
//!     #[checks(F3, F4, F5, F6)]
//!     fn check_frame_geometry(&mut self) { ... }
//!
//!     #[checks(P1-P4, R1-R4)]
//!     fn check_slots(&mut self) { ... }
//! }
//! ```
//!
//! expands to the `impl` with the method attributes removed, plus
//!
//! ```ignore
//! pub const IMPLEMENTED_CHECKS: &[(&str, &[&str])] = &[
//!     ("check_frame_geometry", &["F3", "F4", "F5", "F6"]),
//!     ("check_slots", &["P1", "P2", "P3", "P4", "R1", "R2", "R3", "R4"]),
//! ];
//! ```
//!
//! An id is an uppercase letter followed by digits; `A1-A4` is the inclusive
//! range `A1, A2, A3, A4`. Malformed ids and ids repeated on one method are
//! compile errors. Checks may legitimately appear on several methods when the
//! methods share them.

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    spanned::Spanned,
    Attribute, Ident, ImplItem, ItemImpl, Token,
};

/// `registry = NAME`.
struct ImplArgs {
    registry: Ident,
}

impl Parse for ImplArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key: Ident = input.parse()?;
        if key != "registry" {
            return Err(syn::Error::new(key.span(), "expected `registry = NAME`"));
        }
        input.parse::<Token![=]>()?;
        Ok(Self {
            registry: input.parse()?,
        })
    }
}

/// One item of a method-level list: `F1` or `P1-P4`.
struct IdItem {
    ids: Vec<String>,
}

fn split_id(ident: &Ident) -> syn::Result<(char, u32)> {
    let text = ident.to_string();
    let mut chars = text.chars();
    let letter = chars.next().filter(char::is_ascii_uppercase);
    let number = chars.as_str().parse::<u32>().ok();
    match (letter, number) {
        (Some(letter), Some(number)) if !chars.as_str().is_empty() => Ok((letter, number)),
        _ => Err(syn::Error::new(
            ident.span(),
            format!("`{text}` is not a check id (an uppercase letter followed by digits)"),
        )),
    }
}

impl Parse for IdItem {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lo: Ident = input.parse()?;
        let (letter, start) = split_id(&lo)?;
        if !input.peek(Token![-]) {
            return Ok(Self {
                ids: vec![lo.to_string()],
            });
        }
        input.parse::<Token![-]>()?;
        let hi: Ident = input.parse()?;
        let (hi_letter, end) = split_id(&hi)?;
        if hi_letter != letter || end < start {
            return Err(syn::Error::new(
                hi.span(),
                format!("`{lo}-{hi}` is not an ascending range of one check family"),
            ));
        }
        Ok(Self {
            ids: (start..=end).map(|n| format!("{letter}{n}")).collect(),
        })
    }
}

/// Parses `#[checks(F1, P1-P4)]` into its expanded id list.
fn method_ids(attr: &Attribute) -> syn::Result<Vec<String>> {
    let items = attr.parse_args_with(Punctuated::<IdItem, Token![,]>::parse_terminated)?;
    let mut ids = Vec::new();
    for item in items {
        for id in item.ids {
            if ids.contains(&id) {
                return Err(syn::Error::new(
                    attr.span(),
                    format!("check `{id}` is listed twice on this method"),
                ));
            }
            ids.push(id);
        }
    }
    Ok(ids)
}

/// See the crate documentation.
#[proc_macro_attribute]
pub fn checks(args: TokenStream, item: TokenStream) -> TokenStream {
    let ImplArgs { registry } = parse_macro_input!(args as ImplArgs);
    let mut item_impl = parse_macro_input!(item as ItemImpl);

    let mut entries = Vec::new();
    for item in &mut item_impl.items {
        let ImplItem::Method(method) = item else {
            continue;
        };
        let (checks_attrs, rest): (Vec<_>, Vec<_>) = method
            .attrs
            .drain(..)
            .partition(|attr| attr.path.is_ident("checks"));
        method.attrs = rest;
        for attr in checks_attrs {
            let ids = match method_ids(&attr) {
                Ok(ids) => ids,
                Err(err) => return err.to_compile_error().into(),
            };
            let name = method.sig.ident.to_string();
            entries.push(quote! { (#name, &[#(#ids),*]) });
        }
    }

    quote! {
        #item_impl

        /// Checks evaluated by each method, as declared with `#[checks(...)]`.
        pub const #registry: &[(&str, &[&str])] = &[#(#entries),*];
    }
    .into()
}
