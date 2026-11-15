// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::counters::GLOBAL_LAYOUT_CACHE_MISSES;
use aptos_drop_helper::DEFAULT_DROPPER;
use aptos_mvhashmap::types::TxnIndex;
use aptos_types::{
    error::PanicError, state_store::state_value::StateValue, vm::modules::AptosModuleExtension,
};
use dashmap::DashMap;
use hashbrown::{Equivalent, HashMap};
use move_binary_format::{errors::PartialVMResult, CompiledModule};
use move_core_types::language_storage::ModuleId;
use move_vm_runtime::{LayoutCacheEntry, LayoutCacheKey, Module, RuntimeEnvironment};
use move_vm_types::code::{ModuleCache, ModuleCode, WithSize};
use std::{
    hash::Hash,
    ops::Deref,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
};

/// Entry stored in [GlobalModuleCache]. Can be invalidated by module publishing.
struct Entry<Deserialized, Verified, Extension> {
    /// False if this code is "valid" within the block execution context (i.e., there has been no
    /// republishing of this module so far). If true, executor needs to read the module from the
    /// per-block module caches.
    overridden: AtomicBool,
    /// Cached verified module. Must always be verified.
    module: Arc<ModuleCode<Deserialized, Verified, Extension>>,
}

impl<Deserialized, Verified, Extension> Entry<Deserialized, Verified, Extension>
where
    Verified: Deref<Target = Arc<Deserialized>>,
    Extension: WithSize,
{
    /// Returns a new valid module. Returns a (panic) error if the module is not verified.
    fn new(module: Arc<ModuleCode<Deserialized, Verified, Extension>>) -> Result<Self, PanicError> {
        if !module.code().is_verified() {
            return Err(PanicError::CodeInvariantError(
                "Module code is not verified".to_string(),
            ));
        }

        Ok(Self {
            overridden: AtomicBool::new(false),
            module,
        })
    }

    /// Marks the module as overridden.
    fn mark_overridden(&self) {
        self.overridden.store(true, Ordering::Release)
    }

    /// Returns true if the module is not overridden.
    fn is_not_overridden(&self) -> bool {
        !self.overridden.load(Ordering::Acquire)
    }

    /// Returns the module code stored is this [Entry].
    fn module_code(&self) -> &Arc<ModuleCode<Deserialized, Verified, Extension>> {
        &self.module
    }
}

#[cfg(fuzzing)]
impl<Deserialized, Verified, Extension> Entry<Deserialized, Verified, Extension>
where
    Verified: Deref<Target = Arc<Deserialized>>,
    Extension: WithSize,
{
    pub fn clone_for_fuzzing(&self) -> Self {
        let overridden = self.overridden.load(Ordering::Relaxed);
        Self {
            overridden: AtomicBool::new(overridden),
            module: Arc::clone(&self.module),
        }
    }
}

/// The default maximum for the sum of estimated sizes (in bytes) of all cached layouts. Inserting a
/// layout that would exceed it flushes the layout cache.
pub const DEFAULT_MAX_LAYOUT_CACHE_SIZE_IN_BYTES: usize = 1024 * 1024 * 1024;

/// A global cache for verified code and derived information (such as layouts) that is concurrently
/// accessed during the block execution. Module cache is read-only, and modified safely only at
/// block boundaries. Layout cache can be modified during execution of the block.
pub struct GlobalModuleCache<K, D, V, E> {
    /// Module cache containing the verified code.
    module_cache: HashMap<K, Entry<D, V, E>>,
    /// Sum of serialized sizes (in bytes) of all cached modules.
    size: usize,
    /// Cached layouts of structs or enums. This cache stores roots only and is invalidated when
    /// modules are published.
    struct_layouts: DashMap<LayoutCacheKey, LayoutCacheEntry>,
    /// Sum of estimated sizes (in bytes) of all cached layouts.
    layout_cache_size_estimate_in_bytes: AtomicUsize,
    /// Maximum value of the sum of estimated sizes (in bytes) of all cached layouts. If inserting
    /// a new layout exceeds it, the layout cache is flushed first.
    max_layout_cache_size_in_bytes: usize,
    /// True while a thread is flushing the layout cache, so that only one thread flushes at a time.
    layout_cache_flush_in_progress: AtomicBool,
}

