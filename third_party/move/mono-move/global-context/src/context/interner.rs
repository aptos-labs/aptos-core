// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! This module defines [`DashMap`]-based interner for strings, address-name
//! pairs, type tags, and signature tokens.
//!
//! For zero-copy lookups through equivalent types, [`LookupKey`] wraps a
//! borrowed value and implements [`Hash`] and [`Equivalent`] against the
//! corresponding [`InternerKey`]. This enables cross-format deduplication.
//!
//! # Safety model
//!
//! Key types dereference [`GlobalArenaPtr`] in their [`Hash`], [`PartialEq`]
//! and [`Equivalent`] trait implementations. This is not possible to enforce
//! at type system level because it requires the following:
//!
//!   If global arena is reset, the interner is also reset.
//!
//! There is no way to cleanly enforce this relationship.

use crate::{alloc::GlobalArenaPtr, context::types::Type, ExecutableId};
use dashmap::{DashMap, Entry, Equivalent};
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
use std::hash::{Hash, Hasher};

/// **Internal** key representation used by interner. Wraps interned data
/// pointer.
///
/// # Safety
///
/// **Must** implement structural hash and equality.
struct InternerKey<T: ?Sized>(GlobalArenaPtr<T>);

/// **Internal** lookup key representation used by interner. Is used to wrap
/// equivalent keys for zero-copy lookups.
struct LookupKey<'a, T: ?Sized>(&'a T);

/// Interner based on [`DashMap`]. Stores global arena-allocated pointers,
/// deduplicating them based on structural hash and equality.
pub(crate) struct DashMapInterner<T: ?Sized> {
    // Note: using ahash for fast yet DoS resistant lookups because structural
    // hash is used here.
    inner: DashMap<InternerKey<T>, GlobalArenaPtr<T>, ahash::RandomState>,
}

#[allow(private_bounds)]
impl<T> DashMapInterner<T>
where
    InternerKey<T>: Hash + Eq,
    T: ?Sized,
{
    /// Creates a new interner with default settings.
    pub(crate) fn new() -> Self {
        Self {
            inner: DashMap::with_hasher(ahash::RandomState::new()),
        }
    }

    /// Returns the pointer to interned data if it exists, and [`None`]
    /// otherwise.
    ///
    /// # Safety invariant
    ///
    /// Any [`GlobalArenaPtr`] stored in the map **must** be valid and safe
    /// to dereference. That is, if the data behind the pointer is deallocated,
    /// the interner has to be reset.
    pub(super) fn get<Q>(&self, key: &Q) -> Option<GlobalArenaPtr<T>>
    where
        Q: ?Sized,
        for<'a> LookupKey<'a, Q>: Hash + Equivalent<InternerKey<T>>,
    {
        self.inner.get(&LookupKey(key)).map(|entry| *entry.value())
    }

    /// Inserts the pointer to the interner (key is derived from the pointer
    /// and **must** have structural hash and equality). If the entry exists
    /// (e.g., due to a race condition), does not insert the value and returns
    /// the existing pointer instead. If the entry does not exist, inserts the
    /// pointer and returns its copy.
    ///
    /// # Safety
    ///
    /// Pointer that is being inserted **must** be safe to dereference.
    ///
    /// # Safety invariant
    ///
    /// Any [`GlobalArenaPtr`] stored in the map **must** be valid and safe to
    /// dereference.That is, the interner has to be reset if the data behind
    /// the pointer is deallocated.
    pub(super) unsafe fn insert(&self, ptr: GlobalArenaPtr<T>) -> GlobalArenaPtr<T> {
        match self.inner.entry(InternerKey(ptr)) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => *entry.insert(ptr),
        }
    }

    /// Returns the number of interned entries.
    pub(super) fn len(&self) -> usize {
        self.inner.len()
    }

    /// Removes all entries from the interner's cache.
    pub(super) fn reset(&self) {
        self.inner.clear()
    }
}

impl<T> Default for DashMapInterner<T>
where
    InternerKey<T>: Hash + Eq,
    T: ?Sized,
{
    fn default() -> Self {
        Self::new()
    }
}

//
// Only private APIs below. These implementations are used when getting the
// data from the interner map or inserting into it. Hence, the caller must
// enforce the safety preconditions to ensure every raw pointer dereference
// is safe and sound.
// ----------------------------------------------------------------------------

