// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Provides macros for defining a spec sheet, and binding spec items to
//! their implementation. Use them through the `mono-move-spec` crate, which
//! re-exports them alongside the data types they emit.
//!
//! - `#[spec]` on an item turns the markdown check tables in its doc comment
//!   into `Self::CHECKS: &[CheckSpec]`, validating them at compile time.
//! - `#[check(R1)]` tags the statement or match arm that implements a check.
//! - `#[checks(registry = NAME)]` on an `impl` collects the tags into
//!   `NAME: &[MethodChecks]`.
//! - `#[complexity(class [in "what"] [because "why"])]` declares a method's
//!   time complexity and writes its `Complexity:` doc line. Classes:
//!   `constant`, `log`, `linear`, `n_log_n`; nothing worse exists.
//!
//! A test can then assert that every specified check is implemented and vice
//! versa.
//!
//! ```ignore
//! /// ## Requests
//! ///
//! /// | Id | Property              | Condition          |
//! /// |----|-----------------------|--------------------|
//! /// | R1 | a request has a body  | `len(body) > 0`    |
//! /// | R2 | a request fits a page | `len(body) <= MAX` |
//! #[spec]
//! pub struct Spec;
//!
//! #[checks(registry = IMPLEMENTED_CHECKS)]
//! impl Validator {
//!     #[complexity(constant)]
//!     fn check_request(&mut self, body: &[u8]) {
//!         #[check(R1)]
//!         if body.is_empty() { ... }
//!         #[check(R2)]
//!         if body.len() > MAX { ... }
//!     }
//! }
//! ```
//!
//! Ids are an uppercase letter and digits; `A1-A4` is a range. Tables need
//! `Id`, `Property`, `Condition` columns, optionally `Rationale`; the group is
//! the nearest `## ` heading.

use proc_macro::TokenStream;
use quote::quote;
use regex::Regex;
use std::sync::LazyLock;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    spanned::Spanned,
    visit_mut::{self, VisitMut},
    Attribute, Expr, ExprLit, Ident, ImplItem, Item, ItemImpl, Lit, LitStr, Meta, Token,
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
            match tag_ids(&tag) {
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

/// One item of a `#[check(...)]` list: `F1` or `P1-P4`.
struct IdItem {
    ids: Vec<String>,
}

/// `F12` -> `('F', 12)`: an uppercase letter followed by digits.
fn parse_check_id(text: &str) -> Option<(char, u32)> {
    let mut chars = text.chars();
    let letter = chars.next().filter(char::is_ascii_uppercase)?;
    let digits = chars.as_str();
    let number = digits.parse::<u32>().ok()?;
    digits
        .bytes()
        .all(|b| b.is_ascii_digit())
        .then_some((letter, number))
}

fn split_id(ident: &Ident) -> syn::Result<(char, u32)> {
    let text = ident.to_string();
    parse_check_id(&text).ok_or_else(|| {
        syn::Error::new(
            ident.span(),
            format!("`{text}` is not a check id (an uppercase letter followed by digits)"),
        )
    })
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
    /// `ComplexityClass` variant name.
    variant: &'static str,
    /// What `N` measures, or empty.
    measured_in: String,
    /// Why the method has this class, or empty.
    because: String,
}

impl Parse for ComplexityArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let class_ident: Ident = input.parse()?;
        let (class, variant) = match class_ident.to_string().as_str() {
            "constant" => ("O(1)", "Constant"),
            "log" => ("O(log(N))", "Log"),
            "linear" => ("O(N)", "Linear"),
            "n_log_n" => ("O(N * log(N))", "NLogN"),
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
            variant,
            measured_in,
            because,
        })
    }
}

