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

    pub fn get_type(&mut self, id: &Identifier) -> Option<Type> {
        self.mapping.get(id).cloned()
    }

    pub fn get_identifiers_of_type(&self, typ: &Type) -> Vec<Identifier> {
        self.mapping
            .iter()
            .filter_map(|(id, t)| if t == typ { Some(id.clone()) } else { None })
            .collect()
    }

    /// Returns one of the basic types that does not require a type argument.
    pub fn random_basic_type(&mut self, u: &mut Unstructured) -> Result<Type> {
        Ok(match u.int_in_range(0..=5)? {
            0 => Type::U8,
            1 => Type::U16,
            2 => Type::U32,
            3 => Type::U64,
            4 => Type::U128,
            5 => Type::U256,
            _ => panic!("Unsupported basic type"),
        })
    }
}
