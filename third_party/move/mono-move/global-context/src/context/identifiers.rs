// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! This submodule defines APIs to intern Move identifiers as arena-allocated
//! strings.
//!
//! # Safety model
//!
//! Identifiers are interned as canonical pointers. Public APIs return a scoped
//! pointer ([`ArenaRef`]) with lifetime of the execution guard. This prevents
//! use-after-free for interned identifiers at compile-time.
//!
//! Interning may happen concurrently, in which case it is guaranteed that all
//! workers agree on a single canonical pointer. To reduce contention for
//! better performance, arena allocations happen outside the lock, and so may
//! leak memory. However, any extra allocated memory is always bounded by the
//! number of concurrent workers and unique identifiers.
//!
//! The deduplication map where interned identifiers are stored is **always**
//! cleared when the arena backing the allocation is reset. Hence, it is safe
//! to dereference any arena-based pointers stored in the map.

use crate::{alloc::GlobalArenaPtr, context::ArenaRef, ExecutionGuard};
use dashmap::Equivalent;
use move_core_types::identifier::IdentStr;
use std::hash::{Hash, Hasher};

impl<'guard> ArenaRef<'guard, str> {
    /// Returns the inner string stored behind the reference.
    pub fn as_str(&self) -> &'guard str {
        // SAFETY: The lifetime on this reference guarantees that the execution
        // guard is still alive, which guarantees that the arena allocation is
        // still valid and there were no deallocations.
        unsafe { self.ptr.as_ref_unchecked() }
    }
}

impl<'ctx> ExecutionGuard<'ctx> {
    /// Interns Move identifier as a string and returns a reference to it, with
    /// lifetime scoped to the lifetime of [`ExecutionGuard`].
    ///
    /// On cache hit, returns a canonical pointer interned previously.
    /// On cache miss, allocates the string in the global arena and
    /// returns a pointer to:
    ///   - Allocated data if the string was not interned before.
    ///   - Existing canonical pointer if the interned string exists. Note that
    ///     the entry can exist due to a race condition because allocation is
    ///     done outside the lock (by design). In this case, extra allocations
    ///     are bounded to O(unique identifiers * number of workers) which is
    ///     negligible in practice.
    ///
    /// # Safety precondition
    ///
    /// During global arena reset, identifiers map must be cleared as well.
    pub fn intern_identifier<'guard>(&'guard self, identifier: &IdentStr) -> ArenaRef<'guard, str>
    where
        'ctx: 'guard,
    {
        let ptr = self.intern_identifier_internal(identifier);

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

impl<'ctx> ExecutionGuard<'ctx> {
    pub(super) fn intern_identifier_internal(&self, identifier: &IdentStr) -> GlobalArenaPtr<str> {
        // TODO:
        //   Consider checking that the identifier size is within bounds. While
        //   CompiledModule / CompiledScript deserializer enforces 256 byte
        //   limit (in new config), when coming from deserialized TypeTag from
        //   transaction payload there is no bound. It is not a big problem,
        //   but just makes spam attacks easier to intern some dummy data in the
        //   pool. In general, for type tag interning we might want to enforce
        //   that the modules which are specified actually exist on-chain. In
        //   existing VM we already do that to get ability information, but not
        //   here (for now), so that we ensure that there is no spam that can
        //   get in. However, there still can be a problem with speculative
        //   module publishing: if we speculatively intern new names, but the
        //   publish actually fails, we end up with spam on-chain.
        //   Note: this DoS is only possible via `init_module`. If we remove it
        //   or ensure no speculative data even for names ever get on-chain, we
        //   limit interned set to the on-chain data, so for DoS one actually
        //   has to publish modules (expensive).
        let str = identifier.as_str();

        // SAFETY: All existing keys/values are valid pointers because the map
        // is guaranteed to be cleared on arena's reset.
        if let Some(entry) = self.ctx.identifiers.get(&LookupKey(str)) {
            return *entry.value();
        }

        let ptr = self.global_arena.alloc_str(str);

        // SAFETY: We have just allocated the pointer, hence it is safe to wrap
        // it as a key and compute hash / equality. All existing keys are also
        // valid pointers because the map is cleared on arena's reset.
        *self
            .ctx
            .identifiers
            .entry(IdentifierInternerKey(ptr))
            .or_insert(ptr)
    }
}

/// Wraps allocated string pointer to implement structural hash and equality.
///
/// # Safety
///
/// Constructor must enforce the pointer points to the valid data and can be
/// safely dereferenced.
pub(super) struct IdentifierInternerKey(GlobalArenaPtr<str>);

// Wrapper to avoid orphan rule.
#[derive(Hash, PartialEq, Eq)]
struct LookupKey<'a>(&'a str);

impl Hash for IdentifierInternerKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // SAFETY: It is safe to dereference the pointer because the caller
        // ensures it remains valid during the lifetime of the key.
        let str = unsafe { self.0.as_ref_unchecked() };
        str.hash(state);
    }
}

impl PartialEq for IdentifierInternerKey {
    fn eq(&self, other: &Self) -> bool {
        // SAFETY: It is safe to dereference the pointer because the caller
        // ensures it remains valid during the lifetime of the key.
        unsafe { self.0.as_ref_unchecked() == other.0.as_ref_unchecked() }
    }
}

// PartialEq implementation above is a full equivalence relation.
impl Eq for IdentifierInternerKey {}

impl Equivalent<IdentifierInternerKey> for LookupKey<'_> {
    fn equivalent(&self, key: &IdentifierInternerKey) -> bool {
        // SAFETY: It is safe to dereference the pointer because the caller
        // ensures it remains valid during the lifetime of the key.
        let key = unsafe { key.0.as_ref_unchecked() };
        self.0 == key
    }
}
