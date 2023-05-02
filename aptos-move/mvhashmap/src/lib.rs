// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    types::{MVCodeError, MVCodeOutput, MVDataError, MVDataOutput, TxnIndex, Version},
    versioned_code::VersionedCode,
    versioned_data::VersionedData,
};
use aptos_aggregator::delta_change_set::DeltaOp;
use aptos_types::{
    executable::{Executable, ExecutableDescriptor, ModulePath},
    write_set::TransactionWrite,
};
use std::{fmt::Debug, hash::Hash};

pub mod types;
pub mod versioned_code;
pub mod versioned_data;

#[cfg(test)]
mod unit_tests;

/// Main multi-version data-structure used by threads to read/write during parallel
/// execution.
///
/// Concurrency is managed by DashMap, i.e. when a method accesses a BTreeMap at a
/// given key, it holds exclusive access and doesn't need to explicitly synchronize
/// with other reader/writers.
///
/// TODO: separate V into different generic types for data and modules / code (currently
/// both WriteOp for executor, and use extract_raw_bytes. data for aggregators, and
/// code for computing the module hash.
pub struct MVHashMap<K, V: TransactionWrite, X: Executable> {
    data: VersionedData<K, V>,
    code: VersionedCode<K, V, X>,
}

impl<K: ModulePath + Hash + Clone + Eq + Debug, V: TransactionWrite, X: Executable>
    MVHashMap<K, V, X>
{
    // -----------------------------------
    // Functions shared for data and code.

    // Option<VersionedCode> is passed to allow re-using code cache between blocks.
    pub fn new(code_cache: Option<VersionedCode<K, V, X>>) -> MVHashMap<K, V, X> {
        MVHashMap {
            data: VersionedData::new(),
            code: code_cache.unwrap_or_default(),
        }
    }

    pub fn take(self) -> (VersionedData<K, V>, VersionedCode<K, V, X>) {
        (self.data, self.code)
    }

    /// Mark an entry from transaction 'txn_idx' at access path 'key' as an estimated write
    /// (for future incarnation). Will panic if the entry is not in the data-structure.
    pub fn mark_estimate(&self, key: &K, txn_idx: TxnIndex) {
        match key.module_path() {
            Some(_) => self.code.mark_estimate(key, txn_idx),
            None => self.data.mark_estimate(key, txn_idx),
        }
    }

    /// Delete an entry from transaction 'txn_idx' at access path 'key'. Will panic
    /// if the corresponding entry does not exist.
    pub fn delete(&self, key: &K, txn_idx: TxnIndex) {
        // This internally deserializes the path, TODO: fix.
        match key.module_path() {
            Some(_) => self.code.delete(key, txn_idx),
            None => self.data.delete(key, txn_idx),
        };
    }

    /// Add a versioned write at a specified key, in code or data map according to the key.
    pub fn write(&self, key: &K, version: Version, value: V) {
        match key.module_path() {
            Some(_) => self.code.write(key, version.0, value),
            None => self.data.write(key, version, value),
        }
    }

    // -----------------------------------------------
    // Functions specific to the multi-versioned data.

    /// Add a delta at a specified key.
    pub fn add_delta(&self, key: &K, txn_idx: TxnIndex, delta: DeltaOp) {
        debug_assert!(
            key.module_path().is_none(),
            "Delta must be stored at a path corresponding to data"
        );

        self.data.add_delta(key, txn_idx, delta);
    }

    pub fn materialize_delta(&self, key: &K, txn_idx: TxnIndex) -> Result<u128, DeltaOp> {
        debug_assert!(
            key.module_path().is_none(),
            "Delta must be stored at a path corresponding to data"
        );

        self.data.materialize_delta(key, txn_idx)
    }

    pub fn set_aggregator_base_value(&self, key: &K, value: u128) {
        debug_assert!(
            key.module_path().is_none(),
            "Delta must be stored at a path corresponding to data"
        );

        self.data.set_aggregator_base_value(key, value);
    }

    /// Read data at access path 'key', from the perspective of transaction 'txn_idx'.
    pub fn fetch_data(
        &self,
        key: &K,
        txn_idx: TxnIndex,
    ) -> anyhow::Result<MVDataOutput<V>, MVDataError> {
        self.data.fetch_data(key, txn_idx)
    }

    // ----------------------------------------------
    // Functions specific to the multi-versioned code.

    /// Adds a new executable to the multiversion data-structure. The executable is either
    /// storage-version (and fixed) or uniquely identified by the (cryptographic) hash of the
    /// module published during the block.
    pub fn store_executable(&self, key: &K, descriptor: ExecutableDescriptor, executable: X) {
        self.code.store_executable(key, descriptor, executable);
    }

    pub fn fetch_code(
        &self,
        key: &K,
        txn_idx: TxnIndex,
    ) -> anyhow::Result<MVCodeOutput<V, X>, MVCodeError> {
        self.code.fetch_code(key, txn_idx)
    }
}

impl<K: ModulePath + Hash + Clone + Debug + Eq, V: TransactionWrite, X: Executable> Default
    for MVHashMap<K, V, X>
{
    fn default() -> Self {
        Self::new(None)
    }
}
