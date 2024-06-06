// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Manages identifiers and scope information during generation.

use arbitrary::Arbitrary;
use std::collections::HashMap;

/// Represents a Move identifier.
/// Key invariant: each identifier is globally unique.
/// This is achieved by appending a monotonic counter to the identifier name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Identifier(pub String);

impl Identifier {
    /// Convert the identifier to a scope.
    pub fn to_scope(&self) -> Scope {
        Scope(Some(self.0.clone()))
    }
}

/// Scope is the namespace where a variable can be accessed.
/// None: represents the root scope.
/// Some(scope): the scope must have the format "parent::child".
/// e.g. "Module1::function1"
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Scope(pub Option<String>);

/// Represents the root scope.
pub const ROOT_SCOPE: Scope = Scope(None);

/// Merge two scopes treating the first scope as the parent and the second scope as the child.
pub fn merge_scopes(parent: &Scope, child: &Scope) -> Scope {
    Scope(match (&parent.0, &child.0) {
        (Some(p), Some(c)) => Some(format!("{}::{}", p, c)),
        (Some(p), None) => Some(p.clone()),
        (None, Some(c)) => Some(c.clone()),
        (None, None) => None,
    })
}

/// Checks if the child scope is in the parent scope.
pub fn is_in_scope(child: &Scope, parent: &Scope) -> bool {
    match (&child.0, &parent.0) {
        (Some(c), Some(p)) => c == p || c.starts_with(&format!("{}::", p)),
        (Some(_), None) => true,
        (None, Some(_)) => false,
        (None, None) => true,
    }
}

/// Keeps track of all used identifiers and the scope information.
/// Each different kind of identifier (var, struct, function, etc.) has its own counter.
/// The `scopes` map keeps track of the scope information for each identifier.
/// Key invariant: each scope should be complete, meaning no chasing should be needed.
#[derive(Debug)]
pub struct IdentifierPool {
    vars: Vec<Identifier>,
    structs: Vec<Identifier>,
    functions: Vec<Identifier>,
    modules: Vec<Identifier>,
    scripts: Vec<Identifier>,
    constants: Vec<Identifier>,
    blocks: Vec<Identifier>,
    scopes: HashMap<Identifier, Scope>,
}

/// The types of identifiers.
#[derive(Debug, Clone, Arbitrary)]
pub enum IdentifierType {
    Var,
    Struct,
    Function,
    Module,
    Script,
    Constant,

    // Block identifiers are only used to keep track of scope.
    Block,
}

impl Default for IdentifierPool {
    fn default() -> Self {
        Self::new()
    }
}

impl IdentifierPool {
    /// Initialize an empty identifier pool.
    pub fn new() -> Self {
        Self {
            vars: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            modules: Vec::new(),
            scripts: Vec::new(),
            constants: Vec::new(),
            blocks: Vec::new(),
            scopes: HashMap::new(),
        }
    }

    /// Creates a new identifier under the given scope.
    /// Returns the scope that this new identifier owns.
    ///
    /// For example, to create a new function under a module `Module1`,
    /// `next_identifier(..., Module1)` will return (function1, Module1::function1)
    /// Then to create a new local variable in function1,
    /// the call `next_identifier(..., Module1::function1)` should be used.
    /// This should be followed during generation to maintain the scope hierarchy.
    // TODO: add extra check for the completeness of the scope
    pub fn next_identifier(&mut self, typ: IdentifierType, scope: &Scope) -> (Identifier, Scope) {
        let cnt = self.identifier_count(&typ);
        let name = self.construct_name(&typ, cnt);
        self.insert_new_identifier(&typ, Identifier(name.clone()));
        self.scopes.insert(Identifier(name.clone()), scope.clone());
        let child_scope = Scope(Some(name.clone()));
        let scope = merge_scopes(scope, &child_scope);
        (Identifier(name), scope)
    }

    /// Get the scope where the given identifier is accessible.
    pub fn get_parent_scope_of(&self, id: &Identifier) -> Option<Scope> {
        self.scopes.get(id).cloned()
    }

    /// Get the scope where the children of the given identifier are accessible.
    pub fn get_scope_for_children(&self, id: &Identifier) -> Scope {
        match self.scopes.get(id) {
            Some(scope) => merge_scopes(scope, &id.to_scope()),
            None => id.to_scope(),
        }
    }

    /// Get the flattened access for an identifier used for script generation.
    pub fn flatten_access(&self, id: &Identifier) -> Option<Identifier> {
        match self.get_scope_for_children(id) {
            Scope(Some(scope)) => Some(Identifier(scope)),
            Scope(None) => None,
        }
    }

    /// Helper function to filter identifiers that are in the given scope.
    pub fn filter_identifier_in_scope(
        &self,
        identifiers: &Vec<Identifier>,
        parent_scope: &Scope,
    ) -> Vec<Identifier> {
        let mut in_scope = Vec::new();
        for id in identifiers {
            let id_scope = self.scopes.get(id).unwrap_or(&ROOT_SCOPE);
            if is_in_scope(id_scope, parent_scope) {
                in_scope.push(id.clone());
            }
        }
        in_scope
    }

    /// Returns all identifiers in use.
    pub fn get_all_identifiers(&self) -> Vec<Identifier> {
        self.scopes.keys().cloned().collect()
    }

    /// Returns all identifiers of the given identifier type.
    /// e.g. get all function identifiers.
    pub fn get_identifiers_of_ident_type(&self, typ: IdentifierType) -> Vec<Identifier> {
        self._get_identifiers_of_ident_type(typ).clone()
    }

    /// Returns the number of identifiers of the given identifier type.
    fn _get_identifiers_of_ident_type(&self, typ: IdentifierType) -> &Vec<Identifier> {
        match typ {
            IdentifierType::Var => &self.vars,
            IdentifierType::Struct => &self.structs,
            IdentifierType::Function => &self.functions,
            IdentifierType::Module => &self.modules,
            IdentifierType::Script => &self.scripts,
            IdentifierType::Constant => &self.constants,
            IdentifierType::Block => &self.blocks,
        }
    }

    /// Add a new identifier to the pool.
    fn insert_new_identifier(&mut self, typ: &IdentifierType, name: Identifier) {
        match typ {
            IdentifierType::Var => self.vars.push(name),
            IdentifierType::Struct => self.structs.push(name),
            IdentifierType::Function => self.functions.push(name),
            IdentifierType::Module => self.modules.push(name),
            IdentifierType::Script => self.scripts.push(name),
            IdentifierType::Constant => self.constants.push(name),
            IdentifierType::Block => self.blocks.push(name),
        }
    }

    /// Get the count of identifiers of the given type.
    fn identifier_count(&self, typ: &IdentifierType) -> usize {
        match typ {
            IdentifierType::Var => self.vars.len(),
            IdentifierType::Struct => self.structs.len(),
            IdentifierType::Function => self.functions.len(),
            IdentifierType::Module => self.modules.len(),
            IdentifierType::Script => self.scripts.len(),
            IdentifierType::Constant => self.constants.len(),
            IdentifierType::Block => self.blocks.len(),
        }
    }

    /// Create the name of an identifier.
    fn construct_name(&self, typ: &IdentifierType, idx: usize) -> String {
        let type_prefix = match typ {
            IdentifierType::Var => "var",
            IdentifierType::Struct => "Struct",
            IdentifierType::Function => "function",
            IdentifierType::Module => "Module",
            IdentifierType::Script => "Script",
            IdentifierType::Constant => "CONST",
            IdentifierType::Block => "_block",
        };
        format!("{}{}", type_prefix, idx)
    }
}