impl<K, D, V, E> GlobalModuleCache<K, D, V, E>
where
    K: Hash + Eq + Clone,
    V: Deref<Target = Arc<D>>,
    E: WithSize,
{
    /// Returns new empty module cache.
    pub fn empty() -> Self {
        Self {
            module_cache: HashMap::new(),
            size: 0,
            struct_layouts: DashMap::new(),
            layout_cache_size_estimate_in_bytes: AtomicUsize::new(0),
            max_layout_cache_size_in_bytes: DEFAULT_MAX_LAYOUT_CACHE_SIZE_IN_BYTES,
            layout_cache_flush_in_progress: AtomicBool::new(false),
        }
    }

    /// Sets the maximum value of the sum of estimated sizes (in bytes) of all cached layouts.
    pub fn set_max_layout_cache_size_in_bytes(&mut self, max_layout_cache_size_in_bytes: usize) {
        self.max_layout_cache_size_in_bytes = max_layout_cache_size_in_bytes;
    }

    /// Returns true if the key exists in cache and the corresponding module is not overridden.
    pub fn contains_not_overridden(&self, key: &K) -> bool {
        self.module_cache
            .get(key)
            .is_some_and(|entry| entry.is_not_overridden())
    }

    /// Marks the cached module (if it exists) as overridden. As a result, all subsequent calls to
    /// the cache for the associated key will result in a cache miss. If an entry does not exist,
    /// it is a no-op.
    pub fn mark_overridden(&self, key: &K) {
        if let Some(entry) = self.module_cache.get(key) {
            entry.mark_overridden();
        }
    }

    /// Returns the module stored in cache. If the module has not been cached, or it exists but is
    /// overridden, [None] is returned.
    pub fn get<Q>(&self, key: &Q) -> Option<Arc<ModuleCode<D, V, E>>>
    where
        Q: Hash + Equivalent<K>,
    {
        self.module_cache.get(key).and_then(|entry| {
            entry
                .is_not_overridden()
                .then(|| Arc::clone(entry.module_code()))
        })
    }

    /// Returns the number of entries in the cache.
    pub fn num_modules(&self) -> usize {
        self.module_cache.len()
    }

    /// Returns the number of layout entries in the cache.
    pub fn num_cached_layouts(&self) -> usize {
        self.struct_layouts.len()
    }

    /// Returns the sum of estimated sizes (in bytes) of all cached layouts.
    pub fn layout_cache_size_estimate_in_bytes(&self) -> usize {
        self.layout_cache_size_estimate_in_bytes
            .load(Ordering::Relaxed)
    }

    /// Returns the sum of serialized sizes of modules stored in cache.
    pub fn size_in_bytes(&self) -> usize {
        self.size
    }

    /// Flushes all caches.
    pub fn flush_all_caches(&mut self) {
        self.module_cache.clear();
        self.size = 0;
        // Replace the map instead of clearing it to release the allocated table capacity.
        self.struct_layouts = DashMap::new();
        self.layout_cache_size_estimate_in_bytes
            .store(0, Ordering::Relaxed);
    }

    /// Flushes only layout caches. Used for memory management only (flush when cache size gets
    /// too large).
    ///
    /// Safe to call at any point during block execution: readers clone entries and do not hold
    /// any references into the cache, and cache hits are equivalent to cache misses (in terms of
    /// gas charging and read sets), so flushing cannot change the result of executing a
    /// transaction. Must never be called while holding a reference or entry guard of the cache,
    /// as that would deadlock.
    ///
    /// Only one thread flushes at a time: if a flush is already in progress, returns false
    /// without flushing. Returns true otherwise.
    pub fn flush_layout_cache(&self) -> bool {
        if self
            .layout_cache_flush_in_progress
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            return false;
        }

        // Remove entries one by one, so that the size estimate stays exact even if there are
        // concurrent insertions. Removed entries are kept alive and dropped asynchronously, so
        // that freeing their memory happens off the execution's critical path and outside of the
        // shard locks.
        let mut removed = Vec::with_capacity(self.struct_layouts.len());
        self.struct_layouts.retain(|_, entry| {
            self.layout_cache_size_estimate_in_bytes
                .fetch_sub(entry.size_estimate_in_bytes(), Ordering::Relaxed);
            removed.push(entry.clone());
            false
        });
        self.struct_layouts.shrink_to_fit();
        self.layout_cache_flush_in_progress
            .store(false, Ordering::Release);

        DEFAULT_DROPPER.schedule_drop(removed);
        true
    }

    /// Returns layout entry if it exists in global cache.
    pub(crate) fn get_struct_layout_entry(&self, key: &LayoutCacheKey) -> Option<LayoutCacheEntry> {
        match self.struct_layouts.get(key) {
            None => {
                GLOBAL_LAYOUT_CACHE_MISSES.inc();
                None
            },
            Some(e) => Some(e.deref().clone()),
        }
    }

    pub(crate) fn store_struct_layout_entry(
        &self,
        key: &LayoutCacheKey,
        entry: LayoutCacheEntry,
    ) -> PartialVMResult<()> {
        let size_estimate_in_bytes = entry.size_estimate_in_bytes();

        // Flush before inserting (and before taking any entry guard), so that the size of the
        // cache stays bounded at all times, including in the middle of the block execution.
        let current_size_estimate_in_bytes = self.layout_cache_size_estimate_in_bytes();
        if current_size_estimate_in_bytes.saturating_add(size_estimate_in_bytes)
            > self.max_layout_cache_size_in_bytes
            && !self.flush_layout_cache()
        {
            // Another thread is flushing the cache: do not cache this layout, so that the cache
            // does not grow while the flush is in progress.
            return Ok(());
        }

        if let dashmap::Entry::Vacant(e) = self.struct_layouts.entry(*key) {
            // Only account for the size when the entry is actually inserted, as there can be
            // concurrent insertions of the same key. The size is added while holding the entry
            // guard and before the insertion, so that a concurrent flush (which needs the same
            // shard lock to remove this entry) always subtracts it after it has been added.
            self.layout_cache_size_estimate_in_bytes
                .fetch_add(size_estimate_in_bytes, Ordering::Relaxed);
            e.insert(entry);
        }
        Ok(())
    }

    pub(crate) fn remove_struct_layout_entry(&self, key: &LayoutCacheKey) {
        if let Some((_, entry)) = self.struct_layouts.remove(key) {
            self.layout_cache_size_estimate_in_bytes
                .fetch_sub(entry.size_estimate_in_bytes(), Ordering::Relaxed);
        }
    }

    /// Inserts modules into the cache.
    /// Notes:
    ///   1. Only verified modules are inserted.
    ///   2. Not overridden modules should not be removed, and new modules should have unique
    ///      ownership. If these constraints are violated, a panic error is returned.
    pub fn insert_verified(
        &mut self,
        modules: impl Iterator<Item = (K, Arc<ModuleCode<D, V, E>>)>,
    ) -> Result<(), PanicError> {
        use hashbrown::hash_map::Entry::*;

        for (key, module) in modules {
            if let Occupied(entry) = self.module_cache.entry(key.clone()) {
                if entry.get().is_not_overridden() {
                    return Err(PanicError::CodeInvariantError(
                        "Should never replace a non-overridden module".to_string(),
                    ));
                } else {
                    self.size -= entry.get().module_code().extension().size_in_bytes();
                    entry.remove();
                }
            }

            if module.code().is_verified() {
                self.size += module.extension().size_in_bytes();
                let entry =
                    Entry::new(module).expect("Module has been checked and must be verified");
                let prev = self.module_cache.insert(key.clone(), entry);

                // At this point, we must have removed the entry, or returned a panic error.
                assert!(prev.is_none())
            }
        }
        Ok(())
    }

    /// Insert the module to cache. Used for tests only.
    #[cfg(any(test, feature = "testing"))]
    pub fn insert(&mut self, key: K, module: Arc<ModuleCode<D, V, E>>) {
        self.size += module.extension().size_in_bytes();
        self.module_cache.insert(
            key,
            Entry::new(module).expect("Module code should be verified"),
        );
    }

    /// Removes the module from cache and returns true. If the module does not exist for the
    /// associated key, returns false. Used for tests only.
    #[cfg(any(test, feature = "testing"))]
    pub fn remove(&mut self, key: &K) -> bool {
        if let Some(entry) = self.module_cache.remove(key) {
            self.size -= entry.module_code().extension().size_in_bytes();
            true
        } else {
            false
        }
    }
}

