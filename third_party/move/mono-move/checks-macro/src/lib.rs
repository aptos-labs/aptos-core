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
//!
//! A method may also declare its time complexity, with what `N` measures and
//! an optional explanation:
//!
//! ```ignore
//! #[checks(G3, G4, G5, G7)]
//! #[complexity(n_log_n in "the number of layout slots"
//!              because "each safe-point slot is one binary search into `base`")]
//! fn check_gc_layouts(&mut self) { ... }
//! ```
//!
//! The class is one of `constant`, `log`, `linear`, `n_log_n`; the grammar has
//! nothing worse, so an unmetered checker cannot declare a method beyond
//! `O(N * log(N))`. The macro appends `Complexity: O(N * log(N)) in the number
//! of layout slots: each safe-point slot is one binary search into `base`.` to
//! the method's documentation, and the registry entry becomes
//! `(method, checks, complexity, measured_in, because)`.

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    spanned::Spanned,
    visit_mut::{self, VisitMut},
    Attribute, Ident, ImplItem, ItemImpl, LitStr, Token,
};

/// Collects and strips `#[check(...)]` tags from statements and match arms
/// inside a method body.
struct BodyTags {
    ids: Vec<String>,
    error: Option<syn::Error>,
}

impl BodyTags {
    fn take(&mut self, attrs: &mut Vec<Attribute>) {
        let (tags, rest): (Vec<_>, Vec<_>) = std::mem::take(attrs)
            .into_iter()
            .partition(|attr| attr.path().is_ident("check"));
        *attrs = rest;
        for tag in tags {
            match method_ids(&tag) {
                Ok(ids) => self.ids.extend(ids),
                Err(err) => {
                    self.error.get_or_insert(err);
                },
            }
        }
    }
}

impl VisitMut for BodyTags {
    fn visit_expr_mut(&mut self, expr: &mut syn::Expr) {
        if let Some(attrs) = expr_attrs_mut(expr) {
            self.take(attrs);
        }
        visit_mut::visit_expr_mut(self, expr);
    }

    fn visit_arm_mut(&mut self, arm: &mut syn::Arm) {
        self.take(&mut arm.attrs);
        visit_mut::visit_arm_mut(self, arm);
    }

    fn visit_local_mut(&mut self, local: &mut syn::Local) {
        self.take(&mut local.attrs);
        visit_mut::visit_local_mut(self, local);
    }
}

/// The outer attributes of an expression, for the expression kinds that can
/// carry them in statement position.
fn expr_attrs_mut(expr: &mut syn::Expr) -> Option<&mut Vec<Attribute>> {
    use syn::Expr::*;
    Some(match expr {
        If(e) => &mut e.attrs,
        Match(e) => &mut e.attrs,
        ForLoop(e) => &mut e.attrs,
        While(e) => &mut e.attrs,
        Loop(e) => &mut e.attrs,
        Block(e) => &mut e.attrs,
        Macro(e) => &mut e.attrs,
        Call(e) => &mut e.attrs,
        MethodCall(e) => &mut e.attrs,
        Let(e) => &mut e.attrs,
        _ => return None,
    })
}

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

/// `class [in "what"] [because "why"]`.
struct ComplexityArgs {
    /// Big-O rendering of the class.
    class: &'static str,
    /// What `N` measures, or empty.
    measured_in: String,
    /// Why the method has this class, or empty.
    because: String,
}

impl Parse for ComplexityArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let class_ident: Ident = input.parse()?;
        let class = match class_ident.to_string().as_str() {
            "constant" => "O(1)",
            "log" => "O(log(N))",
            "linear" => "O(N)",
            "n_log_n" => "O(N * log(N))",
            other => {
                return Err(syn::Error::new(
                    class_ident.span(),
                    format!(
                        "unknown complexity class `{other}`; expected constant, log, linear, or n_log_n (nothing worse is allowed)"
                    ),
                ))
            },
        };
        let measured_in = if input.peek(Token![in]) {
            input.parse::<Token![in]>()?;
            input.parse::<LitStr>()?.value()
        } else {
            String::new()
        };
        let because = if input.peek(Ident) && input.fork().parse::<Ident>()? == "because" {
            input.parse::<Ident>()?;
            input.parse::<LitStr>()?.value()
        } else {
            String::new()
        };
        Ok(Self {
            class,
            measured_in,
            because,
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
        let ImplItem::Fn(method) = item else {
            continue;
        };
        // Tags inside the body: `#[check(F3)]` on a statement or match arm.
        let mut body = BodyTags {
            ids: Vec::new(),
            error: None,
        };
        body.visit_block_mut(&mut method.block);
        if let Some(err) = body.error {
            return err.to_compile_error().into();
        }
        let (checks_attrs, rest): (Vec<_>, Vec<_>) = method
            .attrs
            .drain(..)
            .partition(|attr| attr.path().is_ident("checks"));
        let (complexity_attrs, rest): (Vec<_>, Vec<_>) = rest
            .into_iter()
            .partition(|attr| attr.path().is_ident("complexity"));
        method.attrs = rest;

        let mut complexity = None;
        for attr in complexity_attrs {
            if complexity.is_some() {
                return syn::Error::new(attr.span(), "a method declares its complexity once")
                    .to_compile_error()
                    .into();
            }
            let args: ComplexityArgs = match attr.parse_args() {
                Ok(args) => args,
                Err(err) => return err.to_compile_error().into(),
            };
            let mut line = format!(" Complexity: {}", args.class);
            if !args.measured_in.is_empty() {
                line.push_str(&format!(" in {}", args.measured_in));
            }
            if !args.because.is_empty() {
                line.push_str(&format!(": {}", args.because));
            }
            line.push('.');
            method.attrs.push(syn::parse_quote!(#[doc = ""]));
            method.attrs.push(syn::parse_quote!(#[doc = #line]));
            complexity = Some(args);
        }
        let (class, measured_in, because) = match &complexity {
            Some(args) => (args.class, args.measured_in.as_str(), args.because.as_str()),
            None => ("", "", ""),
        };

        let mut ids = body.ids;
        for attr in checks_attrs {
            match method_ids(&attr) {
                Ok(more) => ids.extend(more),
                Err(err) => return err.to_compile_error().into(),
            }
        }
        if ids.is_empty() && complexity.is_none() {
            continue;
        }
        ids.sort();
        ids.dedup();
        let name = method.sig.ident.to_string();
        entries.push(quote! { (#name, &[#(#ids),*], #class, #measured_in, #because) });
    }

    quote! {
        #item_impl

        /// Checks evaluated by each method, as declared with `#[checks(...)]`:
        /// `(method, checks, complexity class, what N measures, why)`. The
        /// last three are empty when the method declares no
        /// `#[complexity(...)]`.
        pub const #registry: &[(&str, &[&str], &str, &str, &str)] = &[#(#entries),*];
    }
    .into()
}
