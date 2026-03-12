// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! This submodule defines APIs to create and IDs for executables.
//!
//! # Safety model
//!
//! All arena string allocations here are wrapped in [`ExecutableIdRef`], which
//! ties the validity of the underlying pointer to the  execution guard's
//! lifetime. The borrow checker therefore prevents any use of an allocation
//! after the guard is dropped, without requiring any runtime checks.

use crate::{alloc::GlobalArenaPtr, ExecutionGuard, NameRef};
use dashmap::{Entry, Equivalent};
use move_core_types::{
    account_address::AccountAddress, identifier::IdentStr, language_storage::ModuleId,
};
use std::{
    hash::{Hash, Hasher},
    marker::PhantomData,
};

/// A reference to interned executable ID, identifies an executable (module or
/// script) by its address and name.
///
/// # Safety model
///
/// The reference lifetime is tied to the lifetime of the [`ExecutionGuard`].
/// It is guaranteed that the data it points to is kept alive as long as the
/// guard is alive.
#[repr(transparent)]
pub struct ExecutableIdRef<'a> {
    ptr: GlobalArenaPtr<ExecutableId>,
    _guard: PhantomData<&'a ()>,
}

impl<'a> ExecutableIdRef<'a> {
    /// Returns the account address of this executable.
    pub fn address(&self) -> &'a AccountAddress {
        // SAFETY: The lifetime on this reference guarantees that the execution
        // guard is still alive, which guarantees that the arena allocation is
        // still valid and there were no deallocations.
        unsafe { &self.ptr.as_ref_unchecked().address }
    }

    /// Returns the name of this executable.
    pub fn name(&self) -> &'a str {
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

    /// Returns the raw address of the allocation of the pointer. For testing
    /// purposes only.
    pub fn raw_address_for_testing(&self) -> usize {
        self.ptr.as_raw_ptr().addr()
    }
}

impl<'a> Hash for ExecutableIdRef<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.ptr.hash(state)
    }
}

impl<'a> PartialEq for ExecutableIdRef<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.ptr == other.ptr
    }
}

impl<'a> Eq for ExecutableIdRef<'a> {}

impl<'a> Copy for ExecutableIdRef<'a> {}

impl<'a> Clone for ExecutableIdRef<'a> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a> ExecutionGuard<'a> {
    /// Interns [`ModuleId`] as [`ExecutableId`] and returns a reference to it.
    /// The reference is valid for the lifetime of the [`ExecutionGuard`].
    pub fn intern_module_id<'b>(&'b self, module_id: &ModuleId) -> ExecutableIdRef<'b>
    where
        'a: 'b,
    {
        self.intern_address_name(&module_id.address, &module_id.name)
    }

    /// Interns [`AccountAddress`]-[`Identifier`] pair as [`ExecutableId`] and
    /// returns a reference to it. The reference is valid for the lifetime of
    /// the [`ExecutionGuard`].
    pub fn intern_address_name<'b>(
        &'b self,
        address: &AccountAddress,
        name: &IdentStr,
    ) -> ExecutableIdRef<'b>
    where
        'a: 'b,
    {
        if let Some(executable_id_ref) = self.get_interned_executable_id(address, name) {
            return executable_id_ref;
        };

        let name = self.intern_identifier(name);
        self.alloc_and_intern_executable_id(*address, name)
    }
}

//
// Only private APIs below.
// ------------------------

/// Identifies an executable (module or script) by its address and name. For
/// internal usage only.
///
/// # Invariant
///
/// Only [`ExecutionGuard::alloc_and_intern_executable_id`] can create a
/// pointer to [`ExecutableId`]. No external code or file can construct or
/// inspect fields directly; access goes through [`ExecutableIdRef`].
pub(super) struct ExecutableId {
    address: AccountAddress,
    name: GlobalArenaPtr<str>,
}

/// Wraps allocated executable ID pointer to implement structural hash and
/// equality.
///
/// # Safety
///
/// APIs must enforce the pointer points to valid data and can be dereferenced.
pub(super) struct ExecutableIdInternerKey(GlobalArenaPtr<ExecutableId>);

impl<'a> ExecutionGuard<'a> {
    /// Returns a reference to the identifier if it has been interned before.
    ///
    /// # Safety precondition
    ///
    /// During global arena reset, executable IDs map must be cleared as well.
    fn get_interned_executable_id<'b>(
        &'b self,
        address: &AccountAddress,
        name: &IdentStr,
    ) -> Option<ExecutableIdRef<'b>>
    where
        'a: 'b,
    {
        self.ctx.executable_ids.get(&(address, name)).map(|entry| {
            // SAFETY: It is safe to cast its lifetime to the lifetime of the
            // execution guard. If the pointer existed before, it must still
            // be valid (during maintenance, if global arena is flushed, so is
            // the map).
            ExecutableIdRef {
                ptr: *entry.value(),
                _guard: PhantomData,
            }
        })
    }

    /// On interner cache miss, allocates the executable ID in the global arena
    /// and returns a pointer to:
    ///   - Allocated data if the ID was not interned before.
    ///   - Existing canonical pointer if the interned ID exists (this can
    ///     happen if there is a race condition).
    ///
    /// # Safety precondition
    ///
    /// During global arena reset, executable IDs map must be cleared as well.
    fn alloc_and_intern_executable_id<'b>(
        &'b self,
        address: AccountAddress,
        name: NameRef<'b>,
    ) -> ExecutableIdRef<'b>
    where
        'a: 'b,
    {
        // SAFETY: By construction, name pointer is valid. Note that **all**
        // arenas in the global arena pool are reset together so this is safe.
        let name = name.as_global_arena_ptr();

        // Allocate outside the lock to reduce contention. The leak is still
        // bounded to the number of concurrent workers, and therefore is
        // negligible in practice.
        let ptr = self.global_arena.alloc(ExecutableId { address, name });

        // SAFETY: We have just allocated the ID pointer, hence dereferencing
        // it to compute the hash and equality is safe (for transitive pointers
        // as well).
        let ptr = match self.ctx.executable_ids.entry(ExecutableIdInternerKey(ptr)) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => *entry.insert(ptr),
        };

        // SAFETY: The allocated pointer is trivially valid until the next
        // maintenance so it is safe to cast its lifetime to the lifetime of
        // the execution guard. If the pointer existed before, it must still
        // be valid (if global arena was flushed, so must have been the map).
        ExecutableIdRef {
            ptr,
            _guard: PhantomData,
        }
    }
}

impl Hash for ExecutableIdInternerKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // SAFETY: must be enforced by the caller.
        unsafe {
            let id = self.0.as_ref_unchecked();
            id.address.hash(state);
            id.name.as_ref_unchecked().hash(state);
        }
    }
}

impl PartialEq for ExecutableIdInternerKey {
    fn eq(&self, other: &Self) -> bool {
        // SAFETY: must be enforced by the caller.
        unsafe {
            let this_id = self.0.as_ref_unchecked();
            let other_id = other.0.as_ref_unchecked();
            this_id.address == other_id.address
                && this_id.name.as_ref_unchecked() == other_id.name.as_ref_unchecked()
        }
    }
}

impl Eq for ExecutableIdInternerKey {}

impl Equivalent<ExecutableIdInternerKey> for (&AccountAddress, &IdentStr) {
    fn equivalent(&self, key: &ExecutableIdInternerKey) -> bool {
        // SAFETY: must be enforced by the caller.
        unsafe {
            let key = key.0.as_ref_unchecked();
            self.0 == &key.address && self.1.as_str() == key.name.as_ref_unchecked()
        }
    }
}
