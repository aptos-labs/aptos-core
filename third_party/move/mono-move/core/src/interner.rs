// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Interning APIs.

use crate::{
    types::{InternedType, InternedTypeList},
    ExecutableId,
};
use mono_move_alloc::GlobalArenaPtr;
use move_core_types::{ability::AbilitySet, account_address::AccountAddress, identifier::IdentStr};

/// Pointer to interned Move identifier allocated in global arena.
pub type InternedIdentifier = GlobalArenaPtr<str>;

/// Pointer to interned module ID allocated in global arena.
// TODO: rename ExecutableId to ModuleID
pub type InternedModuleId = GlobalArenaPtr<ExecutableId>;

/// Interns Move file format types into efficient pointer-based implementation
/// where data is allocated in arena.
///
/// # Invariant
///
/// Implementations deduplicate allocations, so that pointer equality implies
/// structural equality.
pub trait Interner {
    /// Returns a type parameter with the specified index. Note that pointer
    /// equality of any two interned type parameters is structural only. Two
    /// parameters with index 0 but at different scope may represent different
    /// types (but intern to the same pointer).
    fn type_param_of(&self, idx: u16) -> InternedType;

    /// Returns a vector of the specified type.
    fn vector_of(&self, elem: InternedType) -> InternedType;

    /// Returns an immutable reference to the specified type.
    fn immut_ref_of(&self, inner: InternedType) -> InternedType;

    /// Returns a mutable reference to the specified type.
    fn mut_ref_of(&self, inner: InternedType) -> InternedType;

    /// Returns a function type with the given argument and result type lists
    /// and ability set.
    fn function_of(
        &self,
        args: InternedTypeList,
        results: InternedTypeList,
        abilities: AbilitySet,
    ) -> InternedType;

    /// Returns an interned list of types.
    fn type_list_of(&self, types: &[InternedType]) -> InternedTypeList;

    /// Returns the interned nominal (struct or enum) identity.
    fn nominal_of(
        &self,
        module_id: InternedModuleId,
        name: InternedIdentifier,
        ty_args: InternedTypeList,
    ) -> InternedType;

    /// Returns the interned IR corresponding to (address, module name) pair
    /// that identifies a module.
    fn module_id_of(&self, address: &AccountAddress, name: &IdentStr) -> InternedModuleId;

    /// Returns an interned string identifier.
    fn identifier_of(&self, identifier: &IdentStr) -> InternedIdentifier;
}
