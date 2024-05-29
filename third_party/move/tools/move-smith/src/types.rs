use crate::names::Identifier;
use arbitrary::{Result, Unstructured};
use std::collections::HashMap;

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

#[derive(Default, Debug, Clone)]
pub struct TypePool {
    mapping: HashMap<Identifier, Type>,
}

impl TypePool {
    pub fn new() -> Self {
        Self {
            mapping: HashMap::new(),
        }
    }

    pub fn insert(&mut self, id: &Identifier, typ: &Type) {
        self.mapping.insert(id.clone(), typ.clone());
    }

    pub fn get_type(&self, id: &Identifier) -> Option<Type> {
        self.mapping.get(id).cloned()
    }

    pub fn filter_identifier_with_type(&self, typ: &Type, ids: Vec<Identifier>) -> Vec<Identifier> {
        let mut res = Vec::new();
        for id in ids {
            if self.get_type(&id) == Some(typ.clone()) {
                res.push(id);
            }
        }
        res
    }

    /// Returns one of the basic types that does not require a type argument.
    pub fn random_basic_type(&mut self, u: &mut Unstructured) -> Result<Type> {
        Ok(match u.int_in_range(0..=6)? {
            0 => Type::U8,
            1 => Type::U16,
            2 => Type::U32,
            3 => Type::U64,
            4 => Type::U128,
            5 => Type::U256,
            6 => Type::Bool,
            // x => Type::Address, // Leave these two until the end
            // x => Type::Signer,
            _ => panic!("Unsupported basic type"),
        })
    }
}
