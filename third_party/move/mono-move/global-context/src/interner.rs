// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module defines [`DashMap`]-based interner as well as the keys it uses.
//!
//! Interners use the following keys:
//! - [`IdentifierKey`]: wraps a pointer to arena-allocated strings.
//! - [`ExecutableIdKey`]: wraps a pointer to arena-allocated executable IDs.
//! - [`TypeKey`]: wraps a pointer to arena-allocated type.
//! - [`TypeListKey`]: wraps a pointer to arena-allocated list of types.
//!
//! For zero-copy lookups through equivalent types, an additional set of keys
//! is defined:
//! - [`TypeTagKey`]: key for [`TypeTag`] lookups.
//! - [`SignatureTokenKey`]: key for [`SignatureToken`] lookups,
//! - [`TypeTagListKey`]: key for [`TypeTag`] list lookups,
//! - [`SignatureTokenListKey`]: key for [`SignatureToken`] list lookups,
//!
//! All keys implement **compatible** hashing to enable cross-format
//! deduplication.

use crate::{
    arena::ArenaPtr,
    types::{ExecutableIdInternal, TypeInternal},
};
use dashmap::{DashMap, Equivalent};
use move_binary_format::{
    access::ModuleAccess,
    file_format::{SignatureToken, StructHandleIndex},
    CompiledModule,
};
use move_core_types::{
    account_address::AccountAddress,
    identifier::IdentStr,
    language_storage::{FunctionParamOrReturnTag, TypeTag},
};
use std::{
    borrow::Borrow,
    hash::{Hash, Hasher},
};

/// Interned based on [`DashMap`]. Stores arena-allocated pointers, deduplicating them
/// based on structural hash and equality.
pub(crate) struct DashMapInterner<K, T: ?Sized> {
    inner: DashMap<K, ArenaPtr<T>>,
}

impl<K, T> DashMapInterner<K, T>
where
    K: Hash + Eq + From<ArenaPtr<T>>,
    T: ?Sized,
{
    /// Creates a new interner with default settings.
    pub(crate) fn new() -> Self {
        Self {
            inner: DashMap::new(),
        }
    }

    /// Returns the pointer to interned data if it exists, and [`None`] otherwise.
    pub(crate) fn get<Q>(&self, key: &Q) -> Option<ArenaPtr<T>>
    where
        Q: Hash + Equivalent<K> + ?Sized,
    {
        self.inner.get(key).map(|entry| *entry.value())
    }

    /// Inserts the pointer to the interner (key is derived from the pointer
    /// and uses structural hash and equality). If the entry exists (e.g., due
    /// to a race condition), does not insert the value and returns the existing
    /// pointer instead. If the entry does not exist, inserts the pointer and
    /// returns its copy.
    pub(crate) fn insert(&self, ptr: ArenaPtr<T>) -> ArenaPtr<T> {
        match self.inner.entry(K::from(ptr)) {
            dashmap::mapref::entry::Entry::Occupied(entry) => *entry.get(),
            dashmap::mapref::entry::Entry::Vacant(entry) => *entry.insert(ptr),
        }
    }

    /// Returns the number of interned entries.
    pub(crate) fn len(&self) -> usize {
        self.inner.len()
    }

    /// Removes all entries.
    pub(crate) fn clear(&self) {
        self.inner.clear()
    }
}

impl<K, T> Default for DashMapInterner<K, T>
where
    K: Hash + Eq + From<ArenaPtr<T>>,
    T: ?Sized,
{
    fn default() -> Self {
        Self::new()
    }
}

