// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::explicit_sync_wrapper::ExplicitSyncWrapper;
use aptos_mvhashmap::types::TxnIndex;
use aptos_types::error::PanicError;
use crossbeam::utils::CachePadded;
use hashbrown::HashMap;
use move_vm_types::code::ModuleCode;
use std::{
    hash::Hash,
    ops::Deref,
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc,
    },
};

/// Module code stored in cross-block module cache.
struct ImmutableModuleCode<DC, VC, E> {
    /// True if this code is "valid" within the block execution context (i.e, there has been no
    /// republishing of this module so far). If false, executor needs to read the module from the
    /// sync/unsync module caches.
    valid: CachePadded<AtomicBool>,
    /// Represents the generation of the module cache for which this entry has been validated
    /// against the global state. For example, if the generation of the cache is 1, but the entry
    /// has generation 0, it should be re-validated against the state and has its generation reset
    /// accordingly.
    generation: CachePadded<AtomicU32>,
    /// Cached verified module. While [ModuleCode] type is used, the following invariants always
    /// hold:
    ///    1. Module's version is [None] (storage version).
    ///    2. Module's code is always verified.
    module: CachePadded<Arc<ModuleCode<DC, VC, E, Option<TxnIndex>>>>,
}

impl<DC, VC, E> ImmutableModuleCode<DC, VC, E>
where
    VC: Deref<Target = Arc<DC>>,
{
    /// Returns a new valid module. Returns a (panic) error if the module is not verified or has
    /// non-storage version.
    fn new(
        module: Arc<ModuleCode<DC, VC, E, Option<TxnIndex>>>,
        generation: u32,
    ) -> Result<Self, PanicError> {
        if !module.code().is_verified() || module.version().is_some() {
            let msg = format!(
                "Invariant violated for immutable module code : verified ({}), version({:?})",
                module.code().is_verified(),
                module.version()
            );
            return Err(PanicError::CodeInvariantError(msg));
        }

        Ok(Self {
            valid: CachePadded::new(AtomicBool::new(true)),
            generation: CachePadded::new(AtomicU32::new(generation)),
            module: CachePadded::new(module),
        })
    }

    /// Marks the module as invalid.
    fn mark_invalid(&self) {
        self.valid.store(false, Ordering::Release)
    }

    /// Returns the generation of this module.
    pub(crate) fn generation(&self) -> u32 {
        self.generation.load(Ordering::Acquire)
    }

    /// Resets the generation of this module.
    fn set_generation(&self, new_generation: u32) {
        self.generation.store(new_generation, Ordering::Release);
    }

    /// Returns true if the module is valid.
    pub(crate) fn is_valid(&self) -> bool {
        self.valid.load(Ordering::Acquire)
    }

    /// Returns the module code stored is this [ImmutableModuleCode].
    fn inner(&self) -> &Arc<ModuleCode<DC, VC, E, Option<TxnIndex>>> {
        self.module.deref()
    }
}

/// An immutable cache for verified code, that can be accessed concurrently thought the block, and
/// only modified at block boundaries.
pub struct ImmutableModuleCache<K, DC, VC, E> {
    /// Module cache containing the verified code.
    module_cache: ExplicitSyncWrapper<HashMap<K, ImmutableModuleCode<DC, VC, E>>>,
    /// Maximum cache size. If the size is greater than this limit, the cache is flushed. Note that
    /// this can only be done at block boundaries.
    capacity: usize,

    /// Represents the generation of this cache. Incremented for every block.
    generation: ExplicitSyncWrapper<u32>,
}

