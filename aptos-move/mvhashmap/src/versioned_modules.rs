// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::types::{Flag, MVModulesError, MVModulesOutput, ShiftedTxnIndex, TxnIndex};
use aptos_crypto::hash::HashValue;
use aptos_types::{
    executable::{Executable, ExecutableDescriptor},
    vm::modules::OnChainUnverifiedModule,
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
struct Entry {
    /// Used to mark the entry as a "write estimate".
    flag: Flag,

    /// The contents of the module as produced by the VM (can be WriteOp based on a
    /// blob or CompiledModule, but must satisfy TransactionWrite to be able to
    /// generate the hash below.
    module: OnChainUnverifiedModule,
    /// The hash of the blob, used instead of incarnation for validation purposes,
    /// and also for uniquely identifying associated executables.
    hash: HashValue,
}

/// A VersionedValue internally contains a BTreeMap from indices of transactions
/// that update the given access path alongside the corresponding entries.
struct VersionedValue<X: Executable> {
    versioned_map: BTreeMap<ShiftedTxnIndex, CachePadded<Entry>>,

    /// Executables corresponding to published versions of the module, based on hash.
    executables: HashMap<HashValue, Arc<X>>,
}

/// Maps each key (access path) to an internal VersionedValue.
pub struct VersionedModules<K, X: Executable> {
    values: DashMap<K, VersionedValue<X>>,
}

impl Entry {
    pub fn new_write_from(module: OnChainUnverifiedModule) -> Entry {
        let hash = HashValue::new(module.hash);
        Entry {
            flag: Flag::Done,
            module,
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

impl<X: Executable> VersionedValue<X> {
    pub fn new() -> Self {
        Self {
            versioned_map: BTreeMap::new(),
            executables: HashMap::new(),
        }
    }

    fn read(
        &self,
        txn_idx: TxnIndex,
    ) -> Result<(OnChainUnverifiedModule, HashValue), MVModulesError> {
        match self
            .versioned_map
            .range(ShiftedTxnIndex::zero_idx()..ShiftedTxnIndex::new(txn_idx))
            .next_back()
        {
            Some((idx, entry)) => {
                if entry.flag() == Flag::Estimate {
                    // Found a dependency.
                    return Err(MVModulesError::Dependency(
                        idx.idx().expect("May not depend on storage version"),
                    ));
                }

                Ok((entry.module.clone(), entry.hash))
            },
            None => Err(MVModulesError::NotFound),
        }
    }
}

impl<X: Executable> Default for VersionedValue<X> {
    fn default() -> Self {
        VersionedValue::new()
    }
}

impl<K: Hash + Clone + Eq, X: Executable> VersionedModules<K, X> {
    pub(crate) fn new() -> Self {
        Self {
            values: DashMap::new(),
        }
    }

    pub(crate) fn num_keys(&self) -> usize {
        self.values.len()
    }

    /// Mark an entry from transaction 'txn_idx' at access path 'key' as an estimated write
    /// (for future incarnation). Will panic if the entry is not in the data-structure.
    pub fn mark_estimate(&self, key: &K, txn_idx: TxnIndex) {
        let mut v = self.values.get_mut(key).expect("Path must exist");
        v.versioned_map
            .get_mut(&ShiftedTxnIndex::new(txn_idx))
            .expect("Entry by the txn must exist to mark estimate")
            .mark_estimate();
    }

    /// Versioned write of module at a given key (and version).
    pub fn write(&self, key: K, txn_idx: TxnIndex, data: OnChainUnverifiedModule) {
        let mut v = self.values.entry(key).or_default();
        v.versioned_map.insert(
            ShiftedTxnIndex::new(txn_idx),
            CachePadded::new(Entry::new_write_from(data)),
        );
    }

    pub fn set_base(&self, key: K, data: OnChainUnverifiedModule) {
        let mut v = self.values.entry(key).or_default();
        v.versioned_map.insert(
            ShiftedTxnIndex::zero_idx(),
            CachePadded::new(Entry::new_write_from(data)),
        );
    }

    /// Adds a new executable to the multi-version data-structure. The executable is either
    /// storage-version (and fixed) or uniquely identified by the (cryptographic) hash of the
    /// module published during the block.
    pub fn store_executable(&self, key: &K, descriptor_hash: HashValue, executable: X) {
        let mut v = self.values.get_mut(key).expect("Path must exist");
        v.executables
            .entry(descriptor_hash)
            .or_insert_with(|| Arc::new(executable));
    }

    /// Fetches the latest module stored at the given key, either as in an executable form,
    /// if already cached, or in a raw module format that the VM can convert to an executable.
    /// The errors are returned if no module is found, or if a dependency is encountered.
    pub fn fetch_module(
        &self,
        key: &K,
        txn_idx: TxnIndex,
    ) -> Result<MVModulesOutput<X>, MVModulesError> {
        use MVModulesError::*;
        use MVModulesOutput::*;

        match self.values.get(key) {
            Some(v) => v
                .read(txn_idx)
                .map(|(module, hash)| match v.executables.get(&hash) {
                    Some(x) => Executable((x.clone(), ExecutableDescriptor::Published(hash))),
                    None => Module(module),
                }),
            None => Err(NotFound),
        }
    }

    /// Delete an entry from transaction 'txn_idx' at access path 'key'. Will panic
    /// if the corresponding entry does not exist.
    pub fn remove(&self, key: &K, txn_idx: TxnIndex) {
        // TODO: investigate logical deletion.
        let mut v = self.values.get_mut(key).expect("Path must exist");
        assert!(
            v.versioned_map
                .remove(&ShiftedTxnIndex::new(txn_idx))
                .is_some(),
            "Entry must exist to be deleted"
        );
    }
}
