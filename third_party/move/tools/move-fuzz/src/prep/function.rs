// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    deps::PkgKind,
    prep::{datatype::DatatypeRegistry, ident::FunctionIdent, typing::TypeRef},
};
use move_binary_format::{
    binary_views::BinaryIndexedView, file_format::Visibility, CompiledModule,
};
use move_core_types::ability::AbilitySet;
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

pub struct FunctionRegistry {
    decls: BTreeMap<FunctionIdent, FunctionDecl>,
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

            // parse parameters and return types
            let mut parameters = vec![];
            for token in &binary.signature_at(handle.parameters).0 {
                parameters.push(typing.convert_signature_token(&binary, token));
            }
            let mut return_sig = vec![];
            for token in &binary.signature_at(handle.return_).0 {
                return_sig.push(typing.convert_signature_token(&binary, token));
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
        _ => PkgKind::Framework,
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
    use super::parse_script_public_functions;
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
}