impl<K, DC, VC, E> ImmutableModuleCache<K, DC, VC, E>
where
    K: Hash + Eq + Clone,
    VC: Deref<Target = Arc<DC>>,
{
    /// Returns new empty module cache with default capacity.
    pub fn empty() -> Self {
        let default_capacity = 100_000;
        Self::with_capacity(default_capacity)
    }

    /// Returns new empty module cache with specified capacity.
    fn with_capacity(capacity: usize) -> Self {
        Self {
            module_cache: ExplicitSyncWrapper::new(HashMap::new()),
            capacity,
            generation: ExplicitSyncWrapper::new(0),
        }
    }

    /// Returns true if the key exists in immutable cache and the corresponding module is valid.
    pub(crate) fn contains_valid(&self, key: &K) -> bool {
        self.module_cache
            .acquire()
            .get(key)
            .is_some_and(|module| module.is_valid())
    }

    /// Marks the cached module (if it exists) as invalid. As a result, all subsequent calls to the
    /// cache for the associated key  will result in a cache miss. Note that it is fine for an
    /// entry not to exist, in which case this is a no-op.
    pub(crate) fn mark_invalid(&self, key: &K) {
        if let Some(module) = self.module_cache.acquire().get(key) {
            module.mark_invalid();
        }
    }

    /// Sets the generation of the module stored at associated key to the generation of the cache.
    pub(crate) fn set_generation(&self, key: &K) {
        if let Some(module) = self.module_cache.acquire().get(key) {
            module.set_generation(*self.generation.acquire());
        }
    }

    /// Returns the module stored in cache. If the module has not been cached, or it exists but is
    /// not valid, [None] is returned. Also returns a boolean flag to indicate if the cached module
    /// needs validation (i.e., its generation is not equal to the generation of the cache).
    pub(crate) fn get(
        &self,
        key: &K,
    ) -> Option<(Arc<ModuleCode<DC, VC, E, Option<TxnIndex>>>, bool)> {
        self.module_cache.acquire().get(key).and_then(|module| {
            if module.is_valid() {
                let needs_validation = *self.generation.acquire() != module.generation();
                Some((module.inner().clone(), needs_validation))
            } else {
                None
            }
        })
    }

    /// Flushes the cache. Should never be called throughout block-execution. Use with caution.
    pub fn flush_unchecked(&self) {
        self.module_cache.acquire().clear();
    }

    /// Inserts modules into the cache, and increments the generation counter of the cache. Should
    /// never be called throughout block-execution. Use with caution.
    ///
    /// Notes:
    ///   1. Only verified modules are inserted.
    ///   2. Versions of inserted modules are set to [None] (storage version).
    ///   3. Valid modules should not be removed, and new modules should have unique ownership. If
    ///      these constraints are violated, a panic error is returned.
    ///   4. If the cache size exceeds its capacity after all verified modules have been inserted,
    ///      the cache is flushed.
    pub(crate) fn insert_verified_and_increment_generation_unchecked(
        &self,
        modules: impl Iterator<Item = (K, Arc<ModuleCode<DC, VC, E, Option<TxnIndex>>>)>,
    ) -> Result<(), PanicError> {
        use hashbrown::hash_map::Entry::*;

        let current_generation = *self.generation.acquire();
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
                    entry.remove();
                }
            }

            if module.code().is_verified() {
                let mut module = module.as_ref().clone();
                module.set_version(None);

                let code = ImmutableModuleCode::new(Arc::new(module), current_generation)?;
                let prev = module_cache.insert(key.clone(), code);

                // At this point, we must have removed the entry, or returned a panic error.
                assert!(prev.is_none())
            }
        }

        // In case capacity is exceeded, flush the cache.
        if module_cache.len() > self.capacity {
            module_cache.clear();
        }

        // Increment generation counter to ensure we can later check that module cache is in sync
        // with the state.
        *self.generation.acquire() = current_generation.wrapping_add(1);

        Ok(())
    }

    /// Insert the module to cache. Used for tests only.
    #[cfg(any(test, feature = "testing"))]
    pub fn insert(&self, key: K, module: Arc<ModuleCode<DC, VC, E, Option<TxnIndex>>>) {
        self.module_cache
            .acquire()
            .insert(key, ImmutableModuleCode::new(module, 0).unwrap());
    }

    /// Removes the module from cache. Used for tests only.
    #[cfg(any(test, feature = "testing"))]
    pub fn remove(&self, key: &K) {
        self.module_cache.acquire().remove(key);
    }

    /// Returns the size of the cache. Used for tests only.
    #[cfg(any(test, feature = "testing"))]
    pub fn size(&self) -> usize {
        self.module_cache.acquire().len()
    }

    /// Returns the generation counter for the cache. Used for tests only.
    #[cfg(any(test, feature = "testing"))]
    pub fn generation(&self) -> u32 {
        *self.generation.acquire()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use claims::{assert_err, assert_ok, assert_some};
    use move_vm_types::code::{mock_deserialized_code, mock_verified_code};

    #[test]
    fn test_immutable_module_code() {
        assert!(ImmutableModuleCode::new(mock_deserialized_code(0, None), 0).is_err());
        assert!(ImmutableModuleCode::new(mock_deserialized_code(0, Some(22)), 0).is_err());
        assert!(ImmutableModuleCode::new(mock_verified_code(0, Some(22)), 0).is_err());
        assert!(ImmutableModuleCode::new(mock_verified_code(0, None), 0).is_ok());
    }

    #[test]
    fn test_immutable_module_code_validity() {
        let module_code = assert_ok!(ImmutableModuleCode::new(mock_verified_code(0, None), 0));
        assert!(module_code.is_valid());

        module_code.mark_invalid();
        assert!(!module_code.is_valid());
    }

    #[test]
    fn test_immutable_module_code_generation() {
        let module_code = assert_ok!(ImmutableModuleCode::new(mock_verified_code(0, None), 7));
        assert_eq!(module_code.generation(), 7);

        module_code.set_generation(10);
        assert_eq!(module_code.generation(), 10);
    }

    #[test]
    fn test_global_module_cache() {
        let global_cache = ImmutableModuleCache::empty();

        global_cache.insert(0, mock_verified_code(0, None));
        global_cache.insert(1, mock_verified_code(1, None));
        global_cache.mark_invalid(&1);

        assert_eq!(global_cache.size(), 2);

        assert!(global_cache.contains_valid(&0));
        assert!(!global_cache.contains_valid(&1));
        assert!(!global_cache.contains_valid(&3));

        assert!(global_cache.get(&0).is_some());
        assert!(global_cache.get(&1).is_none());
        assert!(global_cache.get(&3).is_none());
    }

    #[test]
    fn test_insert_verified_for_global_module_cache() {
        let capacity = 10;
        let global_cache = ImmutableModuleCache::with_capacity(capacity);
        assert_eq!(global_cache.generation(), 0);

        let mut new_modules = vec![];
        for i in 0..capacity {
            new_modules.push((i, mock_verified_code(i, Some(i as TxnIndex))));
        }
        let result = global_cache
            .insert_verified_and_increment_generation_unchecked(new_modules.into_iter());
        assert!(result.is_ok());
        assert_eq!(global_cache.size(), capacity);
        assert_eq!(global_cache.generation(), 1);

        // Versions should be set to storage. The returned code needs validation because generation
        // has been incremented.
        for key in 0..capacity {
            let (code, needs_validation) = assert_some!(global_cache.get(&key));
            assert!(code.version().is_none());
            assert!(needs_validation);
        }

        // Too many modules added, the cache should be flushed.
        let new_modules = vec![(11, mock_verified_code(11, None))];
        let result = global_cache
            .insert_verified_and_increment_generation_unchecked(new_modules.into_iter());
        assert!(result.is_ok());
        assert_eq!(global_cache.size(), 0);
        assert_eq!(global_cache.generation(), 2);

        // Should not add deserialized code.
        let deserialized_modules = vec![(0, mock_deserialized_code(0, None))];
        assert_ok!(global_cache
            .insert_verified_and_increment_generation_unchecked(deserialized_modules.into_iter()));
        assert_eq!(global_cache.size(), 0);
        assert_eq!(global_cache.generation(), 3);

        // Should not override valid modules.
        global_cache.insert(0, mock_verified_code(0, None));
        let new_modules = vec![(0, mock_verified_code(100, None))];
        assert_err!(global_cache
            .insert_verified_and_increment_generation_unchecked(new_modules.into_iter()));
        assert_eq!(global_cache.generation(), 3);

        // Can override invalid modules.
        global_cache.mark_invalid(&0);
        let new_modules = vec![(0, mock_verified_code(100, None))];
        let result = global_cache
            .insert_verified_and_increment_generation_unchecked(new_modules.into_iter());
        assert!(result.is_ok());
        assert_eq!(global_cache.size(), 1);
        assert_eq!(global_cache.generation(), 4);

        // Generation incremented even if there are no code publishes.
        let result =
            global_cache.insert_verified_and_increment_generation_unchecked(vec![].into_iter());
        assert!(result.is_ok());
        assert_eq!(global_cache.generation(), 5);
    }
}
