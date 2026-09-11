// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Cache of loaded scripts, keyed by the hash of their bytes.

use crate::context::loaded_module::LoadedModule;
use dashmap::{mapref::entry::Entry, DashMap};
use mono_move_alloc::LeakedBoxPtr;
use sha3::{Digest, Sha3_256};

/// The hash of a script's bytes, which identifies it in the cache.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScriptHash([u8; 32]);

impl ScriptHash {
    /// Hashes a script's bytes.
    pub fn of(script_code: &[u8]) -> Self {
        Self(Sha3_256::digest(script_code).into())
    }
}

/// Concurrent long-living cache of scripts, each loaded as a module.
//
// TODO(security, metering): unbounded. Memory is reclaimed only by
// maintenance, as for modules, but every distinct script a user sends adds an
// entry for the price of one transaction, so maintenance needs a size trigger.
pub(super) struct ScriptCache {
    // Keyed hashing: users choose the script bytes, so an unkeyed hash would
    // let them pick colliding keys.
    inner: DashMap<ScriptHash, LeakedBoxPtr<LoadedModule>, ahash::RandomState>,
}

impl ScriptCache {
    /// Creates an empty cache.
    pub(super) fn new() -> Self {
        Self {
            inner: DashMap::with_hasher(ahash::RandomState::default()),
        }
    }

    /// Inserts `module` under `hash`. On a race, frees the caller's box and
    /// returns the winner.
    pub(super) fn insert(
        &self,
        hash: ScriptHash,
        module: Box<LoadedModule>,
    ) -> LeakedBoxPtr<LoadedModule> {
        let leaked = LeakedBoxPtr::from_box(module);
        match self.inner.entry(hash) {
            Entry::Occupied(existing) => {
                // SAFETY: `leaked` is exclusive to this call and has no aliases.
                unsafe { leaked.free_unchecked() };
                *existing.get()
            },
            Entry::Vacant(vacant) => {
                vacant.insert(leaked);
                leaked
            },
        }
    }

    /// Returns the script cached under `hash`, if any.
    pub(super) fn get(&self, hash: &ScriptHash) -> Option<LeakedBoxPtr<LoadedModule>> {
        self.inner.get(hash).map(|entry| *entry.value())
    }

    /// Frees every cached script and clears the map.
    ///
    /// # Safety
    ///
    /// The caller must have exclusive access to the cache, with no live
    /// references to cached scripts.
    pub(super) unsafe fn clear(&self) {
        for entry in self.inner.iter() {
            // SAFETY: caller guarantees no outstanding references.
            unsafe { entry.value().free_unchecked() };
        }
        self.inner.clear();
    }
}
