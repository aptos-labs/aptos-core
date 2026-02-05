// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::counters::GLOBAL_LAYOUT_CACHE_MISSES;
use aptos_mvhashmap::types::TxnIndex;
use aptos_types::{
    error::PanicError, transaction::BlockExecutableTransaction, vm::modules::AptosModuleExtension,
    write_set::TransactionWrite,
};
use aptos_vm_types::module_write_set::ModuleWrite;
use dashmap::DashMap;
use hashbrown::{HashMap, HashSet};
use move_binary_format::{errors::PartialVMResult, CompiledModule};
use move_core_types::language_storage::ModuleId;
use move_vm_runtime::{LayoutCacheEntry, Module, RuntimeEnvironment, StructKey};
use move_vm_types::code::{ModuleCache, ModuleCode, WithSize};
use std::{
    hash::Hash,
    ops::Deref,
    sync::{
        atomic::{AtomicBool, Ordering},
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
    struct_layouts: DashMap<StructKey, LayoutCacheEntry>,
    /// Maps module IDs to the set of layout keys whose layouts depend on that module (i.e., depend
    /// on struct or enum definitions from this module).
    /// Used for selective cache invalidation on module upgrades.
    module_to_layouts: DashMap<ModuleId, HashSet<StructKey>>,
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
            module_to_layouts: DashMap::new(),
        }
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
    pub fn get(&self, key: &K) -> Option<Arc<ModuleCode<D, V, E>>> {
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

    /// Returns the sum of serialized sizes of modules stored in cache.
    pub fn size_in_bytes(&self) -> usize {
        self.size
    }

    /// Flushes all caches.
    pub fn flush(&mut self) {
        self.module_cache.clear();
        self.size = 0;
        self.struct_layouts.clear();
        self.module_to_layouts.clear();
    }

    /// Flushes only layout caches.
    pub fn flush_layout_cache(&self) {
        // TODO(layouts):
        //   Flushing is only needed because of enums. Once we refactor layouts to store a single
        //   variant instead, this can be removed.
        self.struct_layouts.clear();
        self.module_to_layouts.clear();
    }

    /// Flushes only the layouts that depend on the specified module.
    pub fn flush_layouts_for_module(&self, module_id: &ModuleId) {
        if let Some((_, layout_keys)) = self.module_to_layouts.remove(module_id) {
            for key in layout_keys.iter() {
                // Note that removing this key, we can have other revers mappings storing "dead"
                // keys. For example, if we have modules A and B that are both used by layout L,
                // on flush for A, B still points to L's key.
                // This is fine because keys are small, and we do not need to do GC here. It is
                // also safe because layouts are compatible and so new upgraded layout will still
                // depend on the same module.
                self.struct_layouts.remove(key);
            }
        }
    }

    /// Returns layout entry if it exists in global cache.
    pub(crate) fn get_struct_layout_entry(&self, key: &StructKey) -> Option<LayoutCacheEntry> {
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
        key: &StructKey,
        entry: LayoutCacheEntry,
    ) -> PartialVMResult<()> {
        if let dashmap::Entry::Vacant(e) = self.struct_layouts.entry(*key) {
            // Populate reverse index before inserting the layout.
            for module_id in entry.defining_modules().iter() {
                self.module_to_layouts
                    .entry(module_id.clone())
                    .or_insert_with(HashSet::new)
                    .insert(*key);
            }
            e.insert(entry);
        }
        Ok(())
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
            module_to_layouts: self.module_to_layouts.clone(),
        }
    }
}

