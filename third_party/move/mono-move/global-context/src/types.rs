// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::alloc::GlobalArenaPtr;
use move_core_types::{ability::AbilitySet, account_address::AccountAddress};
use std::{
    hash::{Hash, Hasher},
    ptr,
};

/// Represents a stable pointer to an interned function name, which can be
/// safely used by public APIs. Equality and hash are by pointer address.
/// Can only be obtained through [`crate::ExecutionContext`] and its lifetime
/// is bound to the context.
#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct ExecutableId<'a>(&'a ExecutableIdInternal);

/// Represents a stable pointer to an interned function name, which can be
/// safely used by public APIs. Equality and hash are by pointer address.
/// Can only be obtained through [`crate::ExecutionContext`] and its lifetime
/// is bound to the context.
#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct FunctionId<'a>(&'a str);

/// Represents a stable pointer to an interned struct name, which can be
/// safely used by public APIs. Equality and hash are by pointer address.
/// Can only be obtained through [`crate::ExecutionContext`] and its lifetime
/// is bound to the context.
#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct StructId<'a>(&'a str);

/// Represents a stable pointer to an interned type, which can be safely used
/// by public APIs. Equality and hash are by pointer address. Can only be
/// obtained through [`crate::ExecutionContext`] and its lifetime is bound to
/// the context.
#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct Type<'a>(&'a TypeInternal);

/// Represents a stable pointer to an interned type list, which stores
/// pointers to types. Can be safely used by public APIs. Equality and
/// hash are by pointer address. Can only be obtained through
/// [`crate::ExecutionContext`] and its lifetime is bound to the context.
#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct TypeList<'a>(&'a [GlobalArenaPtr<TypeInternal>]);

macro_rules! impl_ptr_eq {
    ($ty:ident) => {
        /// Compares by pointer identity. Two instances are equal
        /// only if they point to the same interned allocation.
        impl<'a> PartialEq for $ty<'a> {
            fn eq(&self, other: &Self) -> bool {
                ptr::eq(self.0 as *const _, other.0 as *const _)
            }
        }
        impl<'a> Eq for $ty<'a> {}
    };
}

macro_rules! impl_ptr_hash {
    ($ty:ident) => {
        /// Hashes the pointer, not the content.
        impl<'a> Hash for $ty<'a> {
            fn hash<H: Hasher>(&self, state: &mut H) {
                self.as_usize().hash(state)
            }
        }
    };
}

// All IDs are pointers to interned data, so using pointer equality.
impl_ptr_eq!(ExecutableId);
impl_ptr_eq!(FunctionId);
impl_ptr_eq!(StructId);
impl_ptr_eq!(Type);
impl_ptr_eq!(TypeList);

// All IDs are pointers to interned data, so using pointer hash.
impl_ptr_hash!(ExecutableId);
impl_ptr_hash!(FunctionId);
impl_ptr_hash!(StructId);
impl_ptr_hash!(Type);
impl_ptr_hash!(TypeList);

impl<'a> ExecutableId<'a> {
    /// Returns address where this executable is deployed.
    pub fn address(&self) -> &AccountAddress {
        &self.0.address
    }

    /// Returns the name of this executable.
    pub fn name(&self) -> &str {
        // SAFETY:
        //   Executable ID can only be constructed by the execution context,
        //   and the lifetime 'a proves that the context is alive (and thus,
        //   all arena allocations are valid). As a result, it is safe to
        //   dereference the inner arena pointer with the same lifetime 'a.
        unsafe { self.0.name.as_ref_unchecked() }
    }

    /// Returns the raw address of this pointer.
    pub fn as_usize(&self) -> usize {
        self.0 as *const _ as usize
    }
}

impl<'a> FunctionId<'a> {
    /// Returns the function name.
    pub fn name(&self) -> &str {
        self.0
    }

    /// Returns the raw address of this pointer.
    pub fn as_usize(&self) -> usize {
        self.0.as_ptr() as usize
    }
}

impl<'a> StructId<'a> {
    /// Returns the struct name.
    pub fn name(&self) -> &str {
        self.0
    }

    /// Returns the raw address of this pointer.
    pub fn as_usize(&self) -> usize {
        self.0.as_ptr() as usize
    }
}

impl<'a> Type<'a> {
    /// Returns the raw address of this pointer.
    pub fn as_usize(&self) -> usize {
        self.0 as *const _ as usize
    }
}

impl<'a> TypeList<'a> {
    /// Returns the length of this type list.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns true if this type list is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the raw address of this pointer.
    pub fn as_usize(&self) -> usize {
        self.0.as_ptr() as usize
    }
}

/// Opaque key for [`crate::executable_cache::ExecutableCache`].
/// Only constructible via [`ExecutableCacheKey::new`].
///
/// Stores the pointer address of an interned [`ExecutableIdInternal`].
/// Valid until the global arena is flushed; the cache is always drained
/// before the arena — see `MaintenanceContext::check_memory_usage`.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct ExecutableCacheKey(usize);

impl ExecutableCacheKey {
    /// Creates a cache key from an interned [`ExecutableId`].
    ///
    /// The key encodes the pointer address and is valid as long as the arena
    /// backing the `ExecutableId` is live (i.e. until the next flush).
    pub fn new(id: ExecutableId<'_>) -> Self {
        Self(id.as_usize())
    }
}

