// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Executable cache for storing compiled module metadata.
//!
//! This module provides a two-tier cache (hot + cold) optimized for read-heavy
//! workloads:
//! - **Hot tier**: [`HashMap`] for committed executables, immutable during
//!   execution.
//! - **Cold tier**: [`DashMap`] for modules loaded from storage or newly
//!   published, which also supports concurrent writes.
//!
//! ## Mono Function Cache
//!
//! Each [`Executable`] carries a per-module mono cache (a [`DashMap`] keyed
//! by `(function_id_ptr, type_list_ptr)`) for lazily monomorphized generic
//! functions. These functions are [`Box`]-allocated for individual eviction.
//!
//! A global `mono_total` [`AtomicUsize`] counter (incremented on insert,
//! decremented on eviction, reset on full flush) drives TTL-based eviction
//! at maintenance time: when the count exceeds the configured threshold,
//! [`ExecutableCache::evict_stale_monomorphized`] sweeps all live executables
//! with a single `retain` pass, evicting entries not accessed within
//! `mono_eviction_ttl_blocks` blocks of the current block index.

use crate::{
    alloc::LeakedBoxPtr,
    executable::Executable,
    types::ExecutableCacheKey,
    version::{BlockIndex, Version},
};
use ahash::HashMap;
use dashmap::DashMap;
use smallvec::{smallvec, SmallVec};
use std::{
    cmp::Ordering,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering as AtomicOrdering},
};

/// Entry in the executable cache. Stores a pointer to the executable and the
/// version when it was inserted.
struct Entry {
    executable: LeakedBoxPtr<Executable>,
    version: Version,
}

impl Entry {
    /// Returns a new cache entry.
    fn new(executable: Box<Executable>, version: Version) -> Self {
        let executable = LeakedBoxPtr::from_box(executable);
        Self {
            executable,
            version,
        }
    }

    /// Returns the reference to executable with explicit lifetime.
    ///
    /// # Safety
    ///
    /// The caller must ensure the returned reference does not outlive
    /// the executable cache and is dropped before any memory deallocation,
    /// e.g., flush operations.
    #[inline]
    unsafe fn as_ref_unchecked<'a>(&self) -> &'a Executable {
        // SAFETY:
        //   Pointer was created from `Box::leak`, valid until `free` is called.
        //   Caller ensures the lifetime 'a is bounded by ExecutionContext.
        unsafe { self.executable.as_ref_unchecked() }
    }

    /// Frees memory from this executable.
    ///
    /// # Safety
    ///
    /// No references to this executable may exist past this point.
    unsafe fn free(self) {
        // SAFETY: Caller guarantees no references exist.
        unsafe { self.executable.free_unchecked() }
    }
}

// SAFETY:
//
// `NonNull<Executable>` is !Send and !Sync by default because raw pointers
// do not carry ownership or thread-safety guarantees.
//
// These implementations are safe because:
// 1. The `Executable` is heap-allocated via `Box::leak`, giving it a stable address.
// 2. The `Executable` is immutable after construction (no internal mutation).
// 3. Access is synchronized by the cache structure:
//    - hot: inside `RwLock<Context>` (read guard for access)
//    - cold: `DashMap` provides internal synchronization
// 4. Deallocation only happens in `MaintenanceContext` which holds write lock,
//    ensuring no concurrent readers exist.
unsafe impl Send for Entry {}
unsafe impl Sync for Entry {}

/// Entry in the cold cache.
struct ColdEntry {
    inner: Entry,
}

impl ColdEntry {
    /// Creates a new cold entry.
    fn new(executable: Box<Executable>, version: Version) -> Self {
        Self {
            inner: Entry::new(executable, version),
        }
    }

    /// Frees memory from this entry.
    ///
    /// # Safety
    ///
    /// No references to this executable may exist past this point.
    unsafe fn free(self) {
        // SAFETY: Caller guarantees no references exist.
        unsafe {
            self.inner.free();
        }
    }
}

/// Entry in the hot cache.
struct HotEntry {
    inner: Entry,
    /// Marked true when cold entry supersedes this.
    stale: AtomicBool,
}

impl HotEntry {
    /// Creates a new hot entry from the cold entry.
    fn from_cold(entry: ColdEntry) -> Self {
        let ColdEntry { inner } = entry;
        Self {
            inner,
            stale: AtomicBool::new(false),
        }
    }

    /// Frees memory from this entry.
    ///
    /// # Safety
    ///
    /// No references to this executable may exist past this point.
    unsafe fn free(self) {
        // SAFETY: Caller guarantees no references exist.
        unsafe {
            self.inner.free();
        }
    }
}