impl Hash for InternerKey<str> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // SAFETY: User of interner must enforce the safety precondition: the
        // global arena pointer points to a live allocation.
        unsafe {
            self.0.as_ref_unchecked().hash(state);
        }
    }
}

impl Hash for LookupKey<'_, str> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl PartialEq for InternerKey<str> {
    fn eq(&self, other: &Self) -> bool {
        // SAFETY: User of interner must enforce the safety precondition: the
        // global arena pointer points to a live allocation.
        unsafe { self.0.as_ref_unchecked() == other.0.as_ref_unchecked() }
    }
}

impl Eq for InternerKey<str> {}

impl Equivalent<InternerKey<str>> for LookupKey<'_, str> {
    fn equivalent(&self, key: &InternerKey<str>) -> bool {
        // SAFETY: User of interner must enforce the safety precondition: the
        // global arena pointer points to a live allocation.
        unsafe { self.0 == key.0.as_ref_unchecked() }
    }
}

impl Hash for InternerKey<ExecutableId> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // SAFETY: User of interner must enforce the safety precondition: the
        // global arena pointer points to a live allocation.
        unsafe {
            let id = self.0.as_ref_unchecked();
            id.address.hash(state);
            id.name.as_ref_unchecked().hash(state);
        }
    }
}

impl Hash for LookupKey<'_, (&AccountAddress, &IdentStr)> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0 .0.hash(state);
        self.0 .1.hash(state);
    }
}

impl PartialEq for InternerKey<ExecutableId> {
    fn eq(&self, other: &Self) -> bool {
        // SAFETY: User of interner must enforce the safety precondition: the
        // global arena pointer points to a live allocation.
        unsafe {
            let this_id = self.0.as_ref_unchecked();
            let other_id = other.0.as_ref_unchecked();
            this_id.address == other_id.address
                && this_id.name.as_ref_unchecked() == other_id.name.as_ref_unchecked()
        }
    }
}

impl Eq for InternerKey<ExecutableId> {}

impl Equivalent<InternerKey<ExecutableId>> for LookupKey<'_, (&AccountAddress, &IdentStr)> {
    fn equivalent(&self, other: &InternerKey<ExecutableId>) -> bool {
        // SAFETY: User of interner must enforce the safety precondition: the
        // global arena pointer points to a live allocation.
        unsafe {
            let other_id = other.0.as_ref_unchecked();
            self.0 .0 == &other_id.address && self.0 .1.as_str() == other_id.name.as_ref_unchecked()
        }
    }
}

/// Canonical discriminants for cross-format hashing. This ensures that **ALL**
/// keys hash to the same value.
mod type_discriminant {
    pub(super) const BOOL: u8 = 0;
    pub(super) const U8: u8 = 1;
    pub(super) const U16: u8 = 2;
    pub(super) const U32: u8 = 3;
    pub(super) const U64: u8 = 4;
    pub(super) const U128: u8 = 5;
    pub(super) const U256: u8 = 6;
    pub(super) const I8: u8 = 7;
    pub(super) const I16: u8 = 8;
    pub(super) const I32: u8 = 9;
    pub(super) const I64: u8 = 10;
    pub(super) const I128: u8 = 11;
    pub(super) const I256: u8 = 12;
    pub(super) const ADDRESS: u8 = 13;
    pub(super) const SIGNER: u8 = 14;
    pub(super) const VECTOR: u8 = 15;
    pub(super) const STRUCT: u8 = 16;
    pub(super) const REFERENCE: u8 = 17;
    pub(super) const REFERENCE_MUT: u8 = 18;
    pub(super) const FUNCTION: u8 = 19;
    pub(super) const TYPE_PARAM: u8 = 20;
}

