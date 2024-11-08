// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::explicit_sync_wrapper::ExplicitSyncWrapper;
use aptos_types::error::PanicError;
use crossbeam::utils::CachePadded;
use hashbrown::HashMap;
use move_vm_types::code::{ModuleCode, WithSize};
use std::{
    hash::Hash,
    ops::Deref,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
};

/// Entry stored in [GlobalModuleCache].
struct Entry<DC, VC, E> {
    /// True if this code is "valid" within the block execution context (i.e., there has been no
    /// republishing of this module so far). If false, executor needs to read the module from the
    /// sync/unsync module caches.
    valid: CachePadded<AtomicBool>,
    /// Cached verified module. Must always be verified.
    module: CachePadded<Arc<ModuleCode<DC, VC, E>>>,
}

impl<DC, VC, E> Entry<DC, VC, E>
where
    VC: Deref<Target = Arc<DC>>,
    E: WithSize,
{
    /// Returns a new valid module. Returns a (panic) error if the module is not verified.
    fn new(module: Arc<ModuleCode<DC, VC, E>>) -> Result<Self, PanicError> {
        if !module.code().is_verified() {
            return Err(PanicError::CodeInvariantError(
                "Module code is not verified".to_string(),
            ));
        }

        Ok(Self {
            valid: CachePadded::new(AtomicBool::new(true)),
            module: CachePadded::new(module),
        })
    }

    /// Marks the module as invalid.
    fn mark_invalid(&self) {
        self.valid.store(false, Ordering::Release)
    }

    /// Returns true if the module is valid.
    pub(crate) fn is_valid(&self) -> bool {
        self.valid.load(Ordering::Acquire)
    }

    /// Returns the module code stored is this [Entry].
    fn module_code(&self) -> &Arc<ModuleCode<DC, VC, E>> {
        self.module.deref()
    }
}

/// A read-only module cache for verified code, that can be accessed concurrently within the block.
/// Can only be modified safely at block boundaries.
pub struct GlobalModuleCache<K, DC, VC, E> {
    /// Module cache containing the verified code.
    module_cache: ExplicitSyncWrapper<HashMap<K, Entry<DC, VC, E>>>,
    /// Sum of serialized sizes (in bytes) of all cached modules.
    size: AtomicUsize,
}