/// Canonical discriminants for cross-format hashing. This ensures that **ALL**
/// keys hash to the same value.
mod type_discriminant {
    pub(crate) const BOOL: u8 = 0;
    pub(crate) const U8: u8 = 1;
    pub(crate) const U16: u8 = 2;
    pub(crate) const U32: u8 = 3;
    pub(crate) const U64: u8 = 4;
    pub(crate) const U128: u8 = 5;
    pub(crate) const U256: u8 = 6;
    pub(crate) const I8: u8 = 7;
    pub(crate) const I16: u8 = 8;
    pub(crate) const I32: u8 = 9;
    pub(crate) const I64: u8 = 10;
    pub(crate) const I128: u8 = 11;
    pub(crate) const I256: u8 = 12;
    pub(crate) const ADDRESS: u8 = 13;
    pub(crate) const SIGNER: u8 = 14;
    pub(crate) const VECTOR: u8 = 15;
    pub(crate) const STRUCT: u8 = 16;
    pub(crate) const REFERENCE: u8 = 17;
    pub(crate) const REFERENCE_MUT: u8 = 18;
    pub(crate) const FUNCTION: u8 = 19;
}

/// Key for interned identifiers.
pub(crate) struct IdentifierKey(ArenaPtr<str>);

/// Key for interned executable IDs.
pub(crate) struct ExecutableIdKey(ArenaPtr<ExecutableIdInternal>);

/// Key for interned types.
pub(crate) struct TypeKey(ArenaPtr<TypeInternal>);

/// Key for interned type lists.
pub(crate) struct TypeListKey(ArenaPtr<[ArenaPtr<TypeInternal>]>);

/// Lookup key for interned types.
pub(crate) struct TypeTagKey<'a>(pub(crate) &'a TypeTag);

/// Lookup key for interned types. Requires module or script context to resolve
/// struct handles.
pub(crate) struct SignatureTokenKey<'a>(
    pub(crate) &'a SignatureToken,
    pub(crate) &'a CompiledModule,
);

/// Lookup key for interned type lists.
pub(crate) struct TypeTagListKey<'a>(pub(crate) &'a [TypeTag]);

/// Lookup key for interned type lists. Requires module or script context to resolve
/// struct handles.
pub(crate) struct SignatureTokenListKey<'a>(
    pub(crate) &'a [SignatureToken],
    pub(crate) &'a CompiledModule,
);

macro_rules! impl_from {
    ($ty:ty, $key:ty) => {
        impl From<ArenaPtr<$ty>> for $key {
            fn from(value: ArenaPtr<$ty>) -> Self {
                Self(value)
            }
        }
    };
}

impl_from!(str, IdentifierKey);
impl_from!(ExecutableIdInternal, ExecutableIdKey);
impl_from!(TypeInternal, TypeKey);
impl_from!([ArenaPtr<TypeInternal>], TypeListKey);

impl IdentifierKey {
    fn as_str(&self) -> &str {
        // SAFETY: identifier keys are only created and hashed within ExecutionContext scope,
        // The arena remains valid during hashing.
        unsafe { self.0.as_ref_unchecked() }
    }
}

impl Hash for IdentifierKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

