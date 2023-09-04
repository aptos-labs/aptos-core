// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    types::{MVDataError, MVDataOutput, MVModulesError, MVModulesOutput, TxnIndex},
    versioned_data::VersionedData,
    versioned_modules::VersionedModules,
};
use aptos_aggregator::delta_change_set::DeltaOp;
use aptos_crypto::hash::HashValue;
use aptos_types::{
    executable::{Executable, ModulePath},
    write_set::TransactionWrite,
};
use std::{fmt::Debug, hash::Hash};

pub mod types;
pub mod unsync_map;
mod utils;
pub mod versioned_data;
pub mod versioned_modules;

#[cfg(test)]
mod unit_tests;

/// Main multi-version data-structure used by threads to read/write during parallel
/// execution.
///
/// Concurrency is managed by DashMap, i.e. when a method accesses a BTreeMap at a
/// given key, it holds exclusive access and doesn't need to explicitly synchronize
/// with other reader/writers.
///
/// TODO: separate V into different generic types for data and code modules with specialized
/// traits (currently both WriteOp for executor).
pub struct MVHashMap<K, V: TransactionWrite, X: Executable> {
    data: VersionedData<K, V>,
    modules: VersionedModules<K, V, X>,
}

impl<K: ModulePath + Hash + Clone + Eq + Debug, V: TransactionWrite, X: Executable>
    MVHashMap<K, V, X>
{
    // -----------------------------------
    // Functions shared for data and modules.

    pub fn new() -> MVHashMap<K, V, X> {
        MVHashMap {
            data: VersionedData::new(),
            modules: VersionedModules::new(),
        }
    }

    pub fn take(self) -> (VersionedData<K, V>, VersionedModules<K, V, X>) {
        (self.data, self.modules)
    }

    pub fn data(&self) -> &VersionedData<K, V> {
        &self.data
    }

    pub fn modules(&self) -> &VersionedModules<K, V, X> {
        &self.modules
    }

    // -----------------------------------------------
    // Functions specific to the multi-versioned data.

    /// Add a delta at a specified key.
    pub fn add_delta(&self, key: K, txn_idx: TxnIndex, delta: DeltaOp) {
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
    // Functions specific to the multi-versioned modules map.

    /// Adds a new executable to the multi-version data-structure. The executable is either
    /// storage-version (and fixed) or uniquely identified by the (cryptographic) hash of the
    /// module published during the block.
    pub fn store_executable(&self, key: &K, descriptor_hash: HashValue, executable: X) {
        self.modules
            .store_executable(key, descriptor_hash, executable);
    }

    /// Fetches the latest module stored at the given key, either as in an executable form,
    /// if already cached, or in a raw module format that the VM can convert to an executable.
    /// The errors are returned if no module is found, or if a dependency is encountered.
    pub fn fetch_module(
        &self,
        key: &K,
        txn_idx: TxnIndex,
    ) -> anyhow::Result<MVModulesOutput<V, X>, MVModulesError> {
        self.modules.fetch_module(key, txn_idx)
    }
}

impl<K: ModulePath + Hash + Clone + Debug + Eq, V: TransactionWrite, X: Executable> Default
    for MVHashMap<K, V, X>
{
    fn default() -> Self {
        Self::new()
    }
}
