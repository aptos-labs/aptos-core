// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Cache of loaded modules, keyed by module IDs.
//!
//! An entry holds the single version of a module visible for the whole block:
//! code upgrades are applied at the maintenance barrier, with no execution in
//! flight, so an entry never changes while a guard is held.
//!
//! Other loaded modules hold direct pointers into a cached module's lowered
//! code. Hence, it is crucial that the only eviction is a clear of the whole
//! cache.

use crate::context::loaded_module::LoadedModule;
use dashmap::{mapref::entry::Entry, DashMap};
use mono_move_alloc::LeakedBoxPtr;
use mono_move_core::interner::InternedModuleId;

/// Concurrent long-living loaded module cache.
pub(super) struct ModuleCache {
    // Uses fxhash because the keys are already well-distributed arena
    // pointers, so a simple, fast hash is sufficient.
    inner: DashMap<InternedModuleId, LeakedBoxPtr<LoadedModule>, fxhash::FxBuildHasher>,
}

impl ModuleCache {
    /// Creates an empty cache.
    pub(super) fn new() -> Self {
        Self {
            inner: DashMap::with_hasher(fxhash::FxBuildHasher::default()),
        }
    }

    /// Inserts `module` under its own ID. On a race, frees the caller's box
    /// and returns the winner, so every caller ends up with the same
    /// canonical pointer.
    ///
    /// Insertion takes the shard write lock and lookup takes the shard read
    /// lock, which publishes the module's initialization to every reader.
    pub(super) fn insert(&self, module: Box<LoadedModule>) -> LeakedBoxPtr<LoadedModule> {
        let id = module.id();
        let leaked = LeakedBoxPtr::from_box(module);
        let winner = match self.inner.entry(id) {
            Entry::Occupied(existing) => Some(*existing.get()),
            Entry::Vacant(vacant) => {
                vacant.insert(leaked);
                None
            },
        };
        match winner {
            // Freed once the shard guard is dropped: dropping a module
            // deallocates its whole IR, which would block the shard.
            Some(winner) => {
                // SAFETY: `leaked` is exclusive to this call and has no aliases.
                unsafe { leaked.free_unchecked() };
                winner
            },
            None => leaked,
        }
    }

    /// Returns the module cached under the specified ID, if any.
    pub(super) fn get(&self, id: InternedModuleId) -> Option<LeakedBoxPtr<LoadedModule>> {
        self.inner.get(&id).map(|entry| *entry.value())
    }

    /// Frees every cached module and clears the map.
    ///
    /// # Safety
    ///
    /// 1. The caller must have exclusive access to the cache.
    /// 2. The caller must ensure no live references to cached loaded modules
    ///    exist.
    pub(super) unsafe fn clear(&self) {
        for entry in self.inner.iter() {
            // SAFETY: caller guarantees no outstanding references.
            unsafe { entry.value().free_unchecked() };
        }
        self.inner.clear();
    }
}