/// Parses `#[check(F1, P1-P4)]` into its expanded id list.
fn tag_ids(attr: &Attribute) -> syn::Result<Vec<String>> {
    let items = attr.parse_args_with(Punctuated::<IdItem, Token![,]>::parse_terminated)?;
    let mut ids = Vec::new();
    for item in items {
        for id in item.ids {
            if ids.contains(&id) {
                return Err(syn::Error::new(
                    attr.span(),
                    format!("check `{id}` is listed twice in this tag"),
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
        if let ImplItem::Fn(method) = item {
            match method_entry(method) {
                Ok(Some(entry)) => entries.push(entry),
                Ok(None) => {},
                Err(err) => return err.to_compile_error().into(),
            }
        }
    }
    quote! {
        #item_impl

        /// What each method implements, from its `#[check(...)]` tags and
        /// `#[complexity(...)]` declaration.
        pub const #registry: &[::mono_move_spec::MethodChecks] = &[#(#entries),*];
    }
    .into()
}

/// Strips a method's `#[check(...)]` tags and `#[complexity(...)]`
/// declaration, returning its `MethodChecks` entry if it has either.
fn method_entry(method: &mut syn::ImplItemFn) -> syn::Result<Option<proc_macro2::TokenStream>> {
    let mut body = BodyTags {
        ids: Vec::new(),
        error: None,
    };
    body.visit_block_mut(&mut method.block);
    if let Some(err) = body.error {
        return Err(err);
    }
    let mut ids = body.ids;
    ids.sort();
    ids.dedup();

    let (complexity_attrs, rest): (Vec<_>, Vec<_>) = method
        .attrs
        .drain(..)
        .partition(|attr| attr.path().is_ident("complexity"));
    method.attrs = rest;
    let complexity = match complexity_attrs.as_slice() {
        [] => None,
        [attr] => Some(attr.parse_args::<ComplexityArgs>()?),
        [_, second, ..] => {
            return Err(syn::Error::new(
                second.span(),
                "a method declares its complexity once",
            ))
        },
    };
    if ids.is_empty() && complexity.is_none() {
        return Ok(None);
    }

    let complexity_expr = match &complexity {
        Some(args) => {
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
            let variant = Ident::new(args.variant, proc_macro2::Span::call_site());
            let (measured_in, because) = (&args.measured_in, &args.because);
            quote! {
                ::core::option::Option::Some(::mono_move_spec::Complexity {
                    class: ::mono_move_spec::ComplexityClass::#variant,
                    measured_in: #measured_in,
                    because: #because,
                })
            }
        },
        None => quote! { ::core::option::Option::None },
    };
    let name = method.sig.ident.to_string();
    Ok(Some(quote! {
        ::mono_move_spec::MethodChecks {
            method: #name,
            checks: &[#(#ids),*],
            complexity: #complexity_expr,
        }
    }))
}

// ---------------------------------------------------------------------------
// #[spec]: the check tables in an item's doc comments, as data
// ---------------------------------------------------------------------------

/// `#[spec]` on an item whose documentation holds the check tables parses
/// every markdown table with an `Id` column and emits
///
/// ```ignore
/// impl Spec {
///     pub const CHECKS: &'static [CheckSpec] = &[
///         CheckSpec { id: "F1", group: "Function shape", property: "...", condition: "...", rationale: "..." },
///         ...
///     ];
/// }
/// ```
///
/// where the group is the nearest preceding `## ` heading. The item's
/// documentation is left unchanged, so rustdoc still renders the tables. A
/// table must have `Id`, `Property`, and `Condition` columns and may have
/// `Rationale`; every row must have as many cells as the header; ids must be
/// well-formed and unique across all tables. Each violation is a compile
/// error pointing at the item.
#[proc_macro_attribute]
pub fn spec(_args: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as Item);
    let (ident, attrs) = match &item {
        Item::Struct(s) => (&s.ident, &s.attrs),
        Item::Enum(e) => (&e.ident, &e.attrs),
        Item::Mod(m) => (&m.ident, &m.attrs),
        _ => {
            return syn::Error::new(item.span(), "#[spec] goes on a struct, enum, or module")
                .to_compile_error()
                .into()
        },
    };
    let doc_lines: Vec<String> = attrs
        .iter()
        .filter_map(|attr| match &attr.meta {
            Meta::NameValue(nv) if nv.path.is_ident("doc") => match &nv.value {
                Expr::Lit(ExprLit {
                    lit: Lit::Str(text),
                    ..
                }) => Some(text.value()),
                _ => None,
            },
            _ => None,
        })
        .collect();
    let rows = match parse_tables(&doc_lines) {
        Ok(rows) => rows,
        Err(msg) => return syn::Error::new(ident.span(), msg).to_compile_error().into(),
    };
    let entries = rows.iter().map(|r| {
        let (id, group, property, condition, rationale) =
            (&r.id, &r.group, &r.property, &r.condition, &r.rationale);
        quote! {
            ::mono_move_spec::CheckSpec {
                id: #id,
                group: #group,
                property: #property,
                condition: #condition,
                rationale: #rationale,
            }
        }
    });
    quote! {
        #item

        impl #ident {
            /// Every check in the tables above, in order.
            pub const CHECKS: &'static [::mono_move_spec::CheckSpec] = &[#(#entries),*];
        }
    }
    .into()
}

struct SpecRow {
    id: String,
    group: String,
    property: String,
    condition: String,
    rationale: String,
}

static GROUP_HEADING: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^## (.+)$").unwrap());
static TABLE_ROW: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\|(.*)\|$").unwrap());
static SEPARATOR_ROW: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\|(\s*:?-+:?\s*\|)+$").unwrap());
static CELL_SPLIT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s*\|\s*").unwrap());

/// Parses every table whose header starts with an `Id` column.
fn parse_tables(lines: &[String]) -> Result<Vec<SpecRow>, String> {
    let mut rows = Vec::new();
    let mut group = String::new();
    let mut header: Option<Vec<String>> = None;
    for raw in lines {
        let line = raw.trim();
        if let Some(h) = GROUP_HEADING.captures(line) {
            group = h[1].trim().to_string();
            header = None;
            continue;
        }
        let Some(row) = TABLE_ROW.captures(line) else {
            header = None;
            continue;
        };
        if SEPARATOR_ROW.is_match(line) {
            continue;
        }
        let cells: Vec<String> = CELL_SPLIT
            .split(row[1].trim())
            .map(str::to_string)
            .collect();
        match &header {
            None => {
                if cells.first().map(String::as_str) == Some("Id") {
                    for required in ["Property", "Condition"] {
                        if !cells.iter().any(|c| c == required) {
                            return Err(format!(
                                "table in group `{group}` lacks a `{required}` column"
                            ));
                        }
                    }
                    header = Some(cells);
                }
            },
            Some(h) => {
                if cells.len() != h.len() {
                    return Err(format!(
                        "row `{}` has {} cells but its table has {} columns",
                        cells[0],
                        cells.len(),
                        h.len()
                    ));
                }
                let col = |name: &str| h.iter().position(|x| x == name).map(|i| cells[i].clone());
                let id = cells[0].clone();
                if parse_check_id(&id).is_none() {
                    return Err(format!(
                        "`{id}` is not a check id (an uppercase letter followed by digits)"
                    ));
                }
                if rows.iter().any(|r: &SpecRow| r.id == id) {
                    return Err(format!("check `{id}` is specified twice"));
                }
                rows.push(SpecRow {
                    id,
                    group: group.clone(),
                    property: col("Property").unwrap_or_default(),
                    condition: col("Condition").unwrap_or_default(),
                    rationale: col("Rationale").unwrap_or_default(),
                });
            },
        }
    }
    if rows.is_empty() {
        return Err("no check table found in the documentation".to_string());
    }
    Ok(rows)
}