impl PartialEq for IdentifierKey {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for IdentifierKey {}

impl Borrow<str> for IdentifierKey {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}


fn hash_function_param_or_return_tag<H: Hasher>(tag: &FunctionParamOrReturnTag, state: &mut H) {
    match tag {
        FunctionParamOrReturnTag::Reference(tag) => {
            type_discriminant::REFERENCE.hash(state);
            TypeTagKey(tag).hash(state);
        },
        FunctionParamOrReturnTag::MutableReference(tag) => {
            type_discriminant::REFERENCE_MUT.hash(state);
            TypeTagKey(tag).hash(state);
        },
        FunctionParamOrReturnTag::Value(tag) => {
            TypeTagKey(tag).hash(state);
        },
    }
}

fn hash_struct_signature_token<H: Hasher>(
    idx: &StructHandleIndex,
    type_args: &[SignatureToken],
    module: &CompiledModule,
    state: &mut H,
) {
    type_discriminant::STRUCT.hash(state);

    let struct_handle = module.struct_handle_at(*idx);
    let module_handle = module.module_handle_at(struct_handle.module);

    module
        .address_identifier_at(module_handle.address)
        .hash(state);
    module
        .identifier_at(module_handle.name)
        .as_str()
        .hash(state);
    module
        .identifier_at(struct_handle.name)
        .as_str()
        .hash(state);

    type_args.len().hash(state);
    for arg in type_args {
        SignatureTokenKey(arg, module).hash(state);
    }
}

impl Hash for TypeTagKey<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self.0 {
            TypeTag::Bool => type_discriminant::BOOL.hash(state),
            TypeTag::U8 => type_discriminant::U8.hash(state),
            TypeTag::U16 => type_discriminant::U16.hash(state),
            TypeTag::U32 => type_discriminant::U32.hash(state),
            TypeTag::U64 => type_discriminant::U64.hash(state),
            TypeTag::U128 => type_discriminant::U128.hash(state),
            TypeTag::U256 => type_discriminant::U256.hash(state),
            TypeTag::I8 => type_discriminant::I8.hash(state),
            TypeTag::I16 => type_discriminant::I16.hash(state),
            TypeTag::I32 => type_discriminant::I32.hash(state),
            TypeTag::I64 => type_discriminant::I64.hash(state),
            TypeTag::I128 => type_discriminant::I128.hash(state),
            TypeTag::I256 => type_discriminant::I256.hash(state),
            TypeTag::Address => type_discriminant::ADDRESS.hash(state),
            TypeTag::Signer => type_discriminant::SIGNER.hash(state),

            TypeTag::Vector(inner) => {
                type_discriminant::VECTOR.hash(state);
                TypeTagKey(inner.as_ref()).hash(state);
            },

            TypeTag::Struct(struct_tag) => {
                type_discriminant::STRUCT.hash(state);
                struct_tag.address.hash(state);
                struct_tag.module.as_str().hash(state);
                struct_tag.name.as_str().hash(state);
                struct_tag.type_args.len().hash(state);
                for type_arg in &struct_tag.type_args {
                    TypeTagKey(type_arg).hash(state);
                }
            },

            TypeTag::Function(function_tag) => {
                type_discriminant::FUNCTION.hash(state);
                function_tag.args.len().hash(state);
                for arg in &function_tag.args {
                    hash_function_param_or_return_tag(arg, state);
                }
                function_tag.results.len().hash(state);
                for result in &function_tag.results {
                    hash_function_param_or_return_tag(result, state);
                }
                function_tag.abilities.hash(state);
            },
        }
    }
}

impl Hash for SignatureTokenKey<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self.0 {
            SignatureToken::Bool => type_discriminant::BOOL.hash(state),
            SignatureToken::U8 => type_discriminant::U8.hash(state),
            SignatureToken::U16 => type_discriminant::U16.hash(state),
            SignatureToken::U32 => type_discriminant::U32.hash(state),
            SignatureToken::U64 => type_discriminant::U64.hash(state),
            SignatureToken::U128 => type_discriminant::U128.hash(state),
            SignatureToken::U256 => type_discriminant::U256.hash(state),
            SignatureToken::I8 => type_discriminant::I8.hash(state),
            SignatureToken::I16 => type_discriminant::I16.hash(state),
            SignatureToken::I32 => type_discriminant::I32.hash(state),
            SignatureToken::I64 => type_discriminant::I64.hash(state),
            SignatureToken::I128 => type_discriminant::I128.hash(state),
            SignatureToken::I256 => type_discriminant::I256.hash(state),
            SignatureToken::Address => type_discriminant::ADDRESS.hash(state),
            SignatureToken::Signer => type_discriminant::SIGNER.hash(state),

            SignatureToken::Vector(tok) => {
                type_discriminant::VECTOR.hash(state);
                SignatureTokenKey(tok.as_ref(), self.1).hash(state);
            },

