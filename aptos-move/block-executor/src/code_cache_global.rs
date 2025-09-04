// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_mvhashmap::types::TxnIndex;
use aptos_types::{
    error::PanicError, transaction::BlockExecutableTransaction, vm::modules::AptosModuleExtension,
    write_set::TransactionWrite,
};
use aptos_vm_types::module_write_set::ModuleWrite;
use hashbrown::HashMap;
use move_binary_format::CompiledModule;
use move_core_types::language_storage::ModuleId;
use move_vm_runtime::{Module, RuntimeEnvironment};
use move_vm_types::code::{ModuleCache, ModuleCode, WithSize};
use std::{
    hash::Hash,
    ops::Deref,
    sync::atomic::{AtomicBool, Ordering},
};
use triomphe::Arc;

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

/// A global module cache for verified code that is read-only and concurrently accessed during the
/// block execution. Modified safely only at block boundaries.
pub struct GlobalModuleCache<K, D, V, E> {
    /// Module cache containing the verified code.
    module_cache: HashMap<K, Entry<D, V, E>>,
    /// Sum of serialized sizes (in bytes) of all cached modules.
    size: usize,
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

    /// Returns the sum of serialized sizes of modules stored in cache.
    pub fn size_in_bytes(&self) -> usize {
        self.size
    }

    /// Flushes the module cache.
    pub fn flush(&mut self) {
        self.module_cache.clear();
        self.size = 0;
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
}