fn hash_function_param_or_return_tag<H: Hasher>(tag: &FunctionParamOrReturnTag, state: &mut H) {
    match tag {
        FunctionParamOrReturnTag::Reference(tag) => {
            type_discriminant::REFERENCE.hash(state);
            LookupKey(tag).hash(state);
        },
        FunctionParamOrReturnTag::MutableReference(tag) => {
            type_discriminant::REFERENCE_MUT.hash(state);
            LookupKey(tag).hash(state);
        },
        FunctionParamOrReturnTag::Value(tag) => {
            LookupKey(tag).hash(state);
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
        LookupKey(&(arg, module)).hash(state);
    }
}

impl Hash for LookupKey<'_, TypeTag> {
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
                Self(inner.as_ref()).hash(state);
            },

            TypeTag::Struct(struct_tag) => {
                type_discriminant::STRUCT.hash(state);
                struct_tag.address.hash(state);
                struct_tag.module.as_str().hash(state);
                struct_tag.name.as_str().hash(state);
                struct_tag.type_args.len().hash(state);
                for type_arg in &struct_tag.type_args {
                    Self(type_arg).hash(state);
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

impl Hash for LookupKey<'_, (&SignatureToken, &CompiledModule)> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self.0 .0 {
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
                LookupKey(&(tok.as_ref(), self.0 .1)).hash(state);
            },

            SignatureToken::Reference(tok) => {
                type_discriminant::REFERENCE.hash(state);
                LookupKey(&(tok.as_ref(), self.0 .1)).hash(state);
            },

            SignatureToken::MutableReference(tok) => {
                type_discriminant::REFERENCE_MUT.hash(state);
                LookupKey(&(tok.as_ref(), self.0 .1)).hash(state);
            },

            SignatureToken::Struct(idx) => {
                hash_struct_signature_token(idx, &[], self.0 .1, state);
            },
            SignatureToken::StructInstantiation(idx, type_args) => {
                hash_struct_signature_token(idx, type_args, self.0 .1, state);
            },

            SignatureToken::Function(args, results, abilities) => {
                type_discriminant::FUNCTION.hash(state);
                args.len().hash(state);
                for arg in args {
                    LookupKey(&(arg, self.0 .1)).hash(state);
                }
                results.len().hash(state);
                for result in results {
                    LookupKey(&(result, self.0 .1)).hash(state);
                }
                abilities.hash(state);
            },

            SignatureToken::TypeParameter(idx) => {
                type_discriminant::TYPE_PARAM.hash(state);
                idx.hash(state);
            },
        }
    }
}

impl Hash for InternerKey<Type> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // SAFETY: Only reachable through DashMapInterner, which requires
        // ExecutionContextScope.
        match unsafe { self.0.as_ref_unchecked() } {
            Type::Bool => type_discriminant::BOOL.hash(state),
            Type::U8 => type_discriminant::U8.hash(state),
            Type::U16 => type_discriminant::U16.hash(state),
            Type::U32 => type_discriminant::U32.hash(state),
            Type::U64 => type_discriminant::U64.hash(state),
            Type::U128 => type_discriminant::U128.hash(state),
            Type::U256 => type_discriminant::U256.hash(state),
            Type::I8 => type_discriminant::I8.hash(state),
            Type::I16 => type_discriminant::I16.hash(state),
            Type::I32 => type_discriminant::I32.hash(state),
            Type::I64 => type_discriminant::I64.hash(state),
            Type::I128 => type_discriminant::I128.hash(state),
            Type::I256 => type_discriminant::I256.hash(state),
            Type::Address => type_discriminant::ADDRESS.hash(state),
            Type::Signer => type_discriminant::SIGNER.hash(state),

            Type::Vector(ty) => {
                type_discriminant::VECTOR.hash(state);
                Self(*ty).hash(state);
            },

            Type::Ref(ty) => {
                type_discriminant::REFERENCE.hash(state);
                Self(*ty).hash(state);
            },

            Type::RefMut(ty) => {
                type_discriminant::REFERENCE_MUT.hash(state);
                Self(*ty).hash(state);
            },

            Type::Struct {
                executable_id,
                name,
                type_args,
            } => {
                type_discriminant::STRUCT.hash(state);

                // SAFETY:
                //
                // User of interner **must** enforce the safety precondition.
                unsafe {
                    let id = executable_id.as_ref_unchecked();
                    id.address.hash(state);
                    id.name.as_ref_unchecked().hash(state);
                    name.as_ref_unchecked().hash(state);
                };
                InternerKey(*type_args).hash(state);
            },

            Type::Function {
                args,
                results,
                abilities,
            } => {
                type_discriminant::FUNCTION.hash(state);
                InternerKey(*args).hash(state);
                InternerKey(*results).hash(state);
                abilities.hash(state);
            },

            Type::TypeParam(idx) => {
                type_discriminant::TYPE_PARAM.hash(state);
                idx.hash(state);
            },
        }
    }
}

impl Hash for LookupKey<'_, [TypeTag]> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.len().hash(state);
        for tag in self.0 {
            LookupKey(tag).hash(state);
        }
    }
}