            SignatureToken::Reference(tok) => {
                type_discriminant::REFERENCE.hash(state);
                SignatureTokenKey(tok.as_ref(), self.1).hash(state);
            },

            SignatureToken::MutableReference(tok) => {
                type_discriminant::REFERENCE_MUT.hash(state);
                SignatureTokenKey(tok.as_ref(), self.1).hash(state);
            },

            SignatureToken::Struct(idx) => {
                hash_struct_signature_token(idx, &[], self.1, state);
            },
            SignatureToken::StructInstantiation(idx, type_args) => {
                hash_struct_signature_token(idx, type_args, self.1, state);
            },

            SignatureToken::Function(args, results, abilities) => {
                type_discriminant::FUNCTION.hash(state);
                args.len().hash(state);
                for arg in args {
                    SignatureTokenKey(arg, self.1).hash(state);
                }
                results.len().hash(state);
                for result in results {
                    SignatureTokenKey(result, self.1).hash(state);
                }
                abilities.hash(state);
            },

            SignatureToken::TypeParameter(_) => {
                panic!("Type parameters cannot be interned!")
            },
        }
    }
}

// SAFETY: TypeKey is only constructed within ExecutionContext methods. The
// arena remains valid during hashing because:
// 1. ExecutionContext holds RwLockReadGuard preventing flush
// 2. Hash is called synchronously within same guard scope
// 3. No suspension points between construction and hash
impl Hash for TypeKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match unsafe { self.0.as_ref_unchecked() } {
            TypeInternal::Bool => type_discriminant::BOOL.hash(state),
            TypeInternal::U8 => type_discriminant::U8.hash(state),
            TypeInternal::U16 => type_discriminant::U16.hash(state),
            TypeInternal::U32 => type_discriminant::U32.hash(state),
            TypeInternal::U64 => type_discriminant::U64.hash(state),
            TypeInternal::U128 => type_discriminant::U128.hash(state),
            TypeInternal::U256 => type_discriminant::U256.hash(state),
            TypeInternal::I8 => type_discriminant::I8.hash(state),
            TypeInternal::I16 => type_discriminant::I16.hash(state),
            TypeInternal::I32 => type_discriminant::I32.hash(state),
            TypeInternal::I64 => type_discriminant::I64.hash(state),
            TypeInternal::I128 => type_discriminant::I128.hash(state),
            TypeInternal::I256 => type_discriminant::I256.hash(state),
            TypeInternal::Address => type_discriminant::ADDRESS.hash(state),
            TypeInternal::Signer => type_discriminant::SIGNER.hash(state),

            TypeInternal::Vector(ty) => {
                type_discriminant::VECTOR.hash(state);
                TypeKey(*ty).hash(state);
            },

            TypeInternal::Ref(ty) => {
                type_discriminant::REFERENCE.hash(state);
                TypeKey(*ty).hash(state);
            },

            TypeInternal::RefMut(ty) => {
                type_discriminant::REFERENCE_MUT.hash(state);
                TypeKey(*ty).hash(state);
            },

            TypeInternal::Struct {
                module_id,
                name,
                type_args,
            } => {
                type_discriminant::STRUCT.hash(state);
                // SAFETY: Already within ExecutionContext scope per impl safety comment
                let module_id_ref = unsafe { module_id.as_ref_unchecked() };
                module_id_ref.address.hash(state);
                module_id_ref.name.hash(state);
                unsafe { name.as_ref_unchecked() }.hash(state);
                TypeListKey(*type_args).hash(state);
            },

            TypeInternal::Function {
                args,
                results,
                abilities,
            } => {
                type_discriminant::FUNCTION.hash(state);
                TypeListKey(*args).hash(state);
                TypeListKey(*results).hash(state);
                abilities.hash(state);
            },
        }
    }
}

impl Hash for TypeTagListKey<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.len().hash(state);
        for tag in self.0 {
            TypeTagKey(tag).hash(state);
        }
    }
}

