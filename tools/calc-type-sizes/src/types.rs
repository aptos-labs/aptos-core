// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Type definitions for stack size computation.

use move_core_types::account_address::AccountAddress;
use std::{cmp::Ordering, fmt};

/// Primitive type variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PrimitiveType {
    Bool,
    U8,
    U16,
    U32,
    U64,
    U128,
    U256,
    I8,
    I16,
    I32,
    I64,
    I128,
    I256,
    Address,
    Signer,
}

impl PrimitiveType {
    /// Stack size in bytes for this primitive
    pub fn stack_size(self) -> usize {
        match self {
            PrimitiveType::Bool | PrimitiveType::U8 | PrimitiveType::I8 => 1,
            PrimitiveType::U16 | PrimitiveType::I16 => 2,
            PrimitiveType::U32 | PrimitiveType::I32 => 4,
            PrimitiveType::U64 | PrimitiveType::I64 => 8,
            PrimitiveType::U128 | PrimitiveType::I128 => 16,
            PrimitiveType::U256
            | PrimitiveType::I256
            | PrimitiveType::Address
            | PrimitiveType::Signer => 32,
        }
    }
}

impl fmt::Display for PrimitiveType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PrimitiveType::Bool => write!(f, "bool"),
            PrimitiveType::U8 => write!(f, "u8"),
            PrimitiveType::U16 => write!(f, "u16"),
            PrimitiveType::U32 => write!(f, "u32"),
            PrimitiveType::U64 => write!(f, "u64"),
            PrimitiveType::U128 => write!(f, "u128"),
            PrimitiveType::U256 => write!(f, "u256"),
            PrimitiveType::I8 => write!(f, "i8"),
            PrimitiveType::I16 => write!(f, "i16"),
            PrimitiveType::I32 => write!(f, "i32"),
            PrimitiveType::I64 => write!(f, "i64"),
            PrimitiveType::I128 => write!(f, "i128"),
            PrimitiveType::I256 => write!(f, "i256"),
            PrimitiveType::Address => write!(f, "address"),
            PrimitiveType::Signer => write!(f, "signer"),
        }
    }
}

/// A type name representation for stack size computation.
/// More specialized than TypeTag - only what we need.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeName {
    /// Primitive types (bool, u8, u64, address, etc.)
    Primitive(PrimitiveType),
    /// Vector<T> - heap allocated, fixed stack size of 24
    Vector(Box<TypeName>),
    /// Function value - just a marker, stack size of 8
    Function,
    /// Reference (immutable or mutable) - stack size of 8 (pointer)
    Reference(Box<TypeName>),
    /// Opaque type - placeholder for uninstantiated type parameters
    /// The u16 is just an identifier to distinguish different opaque types (T0, T1, etc.)
    Opaque(u16),
    /// User-defined struct or enum
    Struct {
        address: AccountAddress,
        module: String,
        name: String,
        type_args: Vec<TypeName>,
    },
}

impl TypeName {
    /// Create a struct type name without type arguments
    pub fn new_struct(address: AccountAddress, module: String, name: String) -> Self {
        TypeName::Struct {
            address,
            module,
            name,
            type_args: vec![],
        }
    }

    /// Create a struct type name with type arguments
    pub fn new_struct_with_args(
        address: AccountAddress,
        module: String,
        name: String,
        type_args: Vec<TypeName>,
    ) -> Self {
        TypeName::Struct {
            address,
            module,
            name,
            type_args,
        }
    }
}

impl fmt::Display for TypeName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeName::Primitive(p) => write!(f, "{}", p),
            TypeName::Vector(inner) => write!(f, "vector<{}>", inner),
            TypeName::Function => write!(f, "|_|_"),
            TypeName::Reference(inner) => write!(f, "&{}", inner),
            TypeName::Opaque(idx) => write!(f, "T{}", idx),
            TypeName::Struct {
                address,
                module,
                name,
                type_args,
            } => {
                write!(f, "{}::{}::{}", address, module, name)?;
                if !type_args.is_empty() {
                    write!(f, "<")?;
                    for (i, arg) in type_args.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", arg)?;
                    }
                    write!(f, ">")?;
                }
                Ok(())
            },
        }
    }
}

// Manual Ord implementation for BTreeMap usage
impl Ord for TypeName {
    fn cmp(&self, other: &Self) -> Ordering {
        self.to_string().cmp(&other.to_string())
    }
}

impl PartialOrd for TypeName {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Type classification (struct vs enum vs primitive)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeKind {
    Primitive,
    Builtin,
    Struct,
    Enum,
}

impl fmt::Display for TypeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeKind::Primitive => write!(f, "primitive"),
            TypeKind::Builtin => write!(f, "builtin"),
            TypeKind::Struct => write!(f, "struct"),
            TypeKind::Enum => write!(f, "enum"),
        }
    }
}

/// Information about a type's stack representation
#[derive(Debug, Clone)]
pub struct TypeInfo {
    /// Stack size in bytes
    pub stack_size: usize,
    /// Nesting depth (0 for primitives, max child depth + 1 for composites)
    pub nested_depth: usize,
    /// Type classification
    pub kind: TypeKind,
}