impl Hash for LookupKey<'_, (&[SignatureToken], &CompiledModule)> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0 .0.len().hash(state);
        for token in self.0 .0 {
            LookupKey(&(token, self.0 .1)).hash(state);
        }
    }
}

impl Hash for InternerKey<[GlobalArenaPtr<Type>]> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // SAFETY:
        //
        // User of interner **must** enforce the safety precondition.
        let types = unsafe { self.0.as_ref_unchecked() };
        types.len().hash(state);
        for ty in types {
            InternerKey(*ty).hash(state);
        }
    }
}

impl PartialEq for InternerKey<Type> {
    fn eq(&self, other: &Self) -> bool {
        // SAFETY:
        //
        // User of interner **must** enforce the safety precondition.
        let this = unsafe { self.0.as_ref_unchecked() };
        let other = unsafe { other.0.as_ref_unchecked() };

        match (this, other) {
            (Type::Bool, Type::Bool)
            | (Type::U8, Type::U8)
            | (Type::U16, Type::U16)
            | (Type::U32, Type::U32)
            | (Type::U64, Type::U64)
            | (Type::U128, Type::U128)
            | (Type::U256, Type::U256)
            | (Type::I8, Type::I8)
            | (Type::I16, Type::I16)
            | (Type::I32, Type::I32)
            | (Type::I64, Type::I64)
            | (Type::I128, Type::I128)
            | (Type::I256, Type::I256)
            | (Type::Address, Type::Address)
            | (Type::Signer, Type::Signer) => true,

            (Type::Vector(ty), Type::Vector(other_ty))
            | (Type::Ref(ty), Type::Ref(other_ty))
            | (Type::RefMut(ty), Type::RefMut(other_ty)) => Self(*ty) == Self(*other_ty),

            (
                Type::Struct {
                    executable_id,
                    name,
                    type_args,
                },
                Type::Struct {
                    executable_id: other_executable_id,
                    name: other_name,
                    type_args: other_type_args,
                },
            ) => {
                executable_id == other_executable_id
                    && name == other_name
                    && InternerKey(*type_args) == InternerKey(*other_type_args)
            },

            (
                Type::Function {
                    args,
                    results,
                    abilities,
                },
                Type::Function {
                    args: other_args,
                    results: other_results,
                    abilities: other_abilities,
                },
            ) => {
                InternerKey(*args) == InternerKey(*other_args)
                    && InternerKey(*results) == InternerKey(*other_results)
                    && abilities == other_abilities
            },

            (Type::TypeParam(a), Type::TypeParam(b)) => a == b,

            _ => false,
        }
    }
}

impl Eq for InternerKey<Type> {}

impl PartialEq for InternerKey<[GlobalArenaPtr<Type>]> {
    fn eq(&self, other: &Self) -> bool {
        // SAFETY:
        //
        // User of interner **must** enforce the safety precondition.
        let types = unsafe { self.0.as_ref_unchecked() };
        let other_types = unsafe { other.0.as_ref_unchecked() };

        if types.len() != other_types.len() {
            return false;
        }

        types
            .iter()
            .zip(other_types.iter())
            .all(|(ty, other_ty)| InternerKey(*ty) == InternerKey(*other_ty))
    }
}

impl Eq for InternerKey<[GlobalArenaPtr<Type>]> {}

fn function_param_or_return_tag_eq_type(
    tag: &FunctionParamOrReturnTag,
    ty: &GlobalArenaPtr<Type>,
) -> bool {
    match tag {
        FunctionParamOrReturnTag::Reference(inner_tag) => {
            // SAFETY:
            //
            // User of interner **must** enforce the safety precondition.
            if let Type::Ref(ty) = unsafe { ty.as_ref_unchecked() } {
                LookupKey(inner_tag).equivalent(&InternerKey(*ty))
            } else {
                false
            }
        },
        FunctionParamOrReturnTag::MutableReference(inner_tag) => {
            // SAFETY:
            //
            // User of interner **must** enforce the safety precondition.
            if let Type::RefMut(ty) = unsafe { ty.as_ref_unchecked() } {
                LookupKey(inner_tag).equivalent(&InternerKey(*ty))
            } else {
                false
            }
        },
        FunctionParamOrReturnTag::Value(inner_tag) => {
            LookupKey(inner_tag).equivalent(&InternerKey(*ty))
        },
    }
}

impl Hash for LookupKey<'_, FunctionParamOrReturnTag> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        hash_function_param_or_return_tag(self.0, state);
    }
}

