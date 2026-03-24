// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! This submodule defines APIs to intern Move module IDs as arena-allocated
//! executable IDs.
//!
//! # Safety model
//!
//! IDs are interned as canonical pointers. Public APIs return a scoped pointer
//! ([`ArenaRef`]) with lifetime of the execution guard. This prevents any
//! use-after-free for interned IDs at compile-time.
//!
//! Interning may happen concurrently, in which case it is guaranteed that all
//! workers agree on a single canonical pointer. To reduce contention for
//! better performance, arena allocations for IDs happen outside the lock, and
//! so may leak memory. However, any extra allocated memory for any unique
//! address-pair and concurrent worker is at most ~40 bytes (address plus
//! pointer sizes).
//!
//! The deduplication map where interned IDs are stored is **always** cleared
//! when the arena backing the allocation is reset. It is safe to dereference
//! any arena-based pointers stored in the map.

use crate::{context::ArenaRef, ExecutionGuard};
use dashmap::Equivalent;
use mono_move_alloc::GlobalArenaPtr;
use move_core_types::{
    account_address::AccountAddress, identifier::IdentStr, language_storage::ModuleId,
};
use std::hash::{Hash, Hasher};

impl<'guard> ArenaRef<'guard, ExecutableId> {
    /// Returns the account address of this executable.
    pub fn address(&self) -> &'guard AccountAddress {
        // SAFETY: The lifetime on this reference guarantees that the execution
        // guard is still alive, which guarantees that the arena allocation is
        // still valid and there were no deallocations.
        unsafe { &self.ptr.as_ref_unchecked().address }
    }

    /// Returns the name of this executable.
    pub fn name(&self) -> &'guard str {
        // SAFETY: The guard guarantees that we are still in execution phase
        // and the pointer to ID is still valid. Because the name was arena
        // allocated before this ID was created (enforced by global arena
        // allocator APIs), the name allocation is  also valid for the same
        // lifetime.
        unsafe {
            let id = self.ptr.as_ref_unchecked();
            id.name.as_ref_unchecked()
        }
    }
}

#[allow(private_interfaces)]
impl<'ctx> ExecutionGuard<'ctx> {
    /// Interns [`ModuleId`] as an arena-allocated executable ID and returns a
    /// reference to it, with lifetime scoped to the lifetime of the execution
    /// guard.
    ///
    /// On cache hit, returns a canonical pointer interned previously.
    /// On cache miss, interns the module name and allocates the ID in the
    /// global arena, returning a pointer to:
    ///   - Allocated ID if the ID was not interned before.
    ///   - Existing canonical pointer if the interned ID exists. Note that the
    ///     entry can exist due to a race condition because allocation is done
    ///     outside the lock (by design). In this case, extra allocations are
    ///     bounded: we allocate extra address and a name pointer (40 bytes)
    ///     per worker in the worst case for each unique address-name pair.
    ///
    /// # Safety precondition
    ///
    /// During global arena reset, identifiers map must be cleared as well.
    pub fn intern_module_id<'guard>(
        &'guard self,
        module_id: &ModuleId,
    ) -> ArenaRef<'guard, ExecutableId>
    where
        'ctx: 'guard,
    {
        self.intern_address_name(&module_id.address, &module_id.name)
    }

    /// Interns [`AccountAddress`]-[`IdentStr`] pair as an arena-allocated
    /// executable ID and returns a reference to it, with lifetime scoped to
    /// the lifetime of the execution guard.
    ///
    /// On cache hit, returns a canonical pointer interned previously.
    /// On cache miss, interns the module name and allocates the ID in the
    /// global arena, returning a pointer to:
    ///   - Allocated ID if the ID was not interned before.
    ///   - Existing canonical pointer if the interned ID exists. Note that the
    ///     entry can exist due to a race condition because allocation is done
    ///     outside the lock (by design). In this case, extra allocations are
    ///     bounded: we allocate extra address and a name pointer (40 bytes)
    ///     per worker in the worst case for each unique address-name pair.
    ///
    /// # Safety precondition
    ///
    /// During global arena reset, identifiers map must be cleared as well.
    pub fn intern_address_name<'guard>(
        &'guard self,
        address: &AccountAddress,
        name: &IdentStr,
    ) -> ArenaRef<'guard, ExecutableId>
    where
        'ctx: 'guard,
    {
        let ptr = self.intern_address_name_internal(*address, name);

        // SAFETY: If the returned pointer is the one that was allocated, it is
        // trivially valid until the next maintenance phase, and it is safe to
        // cast it to the lifetime of the execution guard. If the returned
        // pointer already existed in the map, it also must be valid until the
        // next maintenance (and previous maintenance phase has not invalidated
        // it because then the map would have been cleared).
        unsafe { self.arena_ref(ptr) }
    }
}

