// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Custom data structures for Move types.
//! Manages typing information during generation.

use crate::names::Identifier;
use std::collections::BTreeMap;

/// Collection of Move types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    // Basic types
    U8,
    U16,
    U32,
    U64,
    U128,
    U256,
    Bool,
    Address,
    Signer,
    // Compound types
    Vector(Box<Type>),
    Ref(Box<Type>),
    MutRef(Box<Type>),
    // Custom types
    Struct(Identifier),
    Function(Identifier),

    // Type Parameter
    TypeParameter(TypeParameter),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeParameter {
    pub name: Identifier,
    pub abilities: Vec<Ability>,
    pub is_phantom: bool,
}

/// Abilities of a struct.
/// Key requires storage.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Ability {
    Copy,
    Drop,
    Store,
    Key,
}

impl Ability {
    pub const ALL: [Ability; 4] = [Ability::Copy, Ability::Drop, Ability::Store, Ability::Key];
    pub const NONE: [Ability; 0] = [];
    pub const PRIMITIVES: [Ability; 3] = [Ability::Copy, Ability::Drop, Ability::Store];
    pub const REF: [Ability; 2] = [Ability::Copy, Ability::Drop];
}

impl Type {
    /// Check if the type is numerical.
    pub fn is_numerical(&self) -> bool {
        matches!(
            self,
            Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::U128 | Type::U256
        )
    }

    /// Check if the type is boolean
    pub fn is_bool(&self) -> bool {
        matches!(self, Type::Bool)
    }

    /// Check if the type is numerical or boolean
    pub fn is_num_or_bool(&self) -> bool {
        self.is_numerical() || self.is_bool()
    }

    /// Get an identifier for the type
    ///
    /// The returned name should be used to find the scope of this type
    /// from the IdentifierPool.
    pub fn get_name(&self) -> Identifier {
        match self {
            Type::U8 => Identifier("U8".to_string()),
            Type::U16 => Identifier("U16".to_string()),
            Type::U32 => Identifier("U32".to_string()),
            Type::U64 => Identifier("U64".to_string()),
            Type::U128 => Identifier("U128".to_string()),
            Type::U256 => Identifier("U256".to_string()),
            Type::Bool => Identifier("Bool".to_string()),
            Type::Address => Identifier("Address".to_string()),
            Type::Signer => Identifier("Signer".to_string()),
            Type::Vector(t) => Identifier(format!("Vector<{}>", t.get_name().0)),
            Type::Ref(t) => Identifier(format!("&{}", t.get_name().0)),
            Type::MutRef(t) => Identifier(format!("&mut {}", t.get_name().0)),
            Type::Struct(id) => id.clone(),
            Type::Function(id) => id.clone(),
            Type::TypeParameter(tp) => tp.name.clone(),
        }
    }

    /// Get the possible abilities of a struct type.
    /// Only give the upper bound of possible abilities.
    pub fn get_possible_abilities(&self) -> Vec<Ability> {
        match self {
            Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::U128 | Type::U256 | Type::Bool => {
                Vec::from(Ability::PRIMITIVES)
            },
            // Hardcode struct ability for now, should properly check the `has`
            Type::Struct(_) => vec![Ability::Copy, Ability::Drop],
            Type::TypeParameter(tp) => tp.abilities.clone(),
            _ => Vec::from(Ability::NONE),
        }
    }
}

/// The data structure that keeps track of types of things during generation.
/// `mapping` maps identifiers to types.
/// The identifiers could include:
/// - Variables  (e.g. var1, var2)
/// - Function arguments (e.g. fun1::arg1, fun2::arg2)
/// - Struct fields (e.g. Struct1::field1, Struct2::field2)
///
/// A key invariant assumed by the mapping is that each identifier is globally unique.
/// This is ensured by the IdentifierPool from the names module.
///
/// `all_types` is a list of all available types that have been registered.
/// This can be used to randomly select a type for a let binding.
/// Currently all basic types are registered by default.
/// All generated structs are also registered.
#[derive(Default, Debug, Clone)]
pub struct TypePool {
    mapping: BTreeMap<Identifier, Type>,
    all_types: Vec<Type>,
}

impl TypePool {
    /// Create a new TypePool.
    pub fn new() -> Self {
        Self {
            mapping: BTreeMap::new(),
            all_types: vec![
                Type::U8,
                Type::U16,
                Type::U32,
                Type::U64,
                Type::U128,
                Type::U256,
                Type::Bool,
                // Type::Address,
                // Type::Signer,
            ],
        }
    }

    /// Keep track of the type of an identifier
    pub fn insert_mapping(&mut self, id: &Identifier, typ: &Type) {
        self.mapping.insert(id.clone(), typ.clone());
    }

    /// Register a new type
    pub fn register_type(&mut self, typ: Type) {
        self.all_types.push(typ);
    }

    /// Finds all registered types that are compatible with the given abilities.
    /// If `key` is required, then the type must have `store`.
    /// For other abilities, the type must have the corresponding ability.
    pub fn get_types_for_abilities(&self, requires: &[Ability]) -> Vec<Type> {
        self.all_types
            .iter()
            .filter(|t| {
                let possible_abilities = t.get_possible_abilities();
                requires.iter().all(|req| match req {
                    Ability::Key => possible_abilities.contains(&Ability::Store),
                    _ => possible_abilities.contains(req),
                })
            })
            .cloned()
            .collect()
    }

    /// Get the type of an identifier
    /// Returns `None` if the identifier is not in the mapping.
    pub fn get_type(&self, id: &Identifier) -> Option<Type> {
        self.mapping.get(id).cloned()
    }

    /// Get all registered types
    pub fn get_all_types(&self) -> Vec<Type> {
        self.all_types.clone()
    }

    /// Returns the identifiers from the input vector that have the given type.
    pub fn filter_identifier_with_type(&self, typ: &Type, ids: Vec<Identifier>) -> Vec<Identifier> {
        let mut res = Vec::new();
        for id in ids {
            if self.get_type(&id) == Some(typ.clone()) {
                res.push(id);
            }
        }
        res
    }
}