/// Converts module write into cached module representation, and adds it to the module cache.
pub(crate) fn add_module_write_to_module_cache<T: BlockExecutableTransaction>(
    write: &ModuleWrite<T::Value>,
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
    let state_value = write
        .write_op()
        .as_state_value()
        .ok_or_else(|| PanicError::CodeInvariantError("Modules cannot be deleted".to_string()))?;

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
        .insert_deserialized_module(
            write.module_id().clone(),
            compiled_module,
            extension,
            Some(txn_idx),
        )
        .map_err(|err| {
            let msg = format!(
                "Failed to insert code for module {}::{} at version {} to module cache: {:?}",
                write.module_address(),
                write.module_name(),
                txn_idx,
                err
            );
            PanicError::CodeInvariantError(msg)
        })?;
    global_module_cache.mark_overridden(write.module_id());
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use claims::{assert_err, assert_ok};
    use move_core_types::{account_address::AccountAddress, identifier::Identifier};
    use move_vm_types::code::{mock_deserialized_code, mock_verified_code, MockExtension};

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

        cache.flush();
        assert_eq!(cache.num_modules(), 0);
        assert_eq!(cache.size_in_bytes(), 0);
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

    fn module_id(name: &str) -> ModuleId {
        ModuleId::new(AccountAddress::random(), Identifier::new(name).unwrap())
    }

    #[test]
    fn test_layout_cache() {
        let key1 = StructKey::struct_key_for_testing(1);
        let layout1 = LayoutCacheEntry::new_for_testing([module_id("a")]);

        let key2 = StructKey::struct_key_for_testing(2);
        let layout2 = LayoutCacheEntry::new_for_testing([module_id("a"), module_id("b")]);

        let key3 = StructKey::struct_key_for_testing(3);
        let layout3 = LayoutCacheEntry::new_for_testing([module_id("a"), module_id("b")]);

        let cache = GlobalModuleCache::empty();
        cache.store_struct_layout_entry(&key1, layout1).unwrap();
        cache.store_struct_layout_entry(&key2, layout2).unwrap();
        cache.store_struct_layout_entry(&key3, layout3).unwrap();

        // In total: 3 layouts and 2 modules
        assert_eq!(cache.num_cached_layouts(), 3);
        assert_eq!(cache.module_to_layouts.len(), 2);

        let a_layout_keys = cache
            .module_to_layouts
            .get(&module_id("a"))
            .unwrap()
            .value()
            .clone();
        let b_layout_keys = cache
            .module_to_layouts
            .get(&module_id("b"))
            .unwrap()
            .value()
            .clone();

        assert_eq!(a_layout_keys.len(), 3);
        assert!(a_layout_keys.contains(&key1));
        assert!(a_layout_keys.contains(&key2));
        assert!(a_layout_keys.contains(&key3));

        assert_eq!(b_layout_keys.len(), 2);
        assert!(b_layout_keys.contains(&key2));
        assert!(b_layout_keys.contains(&key3));

        // Now flush for module B. Only 1 layout should be left (for A). Only 1 module entry is
        // left (for A).
        cache.flush_layouts_for_module(&module_id("b"));
        assert_eq!(cache.num_cached_layouts(), 1);
        assert_eq!(cache.module_to_layouts.len(), 1);

        // Keys for A may still be stale!
        let a_layout_keys = cache
            .module_to_layouts
            .get(&module_id("a"))
            .unwrap()
            .value()
            .clone();
        assert_eq!(a_layout_keys.len(), 3);
        assert!(a_layout_keys.contains(&key1));
        assert!(a_layout_keys.contains(&key2));
        assert!(a_layout_keys.contains(&key3));

        // But layouts do not exist.
        assert!(cache.struct_layouts.get(&key1).is_some());
        assert!(cache.struct_layouts.get(&key2).is_none());
        assert!(cache.struct_layouts.get(&key3).is_none());

        cache.flush_layouts_for_module(&module_id("a"));
        assert_eq!(cache.num_cached_layouts(), 0);
        assert_eq!(cache.module_to_layouts.len(), 0);
    }

    #[test]
    fn test_layout_cache_full_flush_clears_reverse_index() {
        let key1 = StructKey::struct_key_for_testing(1);
        let layout1 = LayoutCacheEntry::new_for_testing([module_id("a")]);

        let key2 = StructKey::struct_key_for_testing(2);
        let layout2 = LayoutCacheEntry::new_for_testing([module_id("a"), module_id("b")]);

        let key3 = StructKey::struct_key_for_testing(3);
        let layout3 = LayoutCacheEntry::new_for_testing([module_id("a"), module_id("b")]);

        let cache = GlobalModuleCache::empty();
        cache.store_struct_layout_entry(&key1, layout1).unwrap();
        cache.store_struct_layout_entry(&key2, layout2).unwrap();
        cache.store_struct_layout_entry(&key3, layout3).unwrap();

        // In total: 3 layouts and 2 modules
        assert_eq!(cache.num_cached_layouts(), 3);
        assert_eq!(cache.module_to_layouts.len(), 2);

        // Full flush - both caches are empty.
        cache.flush_layout_cache();
        assert_eq!(cache.num_cached_layouts(), 0);
        assert!(cache.module_to_layouts.is_empty());
    }
}