#[cfg(fuzzing)]
impl<K, D, V, E> GlobalModuleCache<K, D, V, E>
where
    K: Hash + Eq + Clone,
    V: Deref<Target = Arc<D>>,
    E: WithSize,
{
    pub fn clone_for_fuzzing(&self) -> Self {
        let mut module_cache: HashMap<K, Entry<D, V, E>> = HashMap::new();
        for (k, v) in self.module_cache.iter() {
            module_cache.insert(k.clone(), v.clone_for_fuzzing());
        }
        Self {
            module_cache,
            size: self.size,
            struct_layouts: self.struct_layouts.clone(),
            layout_cache_size_estimate_in_bytes: AtomicUsize::new(
                self.layout_cache_size_estimate_in_bytes(),
            ),
            max_layout_cache_size_in_bytes: self.max_layout_cache_size_in_bytes,
            layout_cache_flush_in_progress: AtomicBool::new(false),
        }
    }
}

/// Converts a published module's state value into the cached module representation, and adds it
/// to the module cache.
pub(crate) fn add_module_write_to_module_cache(
    module_id: &ModuleId,
    state_value: StateValue,
    txn_idx: TxnIndex,
    runtime_environment: &RuntimeEnvironment,
    global_module_cache: &GlobalModuleCache<ModuleId, CompiledModule, Module, AptosModuleExtension>,
    per_block_module_cache: &impl ModuleCache<
        Key = ModuleId,
        Deserialized = CompiledModule,
        Verified = Module,
        Extension = AptosModuleExtension,
        Version = Option<TxnIndex>,
    >,
) -> Result<(), PanicError> {
    // Since we have successfully serialized the module when converting into this transaction
    // write, the deserialization should never fail.
    let compiled_module = runtime_environment
        .deserialize_into_compiled_module(state_value.bytes())
        .map_err(|err| {
            let msg = format!("Failed to construct the module from state value: {:?}", err);
            PanicError::CodeInvariantError(msg)
        })?;
    let extension = Arc::new(AptosModuleExtension::new(state_value));

    per_block_module_cache
        .insert_deserialized_module(module_id.clone(), compiled_module, extension, Some(txn_idx))
        .map_err(|err| {
            let msg = format!(
                "Failed to insert code for module {} at version {} to module cache: {:?}",
                module_id, txn_idx, err
            );
            PanicError::CodeInvariantError(msg)
        })?;
    global_module_cache.mark_overridden(module_id);
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use claims::{assert_err, assert_ok};
    use move_vm_types::{
        code::{
            mock_deserialized_code, mock_verified_code, MockDeserializedCode, MockExtension,
            MockVerifiedCode,
        },
        loaded_data::runtime_types::Type,
        ty_interner::InternedTypePool,
    };
    use once_cell::sync::Lazy;

    #[test]
    fn test_entry_new() {
        assert!(Entry::new(mock_deserialized_code(0, MockExtension::new(8))).is_err());
        assert!(Entry::new(mock_verified_code(0, MockExtension::new(8))).is_ok());
    }

    #[test]
    fn test_entry_mark_overridden() {
        let entry = assert_ok!(Entry::new(mock_verified_code(0, MockExtension::new(8))));
        assert!(entry.is_not_overridden());

        entry.mark_overridden();
        assert!(!entry.is_not_overridden());
    }

    #[test]
    fn test_cache_is_not_overridden_and_get() {
        let mut cache = GlobalModuleCache::empty();

        // Set the state.
        cache.insert(0, mock_verified_code(0, MockExtension::new(8)));
        cache.insert(1, mock_verified_code(1, MockExtension::new(8)));
        cache.mark_overridden(&1);

        assert_eq!(cache.num_modules(), 2);

        assert!(cache.contains_not_overridden(&0));
        assert!(!cache.contains_not_overridden(&1));
        assert!(!cache.contains_not_overridden(&3));

        assert!(cache.get(&0).is_some());
        assert!(cache.get(&1).is_none());
        assert!(cache.get(&3).is_none());
    }

    #[test]
    fn test_cache_sizes_and_flush() {
        let mut cache = GlobalModuleCache::empty();
        assert_eq!(cache.num_modules(), 0);
        assert_eq!(cache.size_in_bytes(), 0);

        cache.insert(0, mock_verified_code(0, MockExtension::new(8)));
        cache.insert(1, mock_verified_code(1, MockExtension::new(16)));
        cache.insert(2, mock_verified_code(2, MockExtension::new(8)));
        assert_eq!(cache.num_modules(), 3);
        assert_eq!(cache.size_in_bytes(), 32);

        assert!(cache.remove(&2));
        assert_eq!(cache.num_modules(), 2);
        assert_eq!(cache.size_in_bytes(), 24);

        cache.flush_all_caches();
        assert_eq!(cache.num_modules(), 0);
        assert_eq!(cache.size_in_bytes(), 0);
    }

    /// Returns a distinct layout cache key for every `n`: the key of `vector^n<u8>`.
    fn layout_cache_key(n: u32) -> LayoutCacheKey {
        static TY_POOL: Lazy<InternedTypePool> = Lazy::new(InternedTypePool::new);
        let ty = (0..n).fold(Type::U8, |ty, _| Type::vector(ty));
        LayoutCacheKey {
            id: TY_POOL.instantiate_and_intern(&ty, &[]),
        }
    }

    type MockGlobalModuleCache =
        GlobalModuleCache<u32, MockDeserializedCode, MockVerifiedCode, MockExtension>;

    fn mock_cache() -> MockGlobalModuleCache {
        GlobalModuleCache::empty()
    }

    /// Stores a dummy layout with the specified size estimate under the key for `n`.
    fn store_layout(cache: &MockGlobalModuleCache, n: u32, size_estimate_in_bytes: usize) {
        let entry = LayoutCacheEntry::new_for_testing(size_estimate_in_bytes);
        assert_ok!(cache.store_struct_layout_entry(&layout_cache_key(n), entry));
    }

    #[test]
    fn test_layout_cache_size_accounting() {
        let cache = mock_cache();
        assert_eq!(cache.num_cached_layouts(), 0);
        assert_eq!(cache.layout_cache_size_estimate_in_bytes(), 0);

        store_layout(&cache, 0, 10);
        store_layout(&cache, 1, 20);
        assert_eq!(cache.num_cached_layouts(), 2);
        assert_eq!(cache.layout_cache_size_estimate_in_bytes(), 30);

        // Duplicate insertion is a no-op.
        store_layout(&cache, 1, 100);
        assert_eq!(cache.num_cached_layouts(), 2);
        assert_eq!(cache.layout_cache_size_estimate_in_bytes(), 30);

        // Removal subtracts the size, removal of non-existent entries is a no-op.
        cache.remove_struct_layout_entry(&layout_cache_key(0));
        cache.remove_struct_layout_entry(&layout_cache_key(0));
        cache.remove_struct_layout_entry(&layout_cache_key(5));
        assert_eq!(cache.num_cached_layouts(), 1);
        assert_eq!(cache.layout_cache_size_estimate_in_bytes(), 20);

        cache.flush_layout_cache();
        assert_eq!(cache.num_cached_layouts(), 0);
        assert_eq!(cache.layout_cache_size_estimate_in_bytes(), 0);

        let mut cache = cache;
        store_layout(&cache, 0, 10);
        cache.flush_all_caches();
        assert_eq!(cache.num_cached_layouts(), 0);
        assert_eq!(cache.layout_cache_size_estimate_in_bytes(), 0);
    }

    #[test]
    fn test_layout_cache_flushes_when_full() {
        let mut cache = mock_cache();
        cache.set_max_layout_cache_size_in_bytes(100);

        for i in 0..4 {
            store_layout(&cache, i, 25);
        }
        // Exactly at the limit, so no flush.
        assert_eq!(cache.num_cached_layouts(), 4);
        assert_eq!(cache.layout_cache_size_estimate_in_bytes(), 100);

        // Does not fit, so the cache is flushed first and only the new entry remains.
        store_layout(&cache, 4, 1);
        assert_eq!(cache.num_cached_layouts(), 1);
        assert_eq!(cache.layout_cache_size_estimate_in_bytes(), 1);
        assert!(cache
            .get_struct_layout_entry(&layout_cache_key(4))
            .is_some());
        assert!(cache
            .get_struct_layout_entry(&layout_cache_key(0))
            .is_none());
    }

    #[test]
    fn test_layout_cache_flush_in_progress_skips_flush_and_insert() {
        let mut cache = mock_cache();
        cache.set_max_layout_cache_size_in_bytes(100);
        store_layout(&cache, 0, 60);

        // Simulate a flush in progress on another thread.
        cache
            .layout_cache_flush_in_progress
            .store(true, Ordering::Relaxed);
        assert!(!cache.flush_layout_cache());
        assert_eq!(cache.num_cached_layouts(), 1);

        // Does not fit and cannot flush, so the new layout is not cached.
        store_layout(&cache, 1, 60);
        assert_eq!(cache.num_cached_layouts(), 1);
        assert_eq!(cache.layout_cache_size_estimate_in_bytes(), 60);
        assert!(cache
            .get_struct_layout_entry(&layout_cache_key(1))
            .is_none());

        // Fits, so it is cached without flushing.
        store_layout(&cache, 2, 40);
        assert_eq!(cache.num_cached_layouts(), 2);
        assert_eq!(cache.layout_cache_size_estimate_in_bytes(), 100);

        // Once the other flush is done, flushing works again.
        cache
            .layout_cache_flush_in_progress
            .store(false, Ordering::Relaxed);
        assert!(cache.flush_layout_cache());
        assert_eq!(cache.num_cached_layouts(), 0);
        assert_eq!(cache.layout_cache_size_estimate_in_bytes(), 0);
    }

    #[test]
    fn test_layout_cache_concurrent_inserts_and_flushes() {
        let mut cache = mock_cache();
        cache.set_max_layout_cache_size_in_bytes(1_000);

        let num_threads = 8;
        let num_keys = 500;
        std::thread::scope(|s| {
            for t in 0..num_threads {
                let cache = &cache;
                s.spawn(move || {
                    for i in 0..num_keys {
                        store_layout(cache, i, 1 + (i % 7) as usize);
                        if (i + t) % 50 == 0 {
                            cache.flush_layout_cache();
                        }
                        if (i + t) % 13 == 0 {
                            cache.remove_struct_layout_entry(&layout_cache_key(i / 2));
                        }
                        // Every thread can exceed the bound by at most one entry (inserted
                        // concurrently after the check for flushing).
                        let bound = 1_000 + 7 * num_threads as usize;
                        assert!(cache.layout_cache_size_estimate_in_bytes() <= bound);
                    }
                });
            }
        });

        let expected = cache
            .struct_layouts
            .iter()
            .map(|e| e.value().size_estimate_in_bytes())
            .sum::<usize>();
        assert_eq!(cache.layout_cache_size_estimate_in_bytes(), expected);
    }

    #[test]
    fn test_cache_insert_verified() {
        let mut cache = GlobalModuleCache::empty();

        let mut new_modules = vec![];
        for i in 0..10 {
            new_modules.push((i, mock_verified_code(i, MockExtension::new(8))));
        }
        assert_ok!(cache.insert_verified(new_modules.into_iter()));

        assert_eq!(cache.num_modules(), 10);
        assert_eq!(cache.size_in_bytes(), 80);
    }

    #[test]
    fn test_cache_insert_verified_unchecked_does_not_add_deserialized_code() {
        let mut cache = GlobalModuleCache::empty();

        let deserialized_modules = vec![(0, mock_deserialized_code(0, MockExtension::new(8)))];
        assert_ok!(cache.insert_verified(deserialized_modules.into_iter()));

        assert_eq!(cache.num_modules(), 0);
        assert_eq!(cache.size_in_bytes(), 0);
    }

    #[test]
    fn test_cache_insert_verified_does_not_override_valid_modules() {
        let mut cache = GlobalModuleCache::empty();

        cache.insert(0, mock_verified_code(0, MockExtension::new(8)));
        assert_eq!(cache.num_modules(), 1);
        assert_eq!(cache.size_in_bytes(), 8);

        let new_modules = vec![(0, mock_verified_code(100, MockExtension::new(32)))];
        assert_err!(cache.insert_verified(new_modules.into_iter()));
    }

    #[test]
    fn test_cache_insert_verified_inserts_overridden_modules() {
        let mut cache = GlobalModuleCache::empty();

        cache.insert(0, mock_verified_code(0, MockExtension::new(8)));
        cache.mark_overridden(&0);
        assert_eq!(cache.num_modules(), 1);
        assert_eq!(cache.size_in_bytes(), 8);

        let new_modules = vec![(0, mock_verified_code(100, MockExtension::new(32)))];
        assert_ok!(cache.insert_verified(new_modules.into_iter()));

        assert_eq!(cache.num_modules(), 1);
        assert_eq!(cache.size_in_bytes(), 32);
    }
}
