// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Custom data structures for Move types.
//! Manages typing information during generation.

use crate::names::Identifier;
use std::collections::HashMap;

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
}

impl Type {
    /// A type is considered basic if it can be instantiated without the need
    /// to check the already generated code.
    pub fn is_basic_type(&self) -> bool {
        matches!(
            self,
            Type::U8
                | Type::U16
                | Type::U32
                | Type::U64
                | Type::U128
                | Type::U256
                | Type::Bool
                | Type::Address
                | Type::Signer
        )
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
    mapping: HashMap<Identifier, Type>,
    all_types: Vec<Type>,
}

impl TypePool {
    /// Create a new TypePool.
    pub fn new() -> Self {
        Self {
            mapping: HashMap::new(),
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