//
// Only private APIs below.
// ------------------------

/// Identifies an executable (module or script) by its address and name.
///   - For modules, constructed from module address and name.
///   - For scripts: TODO
///
/// # Safety
///
/// Must be created from a valid global arena pointer to executable's name.
pub struct ExecutableId {
    address: AccountAddress,
    name: GlobalArenaPtr<str>,
}

#[allow(private_interfaces)]
impl<'ctx> ExecutionGuard<'ctx> {
    pub(super) fn intern_address_name_internal(
        &self,
        address: AccountAddress,
        name: &IdentStr,
    ) -> GlobalArenaPtr<ExecutableId> {
        // SAFETY: All existing keys/values are valid pointers because the map
        // is guaranteed to be cleared on arena's reset.
        if let Some(entry) = self.ctx.executable_ids.get(&(&address, name)) {
            return *entry.value();
        }

        // SAFETY: Name pointer has been just interned - it is valid and can be
        // used safely for ID construction.
        let name = self.intern_identifier_internal(name);
        let ptr = self.global_arena.alloc(ExecutableId { address, name });

        // SAFETY: We have just allocated the pointer, hence it is safe to wrap
        // it as a key and compute hash / equality. All existing keys are also
        // valid pointers because the map is cleared on arena's reset.
        *self
            .ctx
            .executable_ids
            .entry(ExecutableIdInternerKey(ptr))
            .or_insert(ptr)
    }
}

/// Wraps allocated executable ID pointer to implement structural hash and
/// equality.
///
/// # Safety
///
/// Constructor must enforce the pointer points to the valid data and can be
/// safely dereferenced.
pub(super) struct ExecutableIdInternerKey(GlobalArenaPtr<ExecutableId>);

impl Hash for ExecutableIdInternerKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // SAFETY: It is safe to dereference the pointer because the caller
        // ensures it remains valid during the lifetime of the key.
        unsafe {
            let id = self.0.as_ref_unchecked();
            id.address.hash(state);
            id.name.as_ref_unchecked().hash(state);
        }
    }
}

impl PartialEq for ExecutableIdInternerKey {
    fn eq(&self, other: &Self) -> bool {
        // SAFETY: It is safe to dereference the pointer because the caller
        // ensures it remains valid during the lifetime of the key.
        unsafe {
            let this_id = self.0.as_ref_unchecked();
            let other_id = other.0.as_ref_unchecked();
            this_id.address == other_id.address
                && this_id.name.as_ref_unchecked() == other_id.name.as_ref_unchecked()
        }
    }
}

// PartialEq implementation above is a full equivalence relation.
impl Eq for ExecutableIdInternerKey {}

impl Equivalent<ExecutableIdInternerKey> for (&AccountAddress, &IdentStr) {
    fn equivalent(&self, key: &ExecutableIdInternerKey) -> bool {
        // SAFETY: It is safe to dereference the pointer because the caller
        // ensures it remains valid during the lifetime of the key.
        unsafe {
            let key = key.0.as_ref_unchecked();
            self.0 == &key.address && self.1.as_str() == key.name.as_ref_unchecked()
        }
    }
}
