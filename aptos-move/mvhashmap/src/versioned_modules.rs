// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::types::{Flag, MVModulesError, MVModulesOutput, ShiftedTxnIndex, TxnIndex};
use aptos_types::vm::modules::OnChainUnverifiedModule;
use crossbeam::utils::CachePadded;
use dashmap::DashMap;
use std::{collections::btree_map::BTreeMap, hash::Hash};

/// Every entry in shared multi-version data-structure has an "estimate" flag
/// and some content.
struct Entry {
    /// Used to mark the entry as a "write estimate".
    flag: Flag,

    /// The contents of the module as produced by the VM.
    // TODO: Use a generic here when we have a more stable representation.
    module: OnChainUnverifiedModule,
}

/// A VersionedValue internally contains a BTreeMap from indices of transactions
/// that update the given access path alongside the corresponding entries.
struct VersionedValue {
    versioned_map: BTreeMap<ShiftedTxnIndex, CachePadded<Entry>>,
}

/// Maps each key (access path) to an internal VersionedValue.
pub struct VersionedModules<K> {
    values: DashMap<K, VersionedValue>,
}

impl Entry {
    pub fn new_write_from(module: OnChainUnverifiedModule) -> Entry {
        Entry {
            flag: Flag::Done,
            module,
        }
    }

    pub fn flag(&self) -> Flag {
        self.flag
    }

    pub fn mark_estimate(&mut self) {
        self.flag = Flag::Estimate;
    }
}

impl VersionedValue {
    pub fn new() -> Self {
        Self {
            versioned_map: BTreeMap::new(),
        }
    }

    fn read(&self, txn_idx: TxnIndex) -> Result<OnChainUnverifiedModule, MVModulesError> {
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

                Ok(entry.module.clone())
            },
            None => Err(MVModulesError::NotFound),
        }
    }
}

impl Default for VersionedValue {
    fn default() -> Self {
        VersionedValue::new()
    }
}

impl<K: Hash + Clone + Eq> VersionedModules<K> {
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

    /// Fetches the latest module stored at the given key, either as in an executable form,
    /// if already cached, or in a raw module format that the VM can convert to an executable.
    /// The errors are returned if no module is found, or if a dependency is encountered.
    pub fn fetch_module(
        &self,
        key: &K,
        txn_idx: TxnIndex,
    ) -> Result<MVModulesOutput, MVModulesError> {
        use MVModulesError::*;
        use MVModulesOutput::*;

        match self.values.get(key) {
            Some(v) => v.read(txn_idx).map(Module),
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
