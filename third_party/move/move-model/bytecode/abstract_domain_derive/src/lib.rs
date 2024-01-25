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
/// Derives `AbstractDomain` for structs. The derived `join` method joins selected fields of a struct, or all fields for structs with anonymous fields, and returns the combined join results.
/// The joined fields must implement `AbstractDomain`.
/// # Usage
///
/// Add `#[derive(AbstractDomain)]` attribute on the struct definition, and `#[join]` on the fields to be.
/// For example,
/// ```
/// pub struct BorrowInfo {
///     live_nodes: SetDomain<BorrowNode>,

///     borrowed_by: MapDomain<BorrowNode, SetDomain<(BorrowNode, BorrowEdge)>>,
///     /// Backward borrow information. This field is not used during analysis, but computed once
///     /// analysis is done.
///     borrows_from: MapDomain<BorrowNode, SetDomain<(BorrowNode, BorrowEdge)>>,
/// }
///
/// impl AbstractDomain for BorrowInfo {
///     fn join(&mut self, other: &Self) -> JoinResult {
///         let live_changed = self.live_nodes.join(&other.live_nodes);
///         let borrowed_changed = self.borrowed_by.join(&other.borrowed_by);
///         borrowed_changed.combine(live_changed)
///     }
/// }
/// ```
/// Can be derived with
/// ```
/// #[derive(AbstractDomain)]
/// pub struct BorrowInfo {
///     #[join]
///     live_nodes: SetDomain<BorrowNode>,
///     #[join]
///     borrowed_by: MapDomain<BorrowNode, SetDomain<(BorrowNode, BorrowEdge)>>,
///     // this field is not joined
///     borrows_from: MapDomain<BorrowNode, SetDomain<(BorrowNode, BorrowEdge)>>,
/// }
/// ```
/// For structs with unnamed fields, the derived `join` method joins *every* field, and no need to write `#[join]`. For example,
/// ```
/// #[derive(AbstractDomain)]
/// struct LiveVars(SetDomain);
/// ```
/// derives a `join` that joins the wrapped field.
///
/// This also works for unit structs. For example,
/// ```
/// #[derive(AbstractDomain)]
/// struct Unit;
/// ```
/// derives a `join` that does nothing and always returns `Unchanged` since `Unit` has no fields.
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