/// Cache storing executable data.
pub struct ExecutableCache {
    /// Immutable map for fast reads. Stores non-speculative, committed executables.
    /// Can only be mutated during maintenance.
    hot: HashMap<ExecutableCacheKey, HotEntry>,
    /// Map for cold entries: either published modules, or cache misses. Executables
    /// from this map can be promoted to hot tier, if needed.
    cold: DashMap<ExecutableCacheKey, SmallVec<[ColdEntry; 2]>>,
    /// Total monomorphized functions across all hot executables.
    /// Incremented on cache insert (Vacant branch), decremented on eviction,
    /// reset to 0 on flush. Relaxed ordering suffices — the count is
    /// approximate and only used for threshold comparisons.
    mono_total: AtomicUsize,
}

impl ExecutableCache {
    /// Creates a new empty cache.
    pub fn new() -> Self {
        Self {
            hot: HashMap::default(),
            cold: DashMap::new(),
            mono_total: AtomicUsize::new(0),
        }
    }

    /// Returns true if both hot and cold caches are empty.
    pub(crate) fn is_empty(&self) -> bool {
        self.hot.is_empty() && self.cold.is_empty()
    }

    /// Returns the executable from the hot cache if it exists and is not stale.
    #[inline]
    fn get_hot(&self, key: ExecutableCacheKey) -> Option<&Executable> {
        let entry = self.hot.get(&key)?;
        if entry.stale.load(AtomicOrdering::Acquire) {
            return None;
        }
        Some(unsafe { entry.inner.as_ref_unchecked() })
    }

    /// Returns the latest available version of executable.
    #[inline]
    pub fn get_latest(&self, key: ExecutableCacheKey) -> Option<&Executable> {
        self.get_hot(key).or_else(|| {
            self.cold.get(&key).and_then(|versions| {
                // SAFETY:
                //   The NonNull pointer in Entry points to Box::leaked memory, which is
                //   stable and only freed during MaintenanceContext. This method is called
                //   from ExecutionContext which holds RwLockReadGuard, preventing flush.
                //   Therefore, extending the lifetime to match ExecutionContext is safe.
                versions
                    .last()
                    .map(|e| unsafe { e.inner.as_ref_unchecked() })
            })
        })
    }

    /// Returns the version of executable at specified version.
    #[inline]
    pub fn get_at_version(&self, key: ExecutableCacheKey, version: Version) -> Option<&Executable> {
        self.get_hot(key)
            .and_then(|exec| {
                // Check if hot entry is valid for this version.
                if self.hot.get(&key).unwrap().inner.version <= version {
                    Some(exec)
                } else {
                    None
                }
            })
            .or_else(|| {
                self.cold.get(&key).and_then(|versions| {
                    // SAFETY:
                    //   The NonNull pointer in Entry points to Box::leaked memory, which is
                    //   stable and only freed during MaintenanceContext. This method is called
                    //   from ExecutionContext which holds RwLockReadGuard, preventing flush.
                    //   Therefore, extending the lifetime to match ExecutionContext is safe.
                    versions
                        .iter()
                        .rev()
                        .find(|e| e.inner.version <= version)
                        .map(|e| unsafe { e.inner.as_ref_unchecked() })
                })
            })
    }

    #[inline]
    pub fn contains(&self, key: ExecutableCacheKey) -> bool {
        self.hot.contains_key(&key) || self.cold.contains_key(&key)
    }

    /// If insertion version is greater than existing version: inserts executable
    /// and returns the reference to it.
    ///
    /// # Panics
    ///
    /// If insertion version is equal or is smaller than existing version.
    pub(crate) fn insert_cold(
        &self,
        key: ExecutableCacheKey,
        executable: Box<Executable>,
        version: Version,
    ) -> &Executable {
        let ptr = match self.cold.entry(key) {
            dashmap::Entry::Vacant(entry) => {
                let cold = ColdEntry::new(executable, version);
                let ptr = cold.inner.executable;
                entry.insert(smallvec![cold]);
                ptr
            },
            dashmap::Entry::Occupied(mut entry) => {
                let entries = entry.get_mut();
                if let Some(prev_cold) = entries.last() {
                    match version.cmp(&prev_cold.inner.version) {
                        Ordering::Greater => {
                            // Proceed with insertion.
                        },
                        Ordering::Equal | Ordering::Less => {
                            unreachable!("Inserted versions should always have higher versions");
                        },
                    }
                }

                let cold = ColdEntry::new(executable, version);
                let ptr = cold.inner.executable;
                entries.push(cold);
                ptr
            },
        };

        // Mark hot as stale if it exists. We mark **after** insertion to make sure
        // reads see the inserted value.
        if let Some(hot) = self.hot.get(&key) {
            hot.stale.store(true, AtomicOrdering::Release);
        }

        // SAFETY:
        //   The executable was just inserted, the pointer is valid.
        //   It will remain valid until freed during maintenance.
        unsafe { ptr.as_ref_unchecked() }
    }