impl<K, DC, VC, E> GlobalModuleCache<K, DC, VC, E>
where
    K: Hash + Eq + Clone,
    VC: Deref<Target = Arc<DC>>,
    E: WithSize,
{
    /// Returns new empty module cache.
    pub fn empty() -> Self {
        Self {
            module_cache: ExplicitSyncWrapper::new(HashMap::new()),
            size: AtomicUsize::new(0),
        }
    }

    /// Returns true if the key exists in cache and the corresponding module is valid.
    pub fn contains_valid(&self, key: &K) -> bool {
        self.module_cache
            .acquire()
            .get(key)
            .is_some_and(|entry| entry.is_valid())
    }

    /// Marks the cached module (if it exists) as invalid. As a result, all subsequent calls to the
    /// cache for the associated key will result in a cache miss. If an entry does not to exist, is
    /// a no-op.
    pub fn mark_invalid_if_contains(&self, key: &K) {
        if let Some(entry) = self.module_cache.acquire().get(key) {
            entry.mark_invalid();
        }
    }

    /// Returns the module stored in cache. If the module has not been cached, or it exists but it
    /// is not valid, [None] is returned.
    pub fn get(&self, key: &K) -> Option<Arc<ModuleCode<DC, VC, E>>> {
        self.module_cache
            .acquire()
            .get(key)
            .and_then(|entry| entry.is_valid().then(|| entry.module_code().clone()))
    }

    /// Flushes the cache. Should never be called throughout block-execution. Use with caution.
    pub fn flush_unchecked(&self) {
        self.module_cache.acquire().clear();
        self.size.store(0, Ordering::Relaxed);
    }

    /// Returns the number of entries in the cache.
    pub fn num_modules(&self) -> usize {
        self.module_cache.acquire().len()
    }

    /// Returns the sum of serialized sizes of modules stored in cache.
    pub fn size_in_bytes(&self) -> usize {
        self.size.load(Ordering::Relaxed)
    }

    /// Returns true if the sum of serialized sizes of modules stored in cache is greater than the
    /// specified value.
    pub fn size_in_bytes_is_greater_than(&self, size: usize) -> bool {
        self.size_in_bytes() > size
    }

    /// Inserts modules into the cache. Should never be called throughout block-execution. Use with
    /// caution.
    ///
    /// Notes:
    ///   1. Only verified modules are inserted.
    ///   2. Valid modules should not be removed, and new modules should have unique ownership. If
    ///      these constraints are violated, a panic error is returned.
    pub fn insert_verified_unchecked(
        &self,
        modules: impl Iterator<Item = (K, Arc<ModuleCode<DC, VC, E>>)>,
    ) -> Result<(), PanicError> {
        use hashbrown::hash_map::Entry::*;

        let mut guard = self.module_cache.acquire();
        let module_cache = guard.dereference_mut();

        for (key, module) in modules {
            if let Occupied(entry) = module_cache.entry(key.clone()) {
                if entry.get().is_valid() {
                    return Err(PanicError::CodeInvariantError(
                        "Should never overwrite a valid module".to_string(),
                    ));
                } else {
                    // Otherwise, remove the invalid entry.
                    let size = entry.get().module_code().extension().size_in_bytes();
                    self.size.fetch_sub(size, Ordering::Relaxed);
                    entry.remove();
                }
            }

            if module.code().is_verified() {
                self.size
                    .fetch_add(module.extension().size_in_bytes(), Ordering::Relaxed);
                let entry =
                    Entry::new(module).expect("Module has been checked and must be verified");
                let prev = module_cache.insert(key.clone(), entry);

                // At this point, we must have removed the entry, or returned a panic error.
                assert!(prev.is_none())
            }
        }
        Ok(())
    }

    /// Insert the module to cache. Used for tests only.
    #[cfg(any(test, feature = "testing"))]
    pub fn insert(&self, key: K, module: Arc<ModuleCode<DC, VC, E>>) {
        self.size
            .fetch_add(module.extension().size_in_bytes(), Ordering::Relaxed);
        self.module_cache.acquire().insert(
            key,
            Entry::new(module).expect("Module code should be verified"),
        );
    }

    /// Removes the module from cache. Used for tests only.
    #[cfg(any(test, feature = "testing"))]
    pub fn remove(&self, key: &K) {
        if let Some(entry) = self.module_cache.acquire().remove(key) {
            self.size.fetch_sub(
                entry.module_code().extension().size_in_bytes(),
                Ordering::Relaxed,
            );
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use claims::{assert_err, assert_ok};
    use move_vm_types::code::{mock_deserialized_code, mock_verified_code, MockExtension};

    #[test]
    fn test_entry_new() {
        assert!(Entry::new(mock_deserialized_code(0, MockExtension::new(8))).is_err());
        assert!(Entry::new(mock_verified_code(0, MockExtension::new(8))).is_ok());
    }

    #[test]
    fn test_entry_mark_invalid() {
        let entry = assert_ok!(Entry::new(mock_verified_code(0, MockExtension::new(8))));
        assert!(entry.is_valid());

        entry.mark_invalid();
        assert!(!entry.is_valid());
    }

    #[test]
    fn test_cache_contains_valid_and_get() {
        let cache = GlobalModuleCache::empty();

        // Set the state.
        cache.insert(0, mock_verified_code(0, MockExtension::new(8)));
        cache.insert(1, mock_verified_code(1, MockExtension::new(8)));
        cache.mark_invalid_if_contains(&1);

        assert_eq!(cache.num_modules(), 2);

        assert!(cache.contains_valid(&0));
        assert!(!cache.contains_valid(&1));
        assert!(!cache.contains_valid(&3));

        assert!(cache.get(&0).is_some());
        assert!(cache.get(&1).is_none());
        assert!(cache.get(&3).is_none());
    }

    #[test]
    fn test_cache_sizes_and_flush_unchecked() {
        let cache = GlobalModuleCache::empty();
        assert_eq!(cache.num_modules(), 0);
        assert_eq!(cache.size_in_bytes(), 0);

        cache.insert(0, mock_verified_code(0, MockExtension::new(8)));
        cache.insert(1, mock_verified_code(1, MockExtension::new(16)));
        cache.insert(2, mock_verified_code(2, MockExtension::new(8)));
        assert_eq!(cache.num_modules(), 3);
        assert_eq!(cache.size_in_bytes(), 32);

        cache.remove(&2);
        assert_eq!(cache.num_modules(), 2);
        assert_eq!(cache.size_in_bytes(), 24);

        assert!(cache.size_in_bytes_is_greater_than(23));
        assert!(!cache.size_in_bytes_is_greater_than(24));
        assert!(!cache.size_in_bytes_is_greater_than(25));

        cache.flush_unchecked();
        assert_eq!(cache.num_modules(), 0);
        assert_eq!(cache.size_in_bytes(), 0);
    }

    #[test]
    fn test_cache_insert_verified_unchecked() {
        let cache = GlobalModuleCache::empty();

        let mut new_modules = vec![];
        for i in 0..10 {
            new_modules.push((i, mock_verified_code(i, MockExtension::new(8))));
        }
        assert!(cache
            .insert_verified_unchecked(new_modules.into_iter())
            .is_ok());

        assert_eq!(cache.num_modules(), 10);
        assert_eq!(cache.size_in_bytes(), 80);
    }

    #[test]
    fn test_cache_insert_verified_unchecked_does_not_add_deserialized_code() {
        let cache = GlobalModuleCache::empty();

        let deserialized_modules = vec![(0, mock_deserialized_code(0, MockExtension::new(8)))];
        assert_ok!(cache.insert_verified_unchecked(deserialized_modules.into_iter()));

        assert_eq!(cache.num_modules(), 0);
        assert_eq!(cache.size_in_bytes(), 0);
    }

    #[test]
    fn test_cache_insert_verified_unchecked_does_not_override_valid_modules() {
        let cache = GlobalModuleCache::empty();

        cache.insert(0, mock_verified_code(0, MockExtension::new(8)));
        assert_eq!(cache.num_modules(), 1);
        assert_eq!(cache.size_in_bytes(), 8);

        let new_modules = vec![(0, mock_verified_code(100, MockExtension::new(32)))];
        assert_err!(cache.insert_verified_unchecked(new_modules.into_iter()));
    }

    #[test]
    fn test_cache_insert_verified_unchecked_overrides_invalid_modules() {
        let cache = GlobalModuleCache::empty();

        cache.insert(0, mock_verified_code(0, MockExtension::new(8)));
        cache.mark_invalid_if_contains(&0);
        assert_eq!(cache.num_modules(), 1);
        assert_eq!(cache.size_in_bytes(), 8);

        let new_modules = vec![(0, mock_verified_code(100, MockExtension::new(32)))];
        assert_ok!(cache.insert_verified_unchecked(new_modules.into_iter()));

        assert_eq!(cache.num_modules(), 1);
        assert_eq!(cache.size_in_bytes(), 32);
    }
}