impl Hash for SignatureTokenListKey<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.len().hash(state);
        for token in self.0 {
            SignatureTokenKey(token, self.1).hash(state);
        }
    }
}

// SAFETY: TypeListKey is only constructed within ExecutionContext methods. The
// arena remains valid during hashing because:
// 1. ExecutionContext holds RwLockReadGuard preventing flush
// 2. Hash is called synchronously within same guard scope
// 3. No suspension points between construction and hash
impl Hash for TypeListKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let types = unsafe { self.0.as_ref_unchecked() };
        types.len().hash(state);
        for ty in types {
            TypeKey(*ty).hash(state);
        }
    }
}

// Structural equality implementations - must match structural hashing

// SAFETY: TypeKey is only constructed within ExecutionContext methods. The
// arena remains valid during equality check because:
// 1. ExecutionContext holds RwLockReadGuard preventing flush
// 2. Eq is called synchronously within same guard scope
// 3. No suspension points between construction and eq
impl PartialEq for TypeKey {
    fn eq(&self, other: &Self) -> bool {
        match (unsafe { self.0.as_ref_unchecked() }, unsafe {
            other.0.as_ref_unchecked()
        }) {
            (TypeInternal::Bool, TypeInternal::Bool)
            | (TypeInternal::U8, TypeInternal::U8)
            | (TypeInternal::U16, TypeInternal::U16)
            | (TypeInternal::U32, TypeInternal::U32)
            | (TypeInternal::U64, TypeInternal::U64)
            | (TypeInternal::U128, TypeInternal::U128)
            | (TypeInternal::U256, TypeInternal::U256)
            | (TypeInternal::I8, TypeInternal::I8)
            | (TypeInternal::I16, TypeInternal::I16)
            | (TypeInternal::I32, TypeInternal::I32)
            | (TypeInternal::I64, TypeInternal::I64)
            | (TypeInternal::I128, TypeInternal::I128)
            | (TypeInternal::I256, TypeInternal::I256)
            | (TypeInternal::Address, TypeInternal::Address)
            | (TypeInternal::Signer, TypeInternal::Signer) => true,

            (TypeInternal::Vector(ty), TypeInternal::Vector(other_ty))
            | (TypeInternal::Ref(ty), TypeInternal::Ref(other_ty))
            | (TypeInternal::RefMut(ty), TypeInternal::RefMut(other_ty)) => {
                TypeKey(*ty) == TypeKey(*other_ty)
            },

            (
                TypeInternal::Struct {
                    module_id,
                    name,
                    type_args,
                },
                TypeInternal::Struct {
                    module_id: other_module_id,
                    name: other_name,
                    type_args: other_type_args,
                },
            ) => {
                // SAFETY: Already within ExecutionContext scope per impl safety comment
                let module_id_ref = unsafe { module_id.as_ref_unchecked() };
                let other_module_id_ref = unsafe { other_module_id.as_ref_unchecked() };
                module_id_ref.address == other_module_id_ref.address
                    && module_id_ref.name == other_module_id_ref.name
                    && unsafe { name.as_ref_unchecked() }
                        == unsafe { other_name.as_ref_unchecked() }
                    && TypeListKey(*type_args) == TypeListKey(*other_type_args)
            },

            (
                TypeInternal::Function {
                    args,
                    results,
                    abilities,
                },
                TypeInternal::Function {
                    args: other_args,
                    results: other_results,
                    abilities: other_abilities,
                },
            ) => {
                TypeListKey(*args) == TypeListKey(*other_args)
                    && TypeListKey(*results) == TypeListKey(*other_results)
                    && abilities == other_abilities
            },

            _ => false,
        }
    }
}

impl Eq for TypeKey {}

