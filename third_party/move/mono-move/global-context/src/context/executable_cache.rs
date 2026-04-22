// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Cache for loaded executables, keyed by executable IDs.
//!
//! Executables are stored as [`LeakedBoxPtr`]s so that their heap addresses
//! remain stable across concurrent reads. During the maintenance phase, all
//! cached executables are freed before the global arena (which backs the keys)
//! is reset.

use dashmap::DashMap;
use mono_move_alloc::{GlobalArenaPtr, LeakedBoxPtr};
use mono_move_core::{Executable, ExecutableId};

/// Concurrent long-living executable cache.
///
// TODO:
//   1. Support speculative writes for Zaptos optimitstic pipeline.
//   2. Support lock-free hot tier?
pub(super) struct ExecutableCache {
    // Uses fxhash because the keys are already well-distributed arena
    // pointers, so a simple, fast hash is sufficient.
    inner: DashMap<GlobalArenaPtr<ExecutableId>, LeakedBoxPtr<Executable>, fxhash::FxBuildHasher>,
}

impl ExecutableCache {
    /// Creates an empty executable cache.
    pub(super) fn new() -> Self {
        Self {
            inner: DashMap::with_hasher(fxhash::FxBuildHasher::default()),
        }
    }

    /// Inserts the executable into the cache if the key is not already present,
    /// leaking the box to obtain a stable pointer. On cache hit, the existing
    /// pointer is returned and the caller's box is dropped.
    pub(super) fn insert(
        &self,
        key: GlobalArenaPtr<ExecutableId>,
        executable: Box<Executable>,
    ) -> LeakedBoxPtr<Executable> {
        *self
            .inner
            .entry(key)
            .or_insert_with(|| LeakedBoxPtr::from_box(executable))
    }

    /// Returns the leaked pointer for the given key, if present.
    pub(super) fn get(
        &self,
        key: GlobalArenaPtr<ExecutableId>,
    ) -> Option<LeakedBoxPtr<Executable>> {
        self.inner.get(&key).map(|entry| *entry.value())
    }

    /// Frees all cached executables and clears the map.
    ///
    /// # Safety
    ///
    /// 1. The caller must have exclusive access to the cache.
    /// 2. The caller must ensure no live references to cached executables
    ///    exist.
    pub(super) unsafe fn clear(&self) {
        for entry in self.inner.iter() {
            // SAFETY: The caller guarantees no outstanding references.
            unsafe {
                entry.value().free_unchecked();
            }
        }
        self.inner.clear();
    }
}
