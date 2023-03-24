// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    access::ModuleAccess,
    file_format::{
        AbilitySet, CompiledModule, FieldDefinition, FunctionDefinition, SignatureToken,
        StructDefinition, StructFieldInformation, StructTypeParameter, TypeParameterIndex,
        Visibility,
    },
};
use move_core_types::{
    account_address::AccountAddress,
    identifier::{IdentStr, Identifier},
    language_storage::{ModuleId, StructTag, TypeTag},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Defines normalized representations of Move types, fields, kinds, structs, functions, and
/// modules. These representations are useful in situations that require require comparing
/// functions, resources, and types across modules. This arises in linking, compatibility checks
/// (e.g., "is it safe to deploy this new module without updating its dependents and/or restarting
/// genesis?"), defining schemas for resources stored on-chain, and (possibly in the future)
/// allowing module updates transactions.

/// A normalized version of `SignatureToken`, a type expression appearing in struct or function
/// declarations. Unlike `SignatureToken`s, `normalized::Type`s from different modules can safely be
/// compared.
#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Serialize, Deserialize)]
pub enum Type {
    #[serde(rename = "bool")]
    Bool,
    #[serde(rename = "u8")]
    U8,
    #[serde(rename = "u64")]
    U64,
    #[serde(rename = "u128")]
    U128,
    #[serde(rename = "address")]
    Address,
    #[serde(rename = "signer")]
    Signer,
    Struct {
        address: AccountAddress,
        module: Identifier,
        name: Identifier,
        type_arguments: Vec<Type>,
    },
    #[serde(rename = "vector")]
    Vector(Box<Type>),
    TypeParameter(TypeParameterIndex),
    Reference(Box<Type>),
    MutableReference(Box<Type>),
    // NOTE: Added in bytecode version v6, do not reorder!
    #[serde(rename = "u16")]
    U16,
    #[serde(rename = "u32")]
    U32,
    #[serde(rename = "u256")]
    U256,
}

/// Normalized version of a `FieldDefinition`. The `name` is included even though it is
/// metadata that it is ignored by the VM. The reason: names are important to clients. We would
/// want a change from `Account { bal: u64, seq: u64 }` to `Account { seq: u64, bal: u64 }` to be
/// marked as incompatible. Not safe to compare without an enclosing `Struct`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Field {
    pub name: Identifier,
    pub type_: Type,
}

/// Normalized version of a `StructDefinition`. Not safe to compare without an associated
/// `ModuleId` or `Module`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Struct {
    pub abilities: AbilitySet,
    pub type_parameters: Vec<StructTypeParameter>,
    pub fields: Vec<Field>,
}

/// Normalized version of a `FunctionDefinition`. Not safe to compare without an associated
/// `ModuleId` or `Module`.
#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Serialize, Deserialize)]
pub struct Function {
    pub visibility: Visibility,
    pub is_entry: bool,
    pub type_parameters: Vec<AbilitySet>,
    pub parameters: Vec<Type>,
    pub return_: Vec<Type>,
}

/// Normalized version of a `CompiledModule`: its address, name, struct declarations, and public
/// function declarations.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Module {
    pub file_format_version: u32,
    pub address: AccountAddress,
    pub name: Identifier,
    pub friends: Vec<ModuleId>,
    pub structs: BTreeMap<Identifier, Struct>,
    pub exposed_functions: BTreeMap<Identifier, Function>,
}

impl Module {
    /// Extract a normalized module from a `CompiledModule`. The module `m` should be verified.
    /// Nothing will break here if that is not the case, but there is little point in computing a
    /// normalized representation of a module that won't verify (since it can't be published).
    pub fn new(m: &CompiledModule) -> Self {
        let friends = m.immediate_friends();
        let structs = m.struct_defs().iter().map(|d| Struct::new(m, d)).collect();
        let exposed_functions = m
            .function_defs()
            .iter()
            .filter(|func_def| {
                let is_vis_exposed = match func_def.visibility {
                    Visibility::Public | Visibility::Friend => true,
                    Visibility::Private => false,
                };
                let is_entry_exposed = func_def.is_entry;
                is_vis_exposed || is_entry_exposed
            })
            .map(|func_def| Function::new(m, func_def))
            .collect();

        Self {
            file_format_version: m.version(),
            address: *m.address(),
            name: m.name().to_owned(),
            friends,
            structs,
            exposed_functions,
        }
    }

    pub fn module_id(&self) -> ModuleId {
        ModuleId::new(self.address, self.name.clone())
    }
}