impl Equivalent<InternerKey<Type>> for LookupKey<'_, FunctionParamOrReturnTag> {
    fn equivalent(&self, key: &InternerKey<Type>) -> bool {
        function_param_or_return_tag_eq_type(self.0, &key.0)
    }
}

impl Hash for LookupKey<'_, [FunctionParamOrReturnTag]> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.len().hash(state);
        for tag in self.0 {
            hash_function_param_or_return_tag(tag, state);
        }
    }
}

impl Equivalent<InternerKey<[GlobalArenaPtr<Type>]>> for LookupKey<'_, [FunctionParamOrReturnTag]> {
    fn equivalent(&self, key: &InternerKey<[GlobalArenaPtr<Type>]>) -> bool {
        // SAFETY:
        //
        // User of interner **must** enforce the safety precondition.
        let types = unsafe { key.0.as_ref_unchecked() };
        if self.0.len() != types.len() {
            return false;
        }
        self.0
            .iter()
            .zip(types.iter())
            .all(|(tag, ty)| function_param_or_return_tag_eq_type(tag, ty))
    }
}

impl Equivalent<InternerKey<Type>> for LookupKey<'_, TypeTag> {
    fn equivalent(&self, key: &InternerKey<Type>) -> bool {
        // SAFETY:
        //
        // User of interner **must** enforce the safety precondition.
        let other = unsafe { key.0.as_ref_unchecked() };

        match (self.0, other) {
            (TypeTag::Bool, Type::Bool)
            | (TypeTag::U8, Type::U8)
            | (TypeTag::U16, Type::U16)
            | (TypeTag::U32, Type::U32)
            | (TypeTag::U64, Type::U64)
            | (TypeTag::U128, Type::U128)
            | (TypeTag::U256, Type::U256)
            | (TypeTag::I8, Type::I8)
            | (TypeTag::I16, Type::I16)
            | (TypeTag::I32, Type::I32)
            | (TypeTag::I64, Type::I64)
            | (TypeTag::I128, Type::I128)
            | (TypeTag::I256, Type::I256)
            | (TypeTag::Address, Type::Address)
            | (TypeTag::Signer, Type::Signer) => true,

            (TypeTag::Vector(tag), Type::Vector(ty)) => {
                LookupKey(tag.as_ref()).equivalent(&InternerKey(*ty))
            },

            (
                TypeTag::Struct(struct_tag),
                Type::Struct {
                    executable_id,
                    name,
                    type_args,
                },
            ) => {
                // SAFETY:
                //
                // User of interner **must** enforce the safety precondition.
                unsafe {
                    let id = executable_id.as_ref_unchecked();
                    id.address == struct_tag.address
                        && id.name.as_ref_unchecked() == struct_tag.module.as_str()
                        && name.as_ref_unchecked() == struct_tag.name.as_str()
                        && LookupKey(struct_tag.type_args.as_slice())
                            .equivalent(&InternerKey(*type_args))
                }
            },

            (
                TypeTag::Function(function_tag),
                Type::Function {
                    args,
                    results,
                    abilities,
                },
            ) => {
                if &function_tag.abilities != abilities {
                    return false;
                }

                // SAFETY:
                //
                // User of interner **must** enforce the safety precondition.
                let args_list = unsafe { args.as_ref_unchecked() };
                let results_list = unsafe { results.as_ref_unchecked() };

                if function_tag.args.len() != args_list.len()
                    || function_tag.results.len() != results_list.len()
                {
                    return false;
                }

                function_tag
                    .args
                    .iter()
                    .zip(args_list.iter())
                    .chain(function_tag.results.iter().zip(results_list.iter()))
                    .all(|(tag, ty)| function_param_or_return_tag_eq_type(tag, ty))
            },

            _ => false,
        }
    }
}

