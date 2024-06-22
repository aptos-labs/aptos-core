// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Manages identifiers and scope information during generation.

use arbitrary::Arbitrary;
use std::collections::HashMap;

/// Represents a Move identifier.
/// Key invariant: each identifier is globally unique.
/// This is achieved by appending a monotonic counter to the identifier name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Identifier {
    pub name: String,
    pub kind: IdentifierKind,
}

impl Identifier {
    pub fn new(name: String, kind: IdentifierKind) -> Self {
        Self { name, kind }
    }

    pub fn new_str(name: &str, kind: IdentifierKind) -> Self {
        Self {
            name: name.to_string(),
            kind,
        }
    }

    /// Convert the identifier to a scope.
    pub fn to_scope(&self) -> Scope {
        Scope(Some(self.name.clone()))
    }

    pub fn is_var(&self) -> bool {
        self.kind == IdentifierKind::Var
    }
}

/// The types of identifiers.
#[derive(Debug, Clone, Arbitrary, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum IdentifierKind {
    Var,
    Struct,
    Function,
    Module,
    Script,
    Constant,
    Type,
    TypeParameter,

    // Block identifiers are only used to keep track of scope.
    Block,
}

impl IdentifierKind {
    pub fn from_name(name: &str) -> Self {
        match name {
            _ if name.starts_with("var") => IdentifierKind::Var,
            _ if name.starts_with("Struct") => IdentifierKind::Struct,
            _ if name.starts_with("function") => IdentifierKind::Function,
            _ if name.starts_with("Module") => IdentifierKind::Module,
            _ if name.starts_with("Script") => IdentifierKind::Script,
            _ if name.starts_with("CONST") => IdentifierKind::Constant,
            _ if name.starts_with("_type") => IdentifierKind::Type,
            _ if name.starts_with('T') => IdentifierKind::TypeParameter,
            _ if name.starts_with("_block") => IdentifierKind::Block,
            _ => panic!("Unknown identifier kind: {}", name),
        }
    }

    pub fn get_kind_name(&self) -> String {
        match self {
            IdentifierKind::Var => "var",
            IdentifierKind::Struct => "Struct",
            IdentifierKind::Function => "function",
            IdentifierKind::Module => "Module",
            IdentifierKind::Script => "Script",
            IdentifierKind::Constant => "Constant",
            IdentifierKind::Type => "_type",
            IdentifierKind::TypeParameter => "T",
            IdentifierKind::Block => "_block",
        }
        .to_string()
    }
}

/// Scope is the namespace where a variable can be accessed.
/// None: represents the root scope.
/// Some(scope): the scope must have the format "parent::child".
/// e.g. "Module1::function1"
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Scope(pub Option<String>);

impl Scope {
    /// Return if the scope is the root scope.
    pub fn is_root(&self) -> bool {
        self.0.is_none()
    }

    pub fn get_name(&self) -> String {
        self.0.clone().unwrap_or("".to_string())
    }

    /// Convert the scope to an identifier.
    pub fn to_identifier(&self) -> Option<Identifier> {
        self.0.as_ref()?;
        let name = self.get_name();
        let pieces = self.to_pieces();
        let kind = IdentifierKind::from_name(pieces.last().unwrap());
        Some(Identifier { name, kind })
    }

    /// Remove all hidden scopes whose name starts with an underscore
    /// e.g. `Module1::function1::_block1::_block2` will result in `Module1::function1`
    pub fn remove_hidden_scopes(&self) -> Scope {
        if self.is_root() {
            return self.clone();
        }

        let pieces = self.to_pieces();
        let new_scope = pieces
            .into_iter()
            .filter(|s| !s.starts_with('_'))
            .map(String::from)
            .collect::<Vec<String>>()
            .join("::");
        Scope(Some(new_scope))
    }

    /// Split the scope into individual pieces.
    /// e.g., `Module1::function1::_block1::_block2` will result in
    /// `["Module1", "function1", "_block1", "_block2"]`
    pub fn to_pieces(&self) -> Vec<String> {
        match &self.0 {
            Some(name) => name.split("::").map(String::from).collect(),
            None => vec![],
        }
    }

    /// Get all parent scopes of the current scope, including self
    pub fn ancestors(&self) -> Vec<Scope> {
        let pieces = self.to_pieces();
        let mut parents = vec![ROOT_SCOPE.clone()];
        for i in 1..pieces.len() {
            let parent = pieces[0..i].join("::");
            parents.push(Scope(Some(parent)));
        }
        parents.push(self.clone());
        parents
    }
}

/// Represents the root scope.
pub const ROOT_SCOPE: Scope = Scope(None);

