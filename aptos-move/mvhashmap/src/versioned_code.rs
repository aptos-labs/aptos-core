// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    types::{Flag, MVCodeError, MVCodeOutput, TxnIndex},
    utils::module_hash,
};
use aptos_crypto::hash::HashValue;
use aptos_executable_store::ExecutableStore;
use aptos_types::{
    executable::{Executable, ExecutableDescriptor, ModulePath},
    write_set::TransactionWrite,
};
use crossbeam::utils::CachePadded;
use dashmap::DashMap;
use rayon::iter::ParallelIterator;
use std::{
    collections::{btree_map::BTreeMap, HashMap},
    hash::Hash,
    sync::Arc,
};

/// Every entry in shared multi-version data-structure has an "estimate" flag
/// and some content.
struct Entry<V: TransactionWrite + Send + Sync> {
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
struct VersionedValue<V: TransactionWrite + Send + Sync, X: Executable> {
    versioned_map: BTreeMap<TxnIndex, CachePadded<Entry<V>>>,

    /// Executables corresponding to published versions of the module, based on hash.
    executables: HashMap<HashValue, X>,
}

/// Maps each key (access path) to an interal VersionedValue.
pub struct VersionedCode<
    K: ModulePath + Hash + Clone + Eq + Send + Sync,
    V: TransactionWrite + Send + Sync,
    X: Executable,
> {
    values: DashMap<K, VersionedValue<V, X>>,
    base_executables: Arc<ExecutableStore<K, X>>,
}

impl<V: TransactionWrite + Send + Sync> Entry<V> {
    pub fn new_write_from(module: V) -> Entry<V> {
        // Compute the module hash eagerly (Note: we could do it on access).
        let hash = module_hash(&module);

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

impl<V: TransactionWrite + Send + Sync, X: Executable> VersionedValue<V, X> {
    fn read(&self, txn_idx: TxnIndex) -> anyhow::Result<(Arc<V>, HashValue), MVCodeError> {
        use MVCodeError::*;

        if let Some((idx, entry)) = self.versioned_map.range(0..txn_idx).next_back() {
            if entry.flag() == Flag::Estimate {
                // Found a dependency.
                return Err(Dependency(*idx));
            }

            Ok((entry.module.clone(), entry.hash))
        } else {
            Err(NotFound)
        }
    }
}

impl<V: TransactionWrite + Send + Sync, X: Executable> Default for VersionedValue<V, X> {
    fn default() -> Self {
        Self {
            versioned_map: BTreeMap::new(),
            executables: HashMap::new(),
        }
    }
}

impl<
        K: ModulePath + Hash + Clone + Eq + Send + Sync,
        V: TransactionWrite + Send + Sync,
        X: Executable,
    > VersionedCode<K, V, X>
{
    pub(crate) fn new(base_executables: Arc<ExecutableStore<K, X>>) -> Self {
        Self {
            values: DashMap::new(),
            base_executables,
        }
    }

    pub(crate) fn mark_estimate(&self, key: &K, txn_idx: TxnIndex) {
        let mut v = self.values.get_mut(key).expect("Path must exist");
        v.versioned_map
            .get_mut(&txn_idx)
            .expect("Entry by the txn must exist to mark estimate")
            .mark_estimate();
    }

    pub(crate) fn write(&self, key: &K, txn_idx: TxnIndex, data: V) {
        let mut v = self.values.entry(key.clone()).or_default();
        v.versioned_map
            .insert(txn_idx, CachePadded::new(Entry::new_write_from(data)));
    }

    pub(crate) fn store_executable(
        &self,
        key: &K,
        descriptor: ExecutableDescriptor,
        executable: X,
    ) {
        match descriptor {
            ExecutableDescriptor::Published(hash) => {
                let mut v = self.values.get_mut(key).expect("Path must exist");
                v.executables.entry(hash).or_insert(executable);
            },
            ExecutableDescriptor::Storage => {
                self.base_executables.insert(key.clone(), executable);
            },
        };
    }

    pub(crate) fn fetch_code(
        &self,
        key: &K,
        txn_idx: TxnIndex,
    ) -> anyhow::Result<MVCodeOutput<Arc<V>, X>, MVCodeError> {
        use MVCodeError::*;
        use MVCodeOutput::*;

        match self.values.get(key) {
            Some(v) => match v.read(txn_idx) {
                Ok((module, hash)) => Ok(match v.executables.get(&hash) {
                    Some(x) => Executable((x.clone(), ExecutableDescriptor::Published(hash))),
                    None => Module((module, hash)),
                }),
                Err(NotFound) => self
                    .base_executables
                    .get(key)
                    .map(|x| Executable((x, ExecutableDescriptor::Storage)))
                    .ok_or(NotFound),
                Err(Dependency(idx)) => Err(Dependency(idx)),
            },
            None => Err(NotFound),
        }
    }

    pub(crate) fn delete(&self, key: &K, txn_idx: TxnIndex) {
        // TODO: investigate logical deletion.
        let mut v = self.values.get_mut(key).expect("Path must exist");
        assert!(
            v.versioned_map.remove(&txn_idx).is_some(),
            "Entry must exist to be deleted"
        );
    }

    /// Prepares base_executables to be used by the next block, by processing the latest
    /// written modules in the versioned_map. If an executable for latest module is available,
    /// it is stored, and otherwise (executable isn't available for a module written during
    /// the last block) any previous base_executable is cleared.
    ///
    /// Uses rayon for parallel processing the updates, so recommended to call with a rayon
    /// threadpool installed.
    pub(crate) fn update_base_executables(&self) {
        self.values.par_iter_mut().for_each(|mut v| {
            let new_base_hash = v.versioned_map.last_key_value().map(|(_, e)| e.hash);
            if let Some(h) = new_base_hash {
                match v.executables.remove(&h) {
                    Some(x) => self.base_executables.insert(v.key().clone(), x),
                    None => self.base_executables.remove(v.key()),
                }
            }
        });

        // Mark that the after block execution update has been completed on the cache.
        self.base_executables.mark_updated();
    }
}
