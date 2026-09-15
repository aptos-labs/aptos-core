// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Registry of script-callable functions.
//!
//! Collects `public` function declarations (parameters, returns, generics, and
//! the `entry` flag) from compiled modules, consulting the module source when
//! available to drop `public(package)` and `public(friend)` functions.

use crate::{
    deps::PkgKind,
    prep::{datatype::DatatypeRegistry, ident::FunctionIdent, typing::TypeRef},
};
use log::debug;
use move_binary_format::{
    binary_views::BinaryIndexedView, file_format::Visibility, CompiledModule,
};
use move_core_types::ability::{Ability, AbilitySet};
use std::collections::{BTreeMap, BTreeSet};

/// Declaration of a function
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct FunctionDecl {
    pub ident: FunctionIdent,
    pub generics: Vec<AbilitySet>,
    pub parameters: Vec<TypeRef>,
    pub return_sig: Vec<TypeRef>,
    pub kind: PkgKind,
    pub is_entry: bool,
}

impl FunctionDecl {
    /// The ability constraints to use when enumerating type arguments for this
    /// declaration.
    ///
    /// This is the declared `generics` list, with `key` added for every type
    /// parameter that appears as the argument of an `Object<_>` in a parameter
    /// or return type.
    ///
    /// `object::Object<phantom T>` declares no constraint on `T`, so
    /// `fun f<T>(o: Object<T>)` is legal Move. It is nevertheless uncallable
    /// unless `T` has `key`: every framework constructor of an `Object<T>`
    /// value (`address_to_object`, `object_from_constructor_ref`,
    /// `object_from_delete_ref`) is declared `<T: key>` and the `inner` field is
    /// private to `0x1::object`, so for a non-`key` `T` the type has no values
    /// at all. Enumerating those instantiations would spend the budget
    /// generating drivers whose arguments can never be produced.
    pub fn effective_generics(&self) -> Vec<AbilitySet> {
        let mut object_params = BTreeSet::new();
        for ty in self.parameters.iter().chain(self.return_sig.iter()) {
            ty.base().object_type_params(&mut object_params);
        }
        self.generics
            .iter()
            .enumerate()
            .map(|(index, constraint)| {
                if object_params.contains(&index) {
                    constraint.add(Ability::Key)
                } else {
                    *constraint
                }
            })
            .collect()
    }
}

pub struct FunctionRegistry {
    decls: BTreeMap<FunctionIdent, FunctionDecl>,
}

impl Default for FunctionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl FunctionRegistry {
    /// Create an empty registry
    pub fn new() -> Self {
        Self {
            decls: BTreeMap::new(),
        }
    }

    /// Analyze a module and register script-callable functions found in this module.
    ///
    /// We register only externally callable functions (`public` visibility).
    /// `entry` metadata is retained for prioritization.
    pub fn analyze(
        &mut self,
        typing: &mut DatatypeRegistry,
        module: &CompiledModule,
        kind: PkgKind,
        source_text: Option<&str>,
    ) {
        let binary = BinaryIndexedView::Module(module);
        let script_public_funs = source_text.map(parse_script_public_functions);

        // go over all functions defined
        for def in &module.function_defs {
            if !matches!(def.visibility, Visibility::Public) {
                continue;
            }

            let handle = binary.function_handle_at(def.function);
            let ident = FunctionIdent::from_function_handle(&binary, handle);
            if let Some(public_funs) = &script_public_funs {
                if !public_funs.contains(ident.function_name()) {
                    continue;
                }
            }

            // parse parameters and return types; a signature the registry cannot
            // model (e.g. `Object<u64>`) excludes the function instead of
            // aborting the whole run
            let mut parameters = vec![];
            let mut unsupported = false;
            for token in &binary.signature_at(handle.parameters).0 {
                match typing.convert_signature_token(&binary, token) {
                    Some(ty) => parameters.push(ty),
                    None => {
                        unsupported = true;
                        break;
                    },
                }
            }
            let mut return_sig = vec![];
            if !unsupported {
                for token in &binary.signature_at(handle.return_).0 {
                    match typing.convert_signature_token(&binary, token) {
                        Some(ty) => return_sig.push(ty),
                        None => {
                            unsupported = true;
                            break;
                        },
                    }
                }
            }
            if unsupported {
                debug!("skipping {ident}: signature has a type move-fuzz cannot model");
                continue;
            }

            // add the declaration
            let decl = FunctionDecl {
                ident: ident.clone(),
                generics: handle.type_parameters.clone(),
                parameters,
                return_sig,
                kind,
                is_entry: def.is_entry,
            };
            match self.decls.get_mut(&ident) {
                None => {
                    self.decls.insert(ident, decl);
                },
                Some(existing) => {
                    if existing.generics == decl.generics
                        && existing.parameters == decl.parameters
                        && existing.return_sig == decl.return_sig
                    {
                        existing.kind = merge_pkg_kind(existing.kind, decl.kind);
                        existing.is_entry |= decl.is_entry;
                    } else {
                        panic!("duplicate function declaration {}", ident);
                    }
                },
            }
        }
    }