impl Equivalent<InternerKey<Type>> for LookupKey<'_, (&SignatureToken, &CompiledModule)> {
    fn equivalent(&self, key: &InternerKey<Type>) -> bool {
        // SAFETY:
        //
        // User of interner **must** enforce the safety precondition.
        let other = unsafe { key.0.as_ref_unchecked() };

        match (self.0 .0, other) {
            (SignatureToken::Bool, Type::Bool)
            | (SignatureToken::U8, Type::U8)
            | (SignatureToken::U16, Type::U16)
            | (SignatureToken::U32, Type::U32)
            | (SignatureToken::U64, Type::U64)
            | (SignatureToken::U128, Type::U128)
            | (SignatureToken::U256, Type::U256)
            | (SignatureToken::I8, Type::I8)
            | (SignatureToken::I16, Type::I16)
            | (SignatureToken::I32, Type::I32)
            | (SignatureToken::I64, Type::I64)
            | (SignatureToken::I128, Type::I128)
            | (SignatureToken::I256, Type::I256)
            | (SignatureToken::Address, Type::Address)
            | (SignatureToken::Signer, Type::Signer) => true,

            (SignatureToken::Vector(tok), Type::Vector(ty))
            | (SignatureToken::Reference(tok), Type::Ref(ty))
            | (SignatureToken::MutableReference(tok), Type::RefMut(ty)) => {
                LookupKey(&(tok.as_ref(), self.0 .1)).equivalent(&InternerKey(*ty))
            },

            (
                SignatureToken::Struct(idx),
                Type::Struct {
                    executable_id,
                    name,
                    type_args,
                },
            ) => {
                let struct_handle = self.0 .1.struct_handle_at(*idx);
                let module_handle = self.0 .1.module_handle_at(struct_handle.module);

                // SAFETY:
                //
                // User of interner **must** enforce the safety precondition.
                unsafe {
                    let id = executable_id.as_ref_unchecked();
                    &id.address == self.0 .1.address_identifier_at(module_handle.address)
                        && id.name.as_ref_unchecked()
                            == self.0 .1.identifier_at(module_handle.name).as_str()
                        && name.as_ref_unchecked()
                            == self.0 .1.identifier_at(struct_handle.name).as_str()
                        && type_args.as_ref_unchecked().is_empty()
                }
            },

            (
                SignatureToken::StructInstantiation(idx, tokens),
                Type::Struct {
                    executable_id,
                    name,
                    type_args,
                },
            ) => {
                let struct_handle = self.0 .1.struct_handle_at(*idx);
                let module_handle = self.0 .1.module_handle_at(struct_handle.module);

                // SAFETY:
                //
                // User of interner **must** enforce the safety precondition.
                unsafe {
                    let id = executable_id.as_ref_unchecked();
                    &id.address == self.0 .1.address_identifier_at(module_handle.address)
                        && id.name.as_ref_unchecked()
                            == self.0 .1.identifier_at(module_handle.name).as_str()
                        && name.as_ref_unchecked()
                            == self.0 .1.identifier_at(struct_handle.name).as_str()
                        && LookupKey(&(tokens.as_slice(), self.0 .1))
                            .equivalent(&InternerKey(*type_args))
                }
            },

            (
                SignatureToken::Function(tok_args, tok_results, tok_abilities),
                Type::Function {
                    args,
                    results,
                    abilities,
                },
            ) => {
                LookupKey(&(tok_args.as_slice(), self.0 .1)).equivalent(&InternerKey(*args))
                    && LookupKey(&(tok_results.as_slice(), self.0 .1))
                        .equivalent(&InternerKey(*results))
                    && tok_abilities == abilities
            },

            (SignatureToken::TypeParameter(idx), Type::TypeParam(n)) => idx == n,

            _ => false,
        }
    }
}

impl Equivalent<InternerKey<[GlobalArenaPtr<Type>]>> for LookupKey<'_, [TypeTag]> {
    fn equivalent(&self, key: &InternerKey<[GlobalArenaPtr<Type>]>) -> bool {
        // SAFETY:
        //
        // User of interner **must** enforce the safety precondition.
        let types = unsafe { key.0.as_ref_unchecked() };
        if self.0.len() != types.len() {
            return false;
        }

        self.0
            .iter()
            .zip(types.iter())
            .all(|(tag, ty)| LookupKey(tag).equivalent(&InternerKey(*ty)))
    }
}

impl Equivalent<InternerKey<[GlobalArenaPtr<Type>]>>
    for LookupKey<'_, (&[SignatureToken], &CompiledModule)>
{
    fn equivalent(&self, key: &InternerKey<[GlobalArenaPtr<Type>]>) -> bool {
        // SAFETY:
        //
        // User of interner **must** enforce the safety precondition.
        let types = unsafe { key.0.as_ref_unchecked() };
        if self.0 .0.len() != types.len() {
            return false;
        }

        self.0
             .0
            .iter()
            .zip(types.iter())
            .all(|(tok, ty)| LookupKey(&(tok, self.0 .1)).equivalent(&InternerKey(*ty)))
    }
}