    /// Returns the total number of cached monomorphized functions across all
    /// hot executables. O(1) — reads the global atomic counter.
    pub(crate) fn total_monomorphized_function_count(&self) -> usize {
        self.mono_total.load(AtomicOrdering::Relaxed)
    }

    /// Returns a reference to the global monomorphized-function counter so
    /// callers can increment it directly on cache insert.
    pub(crate) fn mono_total_ref(&self) -> &AtomicUsize {
        &self.mono_total
    }

    /// Evicts all monomorphized functions not accessed since `cutoff`
    /// (inclusive) across all hot executables. Uses `DashMap::retain` per
    /// executable — O(N) single pass, no intermediate allocations.
    /// Returns the total number evicted.
    ///
    /// # Safety
    ///
    /// Exclusive (maintenance) access required. No execution contexts may exist.
    /// Must be called after `compact_and_promote` (cold tier is empty).
    pub(crate) unsafe fn evict_stale_monomorphized(&self, cutoff: BlockIndex) -> usize {
        let mut total_evicted = 0;
        for hot in self.hot.values() {
            // SAFETY: maintenance has exclusive access; pointer is valid and
            // no concurrent references to individual functions exist.
            let exec: &Executable = unsafe { hot.inner.as_ref_unchecked() };
            total_evicted += unsafe { exec.evict_stale_entries(cutoff) };
        }
        self.mono_total
            .fetch_sub(total_evicted, AtomicOrdering::Relaxed);
        total_evicted
    }

    /// Compactifies the cache by promoting cold entries to hot, and freeing
    /// stale versions. Returns the number of promoted and freed executables.
    ///
    /// # Safety
    ///
    /// Exclusive (single-threaded) access required.
    pub(crate) unsafe fn compact_and_promote(&mut self) -> (usize, usize) {
        let mut promoted = 0;
        let mut freed = 0;

        let keys = self.cold.iter().map(|e| *e.key()).collect::<Vec<_>>();
        for key in keys {
            let mut versions = match self.cold.remove(&key) {
                Some((_, versions)) if !versions.is_empty() => versions,
                Some(_) | None => {
                    continue;
                },
            };

            let latest_cold = versions.pop().expect("There is at least 1 cold version");
            let hot = HotEntry::from_cold(latest_cold);

            // Insert into hot, freeing old hot entry if present.
            if let Some(prev_hot) = self.hot.insert(key, hot) {
                // SAFETY:
                //   Caller guarantees exclusive access (maintenance), so no references
                //   should exist.
                unsafe { prev_hot.free() };
                freed += 1;
            }
            promoted += 1;

            // Free all remaining (superseded) cold versions.
            for cold in versions {
                // SAFETY:
                //   Caller guarantees exclusive access (maintenance), so no references
                //   should exist.
                unsafe { cold.free() };
                freed += 1;
            }
        }

        (promoted, freed)
    }

    /// Flushes all executables.
    ///
    /// # Safety
    ///
    /// Exclusive (single-threaded) access required.
    pub(crate) unsafe fn flush(&mut self) {
        for (_, hot) in self.hot.drain() {
            // SAFETY:
            //   Caller guarantees exclusive access (maintenance), so no references
            //   should exist. Executable::drop frees mono functions automatically.
            unsafe { hot.free() };
        }

        let keys = self.cold.iter().map(|e| *e.key()).collect::<Vec<_>>();
        for key in keys {
            if let Some((_, versions)) = self.cold.remove(&key) {
                for cold in versions {
                    // SAFETY:
                    //   Caller guarantees exclusive access (maintenance), so no references
                    //   should exist. Executable::drop frees mono functions automatically.
                    unsafe { cold.free() };
                }
            }
        }

        // All monomorphized functions have been freed via Executable::drop above.
        self.mono_total.store(0, AtomicOrdering::Relaxed);

        assert!(self.hot.is_empty());
        assert!(self.cold.is_empty());
    }
}

impl Default for ExecutableCache {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ExecutableCache {
    fn drop(&mut self) {
        // SAFETY:
        // - Drop has exclusive access (&mut self).
        // - No executable references outside can exist (drop is ongoing).
        unsafe {
            self.flush();
        }
    }
}