// SAFETY: TypeListKey is only constructed within ExecutionContext methods. The
// arena remains valid during equality check because:
// 1. ExecutionContext holds RwLockReadGuard preventing flush
// 2. Eq is called synchronously within same guard scope
// 3. No suspension points between construction and eq
impl PartialEq for TypeListKey {
    fn eq(&self, other: &Self) -> bool {
        let types = unsafe { self.0.as_ref_unchecked() };
        let other_types = unsafe { other.0.as_ref_unchecked() };

        if types.len() != other_types.len() {
            return false;
        }

        types
            .iter()
            .zip(other_types.iter())
            .all(|(ty, other_ty)| TypeKey(*ty) == TypeKey(*other_ty))
    }
}

impl Eq for TypeListKey {}

// Helper function to compare FunctionParamOrReturnTag with Type
fn function_param_or_return_tag_eq_type(
    tag: &FunctionParamOrReturnTag,
    ty: &ArenaPtr<TypeInternal>,
) -> bool {
    // SAFETY: Called within TypeTagKey::equivalent which is already in ExecutionContext scope
    match tag {
        FunctionParamOrReturnTag::Reference(inner_tag) => {
            if let TypeInternal::Ref(ty) = unsafe { ty.as_ref_unchecked() } {
                TypeTagKey(inner_tag).equivalent(&TypeKey(*ty))
            } else {
                false
            }
        },
        FunctionParamOrReturnTag::MutableReference(inner_tag) => {
            if let TypeInternal::RefMut(ty) = unsafe { ty.as_ref_unchecked() } {
                TypeTagKey(inner_tag).equivalent(&TypeKey(*ty))
            } else {
                false
            }
        },
        FunctionParamOrReturnTag::Value(inner_tag) => {
            TypeTagKey(inner_tag).equivalent(&TypeKey(*ty))
        },
    }
}

// SAFETY: TypeTagKey is used as a borrowed key for DashMap lookups. The key's
// TypeKey contains ArenaPtr which is dereferenced here. This is safe because:
// 1. Equivalent is called during DashMap operations within ExecutionContext
// 2. ExecutionContext holds RwLockReadGuard preventing flush
// 3. No suspension points during the lookup operation
impl Equivalent<TypeKey> for TypeTagKey<'_> {
    fn equivalent(&self, key: &TypeKey) -> bool {
        match (self.0, unsafe { key.0.as_ref_unchecked() }) {
            (TypeTag::Bool, TypeInternal::Bool)
            | (TypeTag::U8, TypeInternal::U8)
            | (TypeTag::U16, TypeInternal::U16)
            | (TypeTag::U32, TypeInternal::U32)
            | (TypeTag::U64, TypeInternal::U64)
            | (TypeTag::U128, TypeInternal::U128)
            | (TypeTag::U256, TypeInternal::U256)
            | (TypeTag::I8, TypeInternal::I8)
            | (TypeTag::I16, TypeInternal::I16)
            | (TypeTag::I32, TypeInternal::I32)
            | (TypeTag::I64, TypeInternal::I64)
            | (TypeTag::I128, TypeInternal::I128)
            | (TypeTag::I256, TypeInternal::I256)
            | (TypeTag::Address, TypeInternal::Address)
            | (TypeTag::Signer, TypeInternal::Signer) => true,

            (TypeTag::Vector(tag), TypeInternal::Vector(ty)) => {
                TypeTagKey(tag).equivalent(&TypeKey(*ty))
            },

            (
                TypeTag::Struct(struct_tag),
                TypeInternal::Struct {
                    module_id,
                    name,
                    type_args,
                },
            ) => {
                // SAFETY: Dereferencing ArenaPtr fields within same ExecutionContext scope
                let module_id_ref = unsafe { module_id.as_ref_unchecked() };
                module_id_ref.address == struct_tag.address
                    && unsafe { module_id_ref.name.as_ref_unchecked() }
                        == struct_tag.module.as_str()
                    && unsafe { name.as_ref_unchecked() } == struct_tag.name.as_str()
                    && TypeTagListKey(&struct_tag.type_args).equivalent(&TypeListKey(*type_args))
            },

            (
                TypeTag::Function(function_tag),
                TypeInternal::Function {
                    args,
                    results,
                    abilities,
                },
            ) => {
                // Check abilities first
                if &function_tag.abilities != abilities {
                    return false;
                }

                // SAFETY: Dereferencing ArenaPtr fields within same ExecutionContext scope
                let args_list = unsafe { args.as_ref_unchecked() };
                let results_list = unsafe { results.as_ref_unchecked() };

                // Check lengths
                if function_tag.args.len() != args_list.len()
                    || function_tag.results.len() != results_list.len()
                {
                    return false;
                }

                // Compare arguments
                for (tag_arg, ty) in function_tag.args.iter().zip(args_list.iter()) {
                    if !function_param_or_return_tag_eq_type(tag_arg, ty) {
                        return false;
                    }
                }

                // Compare results
                for (tag_result, ty) in function_tag.results.iter().zip(results_list.iter()) {
                    if !function_param_or_return_tag_eq_type(tag_result, ty) {
                        return false;
                    }
                }

                true
            },

            _ => false,
        }
    }
}