    /// Lookup a function declaration
    pub fn lookup_decl(&self, ident: &FunctionIdent) -> &FunctionDecl {
        self.decls
            .get(ident)
            .unwrap_or_else(|| panic!("unregistered function {ident}"))
    }

    /// Return an iterator for all declarations collected
    pub fn iter_decls(&self) -> impl Iterator<Item = &FunctionDecl> {
        self.decls.values()
    }
}

#[cfg(test)]
impl FunctionRegistry {
    pub(crate) fn insert_for_test(&mut self, decl: FunctionDecl) {
        let ident = decl.ident.clone();
        let existing = self.decls.insert(ident.clone(), decl);
        assert!(
            existing.is_none(),
            "duplicate test function declaration {ident}"
        );
    }
}

fn merge_pkg_kind(existing: PkgKind, incoming: PkgKind) -> PkgKind {
    match (existing, incoming) {
        (PkgKind::Primary, _) | (_, PkgKind::Primary) => PkgKind::Primary,
        (PkgKind::Dependency, _) | (_, PkgKind::Dependency) => PkgKind::Dependency,
        (PkgKind::Framework, PkgKind::Framework) => PkgKind::Framework,
    }
}

fn parse_script_public_functions(source: &str) -> BTreeSet<String> {
    #[derive(Clone, Copy)]
    enum Token<'a> {
        Ident(&'a str),
        Symbol(char),
    }

    fn is_ident_start(ch: char) -> bool {
        ch.is_ascii_alphabetic() || ch == '_'
    }

    fn is_ident_continue(ch: char) -> bool {
        ch.is_ascii_alphanumeric() || ch == '_'
    }

    fn tokenize(source: &str) -> Vec<Token<'_>> {
        let mut tokens = Vec::new();
        let bytes = source.as_bytes();
        let mut idx = 0;
        let mut block_comment_depth = 0usize;
        while idx < bytes.len() {
            let ch = bytes[idx] as char;

            if block_comment_depth > 0 {
                if ch == '/' && idx + 1 < bytes.len() && bytes[idx + 1] as char == '*' {
                    block_comment_depth += 1;
                    idx += 2;
                    continue;
                }
                if ch == '*' && idx + 1 < bytes.len() && bytes[idx + 1] as char == '/' {
                    block_comment_depth -= 1;
                    idx += 2;
                    continue;
                }
                idx += 1;
                continue;
            }

            if ch.is_ascii_whitespace() {
                idx += 1;
                continue;
            }
            if ch == '/' && idx + 1 < bytes.len() {
                let next = bytes[idx + 1] as char;
                if next == '/' {
                    idx += 2;
                    while idx < bytes.len() && bytes[idx] as char != '\n' {
                        idx += 1;
                    }
                    continue;
                }
                if next == '*' {
                    block_comment_depth = 1;
                    idx += 2;
                    continue;
                }
            }
            if is_ident_start(ch) {
                let start = idx;
                idx += 1;
                while idx < bytes.len() && is_ident_continue(bytes[idx] as char) {
                    idx += 1;
                }
                tokens.push(Token::Ident(&source[start..idx]));
                continue;
            }
            tokens.push(Token::Symbol(ch));
            idx += 1;
        }
        tokens
    }

    fn token_is_ident(token: Option<&Token<'_>>, ident: &str) -> bool {
        matches!(token, Some(Token::Ident(found)) if *found == ident)
    }

    let mut result = BTreeSet::new();
    let tokens = tokenize(source);
    let mut idx = 0;
    while idx < tokens.len() {
        if !token_is_ident(tokens.get(idx), "public") {
            idx += 1;
            continue;
        }

        let mut cursor = idx + 1;
        if matches!(tokens.get(cursor), Some(Token::Symbol('('))) {
            idx += 1;
            continue;
        }
        while token_is_ident(tokens.get(cursor), "entry")
            || token_is_ident(tokens.get(cursor), "native")
        {
            cursor += 1;
        }
        if token_is_ident(tokens.get(cursor), "fun")
            && let Some(Token::Ident(name)) = tokens.get(cursor + 1)
        {
            result.insert((*name).to_string());
        }
        idx += 1;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::{parse_script_public_functions, FunctionDecl};
    use crate::{
        deps::PkgKind,
        prep::{ident::FunctionIdent, typing::TypeRef},
    };
    use move_core_types::{
        ability::{Ability, AbilitySet},
        account_address::AccountAddress,
        identifier::Identifier,
    };
    use std::collections::BTreeSet;

    #[test]
    fn test_parse_script_public_functions_handles_multiline_decls() {
        let source = r#"
            module 0x1::m {
                public
                entry
                fun launch() {}
            }
        "#;
        assert_eq!(
            parse_script_public_functions(source),
            BTreeSet::from(["launch".to_string()])
        );
    }

    #[test]
    fn test_parse_script_public_functions_skips_package_visibility() {
        let source = r#"
            module 0x1::m {
                public(package) fun hidden() {}
                public(friend) fun also_hidden() {}
                public fun visible() {}
            }
        "#;
        assert_eq!(
            parse_script_public_functions(source),
            BTreeSet::from(["visible".to_string()])
        );
    }

    #[test]
    fn test_effective_generics_infers_key_for_object_type_params() {
        use crate::prep::typing::TypeExpr;

        let decl = |parameters: Vec<TypeRef>, return_sig: Vec<TypeRef>| FunctionDecl {
            ident: FunctionIdent::from_function_tuple(
                AccountAddress::ONE,
                Identifier::new("m").unwrap(),
                Identifier::new("f").unwrap(),
            ),
            generics: vec![AbilitySet::EMPTY, AbilitySet::EMPTY],
            parameters,
            return_sig,
            kind: PkgKind::Primary,
            is_entry: true,
        };

        // `fun f<T0, T1>(o: Object<T0>, x: T1)`: only T0 is used as an object,
        // and `Object<T>` has no values unless `T` has `key`.
        let d = decl(
            vec![
                TypeRef::Base(TypeExpr::ObjectParam(0)),
                TypeRef::Base(TypeExpr::Param(1)),
            ],
            vec![],
        );
        let effective = d.effective_generics();
        assert!(effective[0].has_key());
        assert!(!effective[1].has_key());

        // the declared constraint is preserved, not replaced
        let mut d = decl(vec![TypeRef::Base(TypeExpr::ObjectParam(0))], vec![]);
        d.generics = vec![AbilitySet::EMPTY.add(Ability::Drop), AbilitySet::EMPTY];
        let effective = d.effective_generics();
        assert!(effective[0].has_key() && effective[0].has_drop());

        // nesting counts too: `vector<Object<T0>>` and a return position
        let d = decl(
            vec![TypeRef::Base(TypeExpr::Vector {
                element: Box::new(TypeExpr::ObjectParam(0)),
            })],
            vec![TypeRef::Base(TypeExpr::ObjectParam(1))],
        );
        let effective = d.effective_generics();
        assert!(effective[0].has_key());
        assert!(effective[1].has_key());

        // a bare type parameter is untouched
        let d = decl(vec![TypeRef::Base(TypeExpr::Param(0))], vec![]);
        assert!(!d.effective_generics()[0].has_key());
    }
}
