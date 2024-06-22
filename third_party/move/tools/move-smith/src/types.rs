// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Custom data structures for Move types.
//! Manages typing information during generation.

use crate::names::{Identifier, IdentifierKind as IDKind};
use std::collections::BTreeMap;

/// Collection of Move types.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
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
            Type::U8 => Identifier::new_str("U8", IDKind::Type),
            Type::U16 => Identifier::new_str("U16", IDKind::Type),
            Type::U32 => Identifier::new_str("U32", IDKind::Type),
            Type::U64 => Identifier::new_str("U64", IDKind::Type),
            Type::U128 => Identifier::new_str("U128", IDKind::Type),
            Type::U256 => Identifier::new_str("U256", IDKind::Type),
            Type::Bool => Identifier::new_str("Bool", IDKind::Type),
            Type::Address => Identifier::new_str("Address", IDKind::Type),
            Type::Signer => Identifier::new_str("Signer", IDKind::Type),
            Type::Vector(t) => {
                Identifier::new(format!("Vector<{}>", t.get_name().name), IDKind::Type)
            },
            Type::Ref(t) => Identifier::new(format!("&{}", t.get_name().name), IDKind::Type),
            Type::MutRef(t) => Identifier::new(format!("&mut {}", t.get_name().name), IDKind::Type),
            Type::Struct(id) => id.clone(),
            Type::Function(id) => id.clone(),
            Type::TypeParameter(tp) => tp.name.clone(),
        }
    }
}

/// The data structure that keeps track of types of things during generation.
/// `mapping` maps identifiers to types.
/// The identifiers could include:
/// - Variables  (e.g. var1, var2)
/// - Function arguments (e.g. fun1::arg1, fun2::arg2)
/// - Struct fields (e.g. Struct1::field1, Struct2::field2)
/// - Type Parameter name
///
/// A key invariant assumed by the mapping is that each identifier is globally unique.
/// This is ensured by the IdentifierPool from the names module.
#[derive(Default, Debug, Clone)]
pub struct TypePool {
    mapping: BTreeMap<Identifier, Type>,

    /// A list of all available types that have been registered.
    /// This can be used to randomly select a type for a let binding.
    /// Currently all basic types are registered by default.
    /// All generated structs and type parameters are also registered.
    all_types: Vec<Type>,

    /// Keeps track of the concrete type for type parameters
    /// Maps type parameter names to a stack of concrete types
    parameter_types: BTreeMap<Identifier, Vec<Type>>,
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
            parameter_types: BTreeMap::new(),
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

    pub fn register_concrete_type(&mut self, id: &Identifier, typ: &Type) {
        if self.parameter_types.contains_key(id) {
            self.parameter_types.get_mut(id).unwrap().push(typ.clone());
        } else {
            self.parameter_types.insert(id.clone(), vec![typ.clone()]);
        }
    }

    pub fn unregister_concrete_type(&mut self, id: &Identifier) {
        if let Some(types) = self.parameter_types.get_mut(id) {
            types.pop();
        } else {
            panic!("Cannot unregister type parameter: {:?}", id);
        }
    }

    pub fn get_concrete_type(&self, id: &Identifier) -> Option<Type> {
        if let Some(types) = self.parameter_types.get(id) {
            types.last().cloned()
        } else {
            None
        }
    }
}