// SAFETY: SignatureTokenKey is used as a borrowed key for DashMap lookups. The
// key's TypeKey contains ArenaPtr which is dereferenced here. This is safe because:
// 1. Equivalent is called during DashMap operations within ExecutionContext
// 2. ExecutionContext holds RwLockReadGuard preventing flush
// 3. No suspension points during the lookup operation
impl Equivalent<TypeKey> for SignatureTokenKey<'_> {
    fn equivalent(&self, key: &TypeKey) -> bool {
        match (self.0, unsafe { key.0.as_ref_unchecked() }) {
            (SignatureToken::Bool, TypeInternal::Bool)
            | (SignatureToken::U8, TypeInternal::U8)
            | (SignatureToken::U16, TypeInternal::U16)
            | (SignatureToken::U32, TypeInternal::U32)
            | (SignatureToken::U64, TypeInternal::U64)
            | (SignatureToken::U128, TypeInternal::U128)
            | (SignatureToken::U256, TypeInternal::U256)
            | (SignatureToken::I8, TypeInternal::I8)
            | (SignatureToken::I16, TypeInternal::I16)
            | (SignatureToken::I32, TypeInternal::I32)
            | (SignatureToken::I64, TypeInternal::I64)
            | (SignatureToken::I128, TypeInternal::I128)
            | (SignatureToken::I256, TypeInternal::I256)
            | (SignatureToken::Address, TypeInternal::Address)
            | (SignatureToken::Signer, TypeInternal::Signer) => true,

            (SignatureToken::Vector(tok), TypeInternal::Vector(ty))
            | (SignatureToken::Reference(tok), TypeInternal::Ref(ty))
            | (SignatureToken::MutableReference(tok), TypeInternal::RefMut(ty)) => {
                SignatureTokenKey(tok, self.1).equivalent(&TypeKey(*ty))
            },

            (
                SignatureToken::Struct(idx),
                TypeInternal::Struct {
                    module_id,
                    name,
                    type_args,
                },
            ) => {
                let struct_handle = self.1.struct_handle_at(*idx);
                let module_handle = self.1.module_handle_at(struct_handle.module);

                // SAFETY: Dereferencing ArenaPtr fields within same ExecutionContext scope
                let module_id_ref = unsafe { module_id.as_ref_unchecked() };
                &module_id_ref.address == self.1.address_identifier_at(module_handle.address)
                    && unsafe { module_id_ref.name.as_ref_unchecked() }
                        == self.1.identifier_at(module_handle.name).as_str()
                    && unsafe { name.as_ref_unchecked() }
                        == self.1.identifier_at(struct_handle.name).as_str()
                    && unsafe { type_args.as_ref_unchecked() }.is_empty()
            },

            (
                SignatureToken::StructInstantiation(idx, tokens),
                TypeInternal::Struct {
                    module_id,
                    name,
                    type_args,
                },
            ) => {
                let struct_handle = self.1.struct_handle_at(*idx);
                let module_handle = self.1.module_handle_at(struct_handle.module);

                // SAFETY: Dereferencing ArenaPtr fields within same ExecutionContext scope
                let module_id_ref = unsafe { module_id.as_ref_unchecked() };
                &module_id_ref.address == self.1.address_identifier_at(module_handle.address)
                    && unsafe { module_id_ref.name.as_ref_unchecked() }
                        == self.1.identifier_at(module_handle.name).as_str()
                    && unsafe { name.as_ref_unchecked() }
                        == self.1.identifier_at(struct_handle.name).as_str()
                    && SignatureTokenListKey(tokens, self.1).equivalent(&TypeListKey(*type_args))
            },

            (
                SignatureToken::Function(tok_args, tok_results, tok_abilities),
                TypeInternal::Function {
                    args,
                    results,
                    abilities,
                },
            ) => {
                SignatureTokenListKey(tok_args, self.1).equivalent(&TypeListKey(*args))
                    && SignatureTokenListKey(tok_results, self.1).equivalent(&TypeListKey(*results))
                    && tok_abilities == abilities
            },

            (SignatureToken::TypeParameter(_), _) => {
                panic!("TypeParameter cannot be interned")
            },

            _ => false,
        }
    }
}

