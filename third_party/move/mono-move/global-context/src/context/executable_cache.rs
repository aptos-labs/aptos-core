// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Cache of loaded executables, keyed by executable IDs.
//!
//! Each entry is a stable pointer to a slot which stores multiple versions
//! of executables. A slot can be:
//!   1. Empty (e.g., cleared by eviction). Readers treat this as a cache
//!      miss and repopulate the slot.
//!   2. Non-empty. Readers select correct version of executable they need.
//!
//! Because of upgrades, slot can contain more than 1 version of executable at
//! a time.
//!
//! Other executables may point to their dependency slots. Hence, it is crucial
//! that the eviction from the cache never removes the slots themselves, unless
//! the whole cache is cleared.

use anyhow::{anyhow, Result};
use dashmap::DashMap;
use mono_move_alloc::{GlobalArenaPtr, LeakedBoxPtr, VersionedLeakedBoxPtr};
use mono_move_core::{Executable, ExecutableId, ExecutableSlot};

/// Concurrent long-living executable cache.
///
// TODO:
//   1. Support speculative writes for Zaptos optimistic pipeline.
//   2. Support lock-free hot tier?
pub(super) struct ExecutableCache {
    // Uses fxhash because the keys are already well-distributed arena
    // pointers, so a simple, fast hash is sufficient.
    inner: DashMap<GlobalArenaPtr<ExecutableId>, ExecutableSlot, fxhash::FxBuildHasher>,
}

impl ExecutableCache {
    /// Creates an empty executable cache.
    pub(super) fn new() -> Self {
        Self {
            inner: DashMap::with_hasher(fxhash::FxBuildHasher::default()),
        }
    }

    /// Returns the slot for `key`, creating an empty one if absent.
    ///
    /// The returned pointer is stable for the lifetime of the cache.
    /// Takes a shard write lock on the create path.
    pub(super) fn get_or_create_slot(&self, key: GlobalArenaPtr<ExecutableId>) -> ExecutableSlot {
        *self
            .inner
            .entry(key)
            .or_insert_with(|| LeakedBoxPtr::from_box(Box::new(VersionedLeakedBoxPtr::new())))
    }

    /// Inserts `executable` into the slot for `key`, creating the slot if
    /// needed. On race, frees the caller's box and returns the winner.
    pub(super) fn insert(
        &self,
        key: GlobalArenaPtr<ExecutableId>,
        executable: Box<Executable>,
    ) -> Result<LeakedBoxPtr<Executable>> {
        if let Some(existing) = self.get(key) {
            return Ok(existing);
        }

        let slot_ptr = self.get_or_create_slot(key);
        // SAFETY: slots are freed only at maintenance, excluded by any live
        // execution guard.
        let slot = unsafe { slot_ptr.as_ref_unchecked() };

        let leaked = LeakedBoxPtr::from_box(executable);
        match slot.init(leaked) {
            Ok(()) => Ok(leaked),
            Err(loser) => {
                // SAFETY: `loser` is exclusive to this call and has no aliases.
                unsafe { loser.free_unchecked() };
                slot.load()
                    .ok_or_else(|| anyhow!("cache invariant violated: slot null after CAS failure"))
            },
        }
    }

    /// Returns the current content of the slot for `key`, if any.
    pub(super) fn get(
        &self,
        key: GlobalArenaPtr<ExecutableId>,
    ) -> Option<LeakedBoxPtr<Executable>> {
        let slot_ptr = self.inner.get(&key).map(|e| *e.value())?;
        // SAFETY: slots are stable for the cache's lifetime.
        let slot = unsafe { slot_ptr.as_ref_unchecked() };
        slot.load()
    }

    /// Frees all executable content stored in slots, frees the slots
    /// themselves, and clears the map.
    ///
    /// # Safety
    ///
    /// 1. The caller must have exclusive access to the cache.
    /// 2. The caller must ensure no live references to cached executables or
    ///    slots exist.
    pub(super) unsafe fn clear(&self) {
        for entry in self.inner.iter() {
            let slot_ptr: ExecutableSlot = *entry.value();

            // SAFETY: caller guarantees no outstanding references.
            let content = unsafe { slot_ptr.as_ref_unchecked() }.clear();
            if let Some(content) = content {
                unsafe {
                    content.free_unchecked();
                }
            }

            unsafe {
                slot_ptr.free_unchecked();
            }
        }
        self.inner.clear();
    }
}
