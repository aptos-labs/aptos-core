// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Provides macros for defining a spec sheet, and binding spec items to
//! their implementation.
//!
//! - `#[spec]` on an item turns the markdown check tables in its doc comment
//!   into `Self::CHECKS: &[CheckSpec]`, validating them at compile time.
//! - `#[check(R1)]` tags the statement or match arm that implements a check.
//! - `#[checks(registry = NAME)]` on an `impl` collects the tags into a
//!   registry const.
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
        // Tags inside the body: `#[check(R1)]` on a statement or match arm.
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
            CheckSpec {
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
            pub const CHECKS: &'static [CheckSpec] = &[#(#entries),*];
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

/// Splits a markdown table row into trimmed cells.
fn cells(line: &str) -> Vec<String> {
    let inner = line.trim().trim_start_matches('|').trim_end_matches('|');
    inner.split('|').map(|c| c.trim().to_string()).collect()
}

fn is_check_id(id: &str) -> bool {
    let mut chars = id.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_uppercase())
        && !chars.as_str().is_empty()
        && chars.all(|c| c.is_ascii_digit())
}

/// Parses every table whose header starts with an `Id` column.
fn parse_tables(lines: &[String]) -> Result<Vec<SpecRow>, String> {
    let mut rows = Vec::new();
    let mut group = String::new();
    let mut header: Option<Vec<String>> = None;
    for raw in lines {
        let line = raw.trim();
        if let Some(h) = line.strip_prefix("## ") {
            group = h.trim().to_string();
            header = None;
            continue;
        }
        if !line.starts_with('|') {
            header = None;
            continue;
        }
        let c = cells(line);
        if c.iter().all(|x| x.chars().all(|ch| ch == '-')) {
            continue; // separator row
        }
        match &header {
            None => {
                if c.first().map(String::as_str) == Some("Id") {
                    for required in ["Property", "Condition"] {
                        if !c.iter().any(|x| x == required) {
                            return Err(format!(
                                "table in group `{group}` lacks a `{required}` column"
                            ));
                        }
                    }
                    header = Some(c);
                }
            },
            Some(h) => {
                if c.len() != h.len() {
                    return Err(format!(
                        "row `{}` has {} cells but its table has {} columns",
                        c.first().cloned().unwrap_or_default(),
                        c.len(),
                        h.len()
                    ));
                }
                let col = |name: &str| h.iter().position(|x| x == name).map(|i| c[i].clone());
                let id = c[0].clone();
                if !is_check_id(&id) {
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