// SAFETY: TypeTagListKey is used as a borrowed key for DashMap lookups. This is
// safe for the same reasons as TypeTagKey - called within ExecutionContext scope.
impl Equivalent<TypeListKey> for TypeTagListKey<'_> {
    fn equivalent(&self, key: &TypeListKey) -> bool {
        let types = unsafe { key.0.as_ref_unchecked() };
        if self.0.len() != types.len() {
            return false;
        }

        self.0
            .iter()
            .zip(types.iter())
            .all(|(tag, ty)| TypeTagKey(tag).equivalent(&TypeKey(*ty)))
    }
}

// SAFETY: SignatureTokenListKey is used as a borrowed key for DashMap lookups.
// This is safe for the same reasons as SignatureTokenKey - called within
// ExecutionContext scope.
impl Equivalent<TypeListKey> for SignatureTokenListKey<'_> {
    fn equivalent(&self, key: &TypeListKey) -> bool {
        let types = unsafe { key.0.as_ref_unchecked() };
        if self.0.len() != types.len() {
            return false;
        }

        self.0
            .iter()
            .zip(types.iter())
            .all(|(tok, ty)| SignatureTokenKey(tok, self.1).equivalent(&TypeKey(*ty)))
    }
}

impl ExecutableIdKey {
    fn address(&self) -> &AccountAddress {
        // SAFETY: ExecutableId is only created and hashed within ExecutionContext scope.
        // The arena remains valid during hashing.
        let id = unsafe { self.0.as_ref_unchecked() };
        &id.address
    }

    fn name(&self) -> &str {
        // SAFETY: ExecutableId is only created and hashed within ExecutionContext scope.
        // The arena remains valid during hashing.
        let id = unsafe { self.0.as_ref_unchecked() };
        unsafe { id.name.as_ref_unchecked() }
    }
}

impl Hash for ExecutableIdKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.address().hash(state);
        self.name().hash(state);
    }
}

impl PartialEq for ExecutableIdKey {
    fn eq(&self, other: &Self) -> bool {
        self.address() == other.address() && self.name() == other.name()
    }
}

impl Eq for ExecutableIdKey {}

impl Equivalent<ExecutableIdKey> for (&AccountAddress, &IdentStr) {
    fn equivalent(&self, key: &ExecutableIdKey) -> bool {
        self.0 == key.address() && self.1.as_str() == key.name()
    }
}
