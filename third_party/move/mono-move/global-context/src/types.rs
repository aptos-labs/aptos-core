// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::arena::ArenaPtr;
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
pub struct TypeList<'a>(&'a [ArenaPtr<TypeInternal>]);

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
    /// Returns the raw address of this pointer.
    pub fn as_usize(&self) -> usize {
        self.0.as_ptr() as usize
    }
}

// Private APIs.

/// Internal representation that [ExecutableId] points to. Equality and
/// hash are by value.
pub(crate) struct ExecutableIdInternal {
    pub(crate) address: AccountAddress,
    pub(crate) name: ArenaPtr<str>,
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
    Vector(ArenaPtr<TypeInternal>),
    Ref(ArenaPtr<TypeInternal>),
    RefMut(ArenaPtr<TypeInternal>),
    Struct {
        /// Module ID (address and module name).
        module_id: ArenaPtr<ExecutableIdInternal>,
        /// Struct name.
        name: ArenaPtr<str>,
        /// Type arguments.
        type_args: ArenaPtr<[ArenaPtr<TypeInternal>]>,
    },
    Function {
        /// Argument types.
        args: ArenaPtr<[ArenaPtr<TypeInternal>]>,
        /// Return types.
        results: ArenaPtr<[ArenaPtr<TypeInternal>]>,
        /// Abilities of the function.
        abilities: AbilitySet,
    },
}

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
    pub(crate) fn new_internal(tys: &'a [ArenaPtr<TypeInternal>]) -> Self {
        Self(tys)
    }
}