impl Type {
    /// Create a normalized `Type` for `SignatureToken` `s` in module `m`.
    pub fn new(m: &CompiledModule, s: &SignatureToken) -> Self {
        use SignatureToken::*;
        match s {
            Struct(shi) => {
                let s_handle = m.struct_handle_at(*shi);
                assert!(s_handle.type_parameters.is_empty(), "A struct with N type parameters should be encoded as StructModuleInstantiation with type_arguments = [TypeParameter(1), ..., TypeParameter(N)]");
                let m_handle = m.module_handle_at(s_handle.module);
                Type::Struct {
                    address: *m.address_identifier_at(m_handle.address),
                    module: m.identifier_at(m_handle.name).to_owned(),
                    name: m.identifier_at(s_handle.name).to_owned(),
                    type_arguments: Vec::new(),
                }
            },
            StructInstantiation(shi, type_actuals) => {
                let s_handle = m.struct_handle_at(*shi);
                let m_handle = m.module_handle_at(s_handle.module);
                Type::Struct {
                    address: *m.address_identifier_at(m_handle.address),
                    module: m.identifier_at(m_handle.name).to_owned(),
                    name: m.identifier_at(s_handle.name).to_owned(),
                    type_arguments: type_actuals.iter().map(|t| Type::new(m, t)).collect(),
                }
            },
            Bool => Type::Bool,
            U8 => Type::U8,
            U16 => Type::U16,
            U32 => Type::U32,
            U64 => Type::U64,
            U128 => Type::U128,
            U256 => Type::U256,
            Address => Type::Address,
            Signer => Type::Signer,
            Vector(t) => Type::Vector(Box::new(Type::new(m, t))),
            TypeParameter(i) => Type::TypeParameter(*i),
            Reference(t) => Type::Reference(Box::new(Type::new(m, t))),
            MutableReference(t) => Type::MutableReference(Box::new(Type::new(m, t))),
        }
    }

    /// Return true if `self` is a closed type with no free type variables
    pub fn is_closed(&self) -> bool {
        use Type::*;
        match self {
            TypeParameter(_) => false,
            Bool => true,
            U8 => true,
            U16 => true,
            U32 => true,
            U64 => true,
            U128 => true,
            U256 => true,
            Address => true,
            Signer => true,
            Struct { type_arguments, .. } => type_arguments.iter().all(|t| t.is_closed()),
            Vector(t) | Reference(t) | MutableReference(t) => t.is_closed(),
        }
    }

    pub fn into_type_tag(self) -> Option<TypeTag> {
        use Type::*;
        Some(
            if self.is_closed() {
                match self {
                    Reference(_) | MutableReference(_) => return None,
                    Bool => TypeTag::Bool,
                    U8 => TypeTag::U8,
                    U16 => TypeTag::U16,
                    U32 => TypeTag::U32,
                    U64 => TypeTag::U64,
                    U128 => TypeTag::U128,
                    U256 => TypeTag::U256,
                    Address => TypeTag::Address,
                    Signer => TypeTag::Signer,
                    Vector(t) => TypeTag::Vector(Box::new(
                        t.into_type_tag()
                            .expect("Invariant violation: vector type argument contains reference"),
                    )),
                    Struct {
                        address,
                        module,
                        name,
                        type_arguments,
                    } => TypeTag::Struct(Box::new(StructTag {
                        address,
                        module,
                        name,
                        type_params: type_arguments
                            .into_iter()
                            .map(|t| {
                                t.into_type_tag().expect(
                                    "Invariant violation: struct type argument contains reference",
                                )
                            })
                            .collect(),
                    })),
                    TypeParameter(_) => unreachable!(),
                }
            } else {
                return None;
            },
        )
    }

    pub fn into_struct_tag(self) -> Option<StructTag> {
        match self.into_type_tag()? {
            TypeTag::Struct(s) => Some(*s),
            _ => None,
        }
    }

    pub fn subst(&self, type_args: &[Type]) -> Self {
        use Type::*;
        match self {
            Bool | U8 | U16 | U32 | U64 | U128 | U256 | Address | Signer => self.clone(),
            Reference(ty) => Reference(Box::new(ty.subst(type_args))),
            MutableReference(ty) => MutableReference(Box::new(ty.subst(type_args))),
            Vector(t) => Vector(Box::new(t.subst(type_args))),
            Struct {
                address,
                module,
                name,
                type_arguments,
            } => Struct {
                address: *address,
                module: module.clone(),
                name: name.clone(),
                type_arguments: type_arguments
                    .iter()
                    .map(|t| t.subst(type_args))
                    .collect::<Vec<_>>(),
            },
            TypeParameter(i) => type_args
                .get(*i as usize)
                .expect("Type parameter index out of bound")
                .clone(),
        }
    }
}

