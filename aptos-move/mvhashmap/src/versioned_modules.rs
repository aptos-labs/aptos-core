// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::types::{Flag, MVModulesError, TxnIndex};
use aptos_crypto::hash::{DefaultHasher, HashValue};
use aptos_types::{
    executable::{Executable, ModuleDescriptor},
    write_set::TransactionWrite,
};
use crossbeam::utils::CachePadded;
use dashmap::DashMap;
use std::{
    collections::{btree_map::BTreeMap, HashMap},
    hash::Hash,
    sync::Arc,
};

/// Every entry in shared multi-version data-structure has an "estimate" flag
/// and some content.
struct Entry<V: TransactionWrite> {
    /// Used to mark the entry as a "write estimate".
    flag: Flag,

    /// The contents of the module as produced by the VM (can be WriteOp based on a
    /// blob or CompiledModule, but must satisfy TransactionWrite to be able to
    /// generate the hash below.
    module: Arc<V>,
    /// The hash of the blob, used instead of incarnation for validation purposes,
    /// and also for uniquely identifying associated executables.
    hash: HashValue,
}

/// A VersionedValue internally contains a BTreeMap from indices of transactions
/// that update the given access path alongside the corresponding entries.
struct VersionedValue<V: TransactionWrite, X: Executable> {
    versioned_map: BTreeMap<TxnIndex, CachePadded<Entry<V>>>,
    /// Module contents at storage version.
    maybe_base_module: Option<Arc<V>>,

    /// Executables corresponding to published versions of the module, based on hash.
    executables: HashMap<HashValue, X>,
    /// Executable at storage version.
    maybe_base_executable: Option<X>,
}

/// Maps each key (access path) to an internal VersionedValue.
pub struct VersionedModules<K, V: TransactionWrite, X: Executable> {
    values: DashMap<K, VersionedValue<V, X>>,
}

impl<V: TransactionWrite> Entry<V> {
    fn new_write_from(module: V) -> Entry<V> {
        let hash = module
            .extract_raw_bytes()
            .map(|bytes| {
                let mut hasher = DefaultHasher::new(b"Module");
                hasher.update(&bytes);
                hasher.finish()
            })
            .expect("Module can't be deleted");

        Entry {
            flag: Flag::Done,
            module: Arc::new(module),
            hash,
        }
    }

    pub fn flag(&self) -> Flag {
        self.flag
    }

    pub fn mark_estimate(&mut self) {
        self.flag = Flag::Estimate;
    }
}

impl<V: TransactionWrite, X: Executable> VersionedValue<V, X> {
    pub fn new() -> Self {
        Self {
            versioned_map: BTreeMap::new(),
            maybe_base_module: None,
            executables: HashMap::new(),
            maybe_base_executable: None,
        }
    }

    fn read(&self, txn_idx: TxnIndex) -> Result<(Arc<V>, ModuleDescriptor), MVModulesError> {
        match self.versioned_map.range(0..txn_idx).next_back() {
            Some((idx, entry)) => {
                if entry.flag() == Flag::Estimate {
                    // Found a dependency.
                    return Err(MVModulesError::Dependency(*idx));
                }

                Ok((
                    entry.module.clone(),
                    ModuleDescriptor::Published(entry.hash),
                ))
            },
            None => self
                .maybe_base_module
                .as_ref()
                .map(|v| (v.clone(), ModuleDescriptor::Storage))
                .ok_or_else(|| MVModulesError::NotFound),
        }
    }
}

impl<V: TransactionWrite, X: Executable> Default for VersionedValue<V, X> {
    fn default() -> Self {
        VersionedValue::new()
    }
}

impl<K: Hash + Clone + Eq, V: TransactionWrite, X: Executable> VersionedModules<K, V, X> {
    pub(crate) fn new() -> Self {
        Self {
            values: DashMap::new(),
        }
    }

    /// Mark an entry from transaction 'txn_idx' at access path 'key' as an estimated write
    /// (for future incarnation). Will panic if the entry is not in the data-structure.
    pub fn mark_estimate(&self, key: &K, txn_idx: TxnIndex) {
        let mut v = self.values.get_mut(key).expect("Path must exist");
        v.versioned_map
            .get_mut(&txn_idx)
            .expect("Entry by the txn must exist to mark estimate")
            .mark_estimate();
    }

    /// Versioned write of module at a given key (and version).
    pub fn write(&self, key: K, txn_idx: TxnIndex, data: V) {
        let mut v = self.values.entry(key).or_default();
        v.versioned_map
            .insert(txn_idx, CachePadded::new(Entry::new_write_from(data)));
    }

    pub fn store_base_module(&self, key: K, data: V) {
        let mut v = self.values.entry(key).or_default();
        v.maybe_base_module = Some(Arc::new(data));
    }

    /// Adds a new executable to the multi-version data-structure. The executable is either
    /// storage-version (and fixed) or uniquely identified by the (cryptographic) hash of the
    /// module published during the block.
    pub fn store_executable(&self, key: &K, descriptor: ModuleDescriptor, executable: X) {
        let mut v = self.values.get_mut(&key).expect("Path must exist");

        use ModuleDescriptor::*;
        match descriptor {
            Published(descriptor_hash) => {
                v.executables
                    .entry(descriptor_hash)
                    .or_insert_with(|| executable);
            },
            Storage => {
                v.maybe_base_executable = Some(executable);
            },
        }
    }

    /// Fetches the latest module stored at the given key, and hash/storage descriptor.
    /// The errors are returned if no module is found, or if a dependency is encountered.
    pub fn fetch_module(
        &self,
        key: &K,
        txn_idx: TxnIndex,
    ) -> Result<(Arc<V>, ModuleDescriptor), MVModulesError> {
        self.values
            .get(key)
            .map_or_else(|| Err(MVModulesError::NotFound), |v| v.read(txn_idx))
    }

    /// Fetches the latest executable stored at the given key, and hash/storage descriptor.
    /// The errors are returned if no module is found, or if a dependency is encountered.
    pub fn fetch_executable(
        &self,
        key: &K,
        txn_idx: TxnIndex,
    ) -> Option<(X, Arc<V>, ModuleDescriptor)> {
        self.values.get(key).and_then(|v| {
            v.read(txn_idx).ok().and_then(|(module, descriptor)| {
                let maybe_executable = match descriptor {
                    ModuleDescriptor::Published(module_hash) => v
                        .executables
                        .get(&module_hash)
                        .map(|executable| executable.clone()),
                    ModuleDescriptor::Storage => v.maybe_base_executable.clone(),
                };

                maybe_executable.map(|executable| (executable, module, descriptor))
            })
        })
    }

    /// Delete an entry from transaction 'txn_idx' at access path 'key'. Will panic
    /// if the corresponding entry does not exist.
    pub fn remove(&self, key: &K, txn_idx: TxnIndex) {
        // TODO: investigate logical deletion.
        let mut v = self.values.get_mut(key).expect("Path must exist");
        assert!(
            v.versioned_map.remove(&txn_idx).is_some(),
            "Entry must exist to be deleted"
        );
    }
}