/// Opaque key for the non-generic function map in `ExecutableData`.
/// Only constructible via [`FunctionCacheKey::new`].
///
/// Stores the pointer address of an interned function name string.
/// Valid for the lifetime of the executable that owns the map.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct FunctionCacheKey(usize);

impl FunctionCacheKey {
    /// Creates a cache key from an interned [`FunctionId`].
    ///
    /// The key encodes the pointer address and is valid for the lifetime of
    /// the executable that stores the function map.
    pub fn new(id: FunctionId<'_>) -> Self {
        Self(id.as_usize())
    }
}

/// Cache key for memoizing type substitution results. Encodes both the template
/// type pointer and the type-arguments list pointer by address, enabling O(1)
/// lookup when the same (template, ty_args) pair is encountered again.
///
/// Valid until the global arena is flushed; both caches are cleared alongside
/// `types` and `type_lists` in `MaintenanceContext::check_memory_usage`.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub(crate) struct SubstitutionKey {
    /// Pointer to the interned generic type.
    ty: GlobalArenaPtr<TypeInternal>,
    /// Pointer to the interned type argument list.
    ty_args: GlobalArenaPtr<[GlobalArenaPtr<TypeInternal>]>,
}

impl SubstitutionKey {
    pub fn new(
        ty: GlobalArenaPtr<TypeInternal>,
        ty_args: GlobalArenaPtr<[GlobalArenaPtr<TypeInternal>]>,
    ) -> Self {
        Self { ty, ty_args }
    }
}

// Private APIs.

/// Internal representation that [ExecutableId] points to. Equality and
/// hash are by value.
pub(crate) struct ExecutableIdInternal {
    pub(crate) address: AccountAddress,
    pub(crate) name: GlobalArenaPtr<str>,
}

/// Internal type representation used for type interning. All equivalent types
/// intern to the same pointer.
pub(crate) enum TypeInternal {
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
    Vector(GlobalArenaPtr<TypeInternal>),
    Ref(GlobalArenaPtr<TypeInternal>),
    RefMut(GlobalArenaPtr<TypeInternal>),
    Struct {
        /// Module ID (address and module name).
        module_id: GlobalArenaPtr<ExecutableIdInternal>,
        /// Struct name.
        name: GlobalArenaPtr<str>,
        /// Type arguments.
        type_args: GlobalArenaPtr<[GlobalArenaPtr<TypeInternal>]>,
    },
    Function {
        /// Argument types.
        args: GlobalArenaPtr<[GlobalArenaPtr<TypeInternal>]>,
        /// Return types.
        results: GlobalArenaPtr<[GlobalArenaPtr<TypeInternal>]>,
        /// Abilities of the function.
        abilities: AbilitySet,
    },
    /// A type parameter. Substituted at monomorphization time and the type is
    /// re-canonicalized.
    TyParam(u16),
}

pub(crate) static BOOL_INTERNAL: TypeInternal = TypeInternal::Bool;
pub(crate) static U8_INTERNAL: TypeInternal = TypeInternal::U8;
pub(crate) static U16_INTERNAL: TypeInternal = TypeInternal::U16;
pub(crate) static U32_INTERNAL: TypeInternal = TypeInternal::U32;
pub(crate) static U64_INTERNAL: TypeInternal = TypeInternal::U64;
pub(crate) static U128_INTERNAL: TypeInternal = TypeInternal::U128;
pub(crate) static U256_INTERNAL: TypeInternal = TypeInternal::U256;
pub(crate) static I8_INTERNAL: TypeInternal = TypeInternal::I8;
pub(crate) static I16_INTERNAL: TypeInternal = TypeInternal::I16;
pub(crate) static I32_INTERNAL: TypeInternal = TypeInternal::I32;
pub(crate) static I64_INTERNAL: TypeInternal = TypeInternal::I64;
pub(crate) static I128_INTERNAL: TypeInternal = TypeInternal::I128;
pub(crate) static I256_INTERNAL: TypeInternal = TypeInternal::I256;
pub(crate) static ADDRESS_INTERNAL: TypeInternal = TypeInternal::Address;
pub(crate) static SIGNER_INTERNAL: TypeInternal = TypeInternal::Signer;

impl<'a> ExecutableId<'a> {
    /// Creates a new executable ID.
    ///
    /// ONLY FOR INTERNAL USE BY [`crate::ExecutionContext`].
    pub(crate) fn new_internal(id: &'a ExecutableIdInternal) -> Self {
        Self(id)
    }
}

impl<'a> FunctionId<'a> {
    /// Creates a new function ID.
    ///
    /// ONLY FOR INTERNAL USE BY [`crate::ExecutionContext`].
    pub(crate) fn new_internal(id: &'a str) -> Self {
        Self(id)
    }
}

impl<'a> StructId<'a> {
    /// Creates a new struct ID.
    ///
    /// ONLY FOR INTERNAL USE BY [`crate::ExecutionContext`].
    pub(crate) fn new_internal(id: &'a str) -> Self {
        Self(id)
    }
}

impl<'a> Type<'a> {
    /// Creates a new type.
    ///
    /// ONLY FOR INTERNAL USE BY [`crate::ExecutionContext`].
    pub(crate) fn new_internal(ty: &'a TypeInternal) -> Self {
        Self(ty)
    }
}

impl<'a> TypeList<'a> {
    /// Creates a new type list.
    ///
    /// ONLY FOR INTERNAL USE BY [`crate::ExecutionContext`].
    pub(crate) fn new_internal(tys: &'a [GlobalArenaPtr<TypeInternal>]) -> Self {
        Self(tys)
    }
}