impl Field {
    /// Create a `Field` for `FieldDefinition` `f` in module `m`.
    pub fn new(m: &CompiledModule, f: &FieldDefinition) -> Self {
        Field {
            name: m.identifier_at(f.name).to_owned(),
            type_: Type::new(m, &f.signature.0),
        }
    }
}

impl Struct {
    /// Create a `Struct` for `StructDefinition` `def` in module `m`. Panics if `def` is a
    /// a native struct definition.
    pub fn new(m: &CompiledModule, def: &StructDefinition) -> (Identifier, Self) {
        let handle = m.struct_handle_at(def.struct_handle);
        let fields = match &def.field_information {
            StructFieldInformation::Native => {
                // Pretend for compatibility checking no fields
                vec![]
            },
            StructFieldInformation::Declared(fields) => {
                fields.iter().map(|f| Field::new(m, f)).collect()
            },
        };
        let name = m.identifier_at(handle.name).to_owned();
        let s = Struct {
            abilities: handle.abilities,
            type_parameters: handle.type_parameters.clone(),
            fields,
        };
        (name, s)
    }

    pub fn type_param_constraints(&self) -> impl ExactSizeIterator<Item = &AbilitySet> {
        self.type_parameters.iter().map(|param| &param.constraints)
    }
}

impl Function {
    /// Create a `FunctionSignature` for `FunctionHandle` `f` in module `m`.
    pub fn new(m: &CompiledModule, def: &FunctionDefinition) -> (Identifier, Self) {
        let fhandle = m.function_handle_at(def.function);
        let name = m.identifier_at(fhandle.name).to_owned();
        let f = Function {
            visibility: def.visibility,
            is_entry: def.is_entry,
            type_parameters: fhandle.type_parameters.clone(),
            parameters: m
                .signature_at(fhandle.parameters)
                .0
                .iter()
                .map(|s| Type::new(m, s))
                .collect(),
            return_: m
                .signature_at(fhandle.return_)
                .0
                .iter()
                .map(|s| Type::new(m, s))
                .collect(),
        };
        (name, f)
    }

    /// Create a `Function` for function named `func_name` in module `m`.
    pub fn new_from_name(m: &CompiledModule, func_name: &IdentStr) -> Option<Self> {
        for func_defs in &m.function_defs {
            if m.identifier_at(m.function_handle_at(func_defs.function).name) == func_name {
                return Some(Self::new(m, func_defs).1);
            }
        }
        None
    }
}

impl From<TypeTag> for Type {
    fn from(ty: TypeTag) -> Type {
        use Type::*;
        match ty {
            TypeTag::Bool => Bool,
            TypeTag::U8 => U8,
            TypeTag::U16 => U16,
            TypeTag::U32 => U32,
            TypeTag::U64 => U64,
            TypeTag::U128 => U128,
            TypeTag::U256 => U256,
            TypeTag::Address => Address,
            TypeTag::Signer => Signer,
            TypeTag::Vector(ty) => Vector(Box::new(Type::from(*ty))),
            TypeTag::Struct(s) => Struct {
                address: s.address,
                module: s.module,
                name: s.name,
                type_arguments: s.type_params.into_iter().map(|ty| ty.into()).collect(),
            },
        }
    }
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Type::Struct {
                address,
                module,
                name,
                type_arguments,
            } => {
                write!(
                    f,
                    "0x{}::{}::{}",
                    address.short_str_lossless(),
                    module,
                    name
                )?;
                if let Some(first_ty) = type_arguments.first() {
                    write!(f, "<")?;
                    write!(f, "{}", first_ty)?;
                    for ty in type_arguments.iter().skip(1) {
                        write!(f, ", {}", ty)?;
                    }
                    write!(f, ">")?;
                }
                Ok(())
            },
            Type::Vector(ty) => write!(f, "vector<{}>", ty),
            Type::U8 => write!(f, "u8"),
            Type::U16 => write!(f, "u16"),
            Type::U32 => write!(f, "u32"),
            Type::U64 => write!(f, "u64"),
            Type::U128 => write!(f, "u128"),
            Type::U256 => write!(f, "u256"),
            Type::Address => write!(f, "address"),
            Type::Signer => write!(f, "signer"),
            Type::Bool => write!(f, "bool"),
            Type::Reference(r) => write!(f, "&{}", r),
            Type::MutableReference(r) => write!(f, "&mut {}", r),
            Type::TypeParameter(i) => write!(f, "T{:?}", i),
        }
    }
}