/// Keeps track of all used identifiers and the scope information.
/// Each different kind of identifier (var, struct, function, etc.) has its own counter.
/// The `scopes` map keeps track of the scope information for each identifier.
/// Key invariant: each scope should be complete, meaning no chasing should be needed.
#[derive(Debug)]
pub struct IdentifierPool {
    all_ids: Vec<Identifier>,
    counters: HashMap<IdentifierKind, usize>,
    scopes: HashMap<Identifier, Scope>,
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
            all_ids: Vec::new(),
            counters: HashMap::new(),
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
    pub fn next_identifier(&mut self, typ: IdentifierKind, scope: &Scope) -> (Identifier, Scope) {
        let cnt = self.identifier_count(&typ);
        let name = self.construct_name(&typ, cnt);
        let new_id = Identifier {
            name: name.clone(),
            kind: typ.clone(),
        };

        self.insert_new_identifier(&typ, new_id.clone());

        self.scopes.insert(new_id.clone(), scope.clone());

        let child_scope = Scope(Some(name.clone()));
        let new_scope: Scope = self.merge_scopes(scope, &child_scope);

        (new_id, new_scope)
    }

    /// Get the outter most scope where the given identifier is accessible.
    pub fn get_parent_scope_of(&self, id: &Identifier) -> Option<Scope> {
        self.scopes.get(id).cloned()
    }

    /// Get the scope where the children of the given identifier are accessible.
    pub fn get_scope_for_children(&self, id: &Identifier) -> Scope {
        match self.scopes.get(id) {
            Some(scope) => self.merge_scopes(scope, &id.to_scope()),
            None => id.to_scope(),
        }
    }

    /// Get the flattened access for an identifier used for script generation.
    // TODO: this currently contains _block scopes, which might cause trouble.
    pub fn flatten_access(&self, id: &Identifier) -> Identifier {
        match self.scopes.get(id) {
            Some(scope) => self
                .merge_scopes(scope, &id.to_scope())
                .to_identifier()
                .unwrap(),
            None => id.clone(),
        }
    }

    /// Check if an identifier is accessible in the given scope.
    pub fn is_id_in_scope(&self, id: &Identifier, scope: &Scope) -> bool {
        // let flat_id = self.flatten_access(id);
        let parent_of_id = self.get_parent_scope_of(id);
        match parent_of_id {
            Some(parent) => self.is_in_scope(scope, &parent),
            None => true,
        }
    }

    /// Check if an Identifier is accessible within another Identifier.
    /// The parent identifier should be function, block, struct, etc.
    pub fn is_id_in_id(&self, child: &Identifier, parent: &Identifier) -> bool {
        let parent_scope = self.get_parent_scope_of(parent).unwrap();
        self.is_id_in_scope(child, &parent_scope)
    }

    /// Returns whether child is the same as or within parent
    /// e.g. (M1::F1::B1::B2, M1::F1) ==> true
    fn is_in_scope(&self, child: &Scope, parent: &Scope) -> bool {
        match (&child.0, &parent.0) {
            (Some(c), Some(p)) => c == p || c.starts_with(&format!("{}::", p)),
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (None, None) => true,
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
            if self.is_id_in_scope(id, parent_scope) {
                in_scope.push(id.clone());
            }
        }
        in_scope
    }

    /// Returns all identifiers in use.
    pub fn get_all_identifiers(&self) -> Vec<Identifier> {
        self.scopes.keys().cloned().collect()
    }

    /// Returns all identifiers of the given identifier kind.
    /// e.g. get all function identifiers.
    pub fn get_identifiers_of_ident_kind(&self, typ: IdentifierKind) -> Vec<Identifier> {
        self.all_ids
            .iter()
            .filter(|id| id.kind == typ)
            .cloned()
            .collect()
    }

    /// Add a new identifier to the pool.
    fn insert_new_identifier(&mut self, typ: &IdentifierKind, id: Identifier) {
        self.counters
            .entry(typ.clone())
            .and_modify(|e| *e += 1)
            .or_insert(1);
        self.all_ids.push(id);
    }

    /// Get the count of identifiers of the given type.
    fn identifier_count(&self, typ: &IdentifierKind) -> usize {
        return self.counters.get(typ).cloned().unwrap_or(0);
    }

    /// Create the name of an identifier.
    fn construct_name(&self, typ: &IdentifierKind, idx: usize) -> String {
        format!("{}{}", typ.get_kind_name(), idx)
    }

    fn merge_scopes(&self, parent: &Scope, child: &Scope) -> Scope {
        Scope(match (&parent.0, &child.0) {
            (Some(p), Some(c)) => Some(format!("{}::{}", p, c)),
            (Some(p), None) => Some(p.clone()),
            (None, Some(c)) => Some(c.clone()),
            (None, None) => None,
        })
    }
}

#[test]
fn test_scope() {
    let scope = Scope(Some("Module1::function1::_block1::_block2".to_string()));
    let pieces = scope.to_pieces();
    assert_eq!(pieces, vec!["Module1", "function1", "_block1", "_block2"]);
    let ans = scope.ancestors();
    assert_eq!(ans.len(), 5);
}
