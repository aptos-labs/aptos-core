// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::types::{Flag, MVCodeError, MVCodeOutput, TxnIndex};
use aptos_crypto::hash::{DefaultHasher, HashValue};
use aptos_types::{
    executable::{Executable, ExecutableDescriptor},
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

    /// Executable based on the storage version of the module.
    base_executable: Option<Arc<X>>,
    /// Executables corresponding to published versions of the module, based on hash.
    executables: HashMap<HashValue, Arc<X>>,
}

/// Maps each key (access path) to an interal VersionedValue.
pub struct VersionedCode<K, V: TransactionWrite, X: Executable> {
    values: DashMap<K, VersionedValue<V, X>>,
}

impl<V: TransactionWrite> Entry<V> {
    pub fn new_write_from(module: V) -> Entry<V> {
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
            base_executable: None,
            executables: HashMap::new(),
        }
    }

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

impl<V: TransactionWrite, X: Executable> Default for VersionedValue<V, X> {
    fn default() -> Self {
        VersionedValue::new()
    }
}

impl<K: Hash + Clone + Eq, V: TransactionWrite, X: Executable> VersionedCode<K, V, X> {
    pub(crate) fn new() -> Self {
        Self {
            values: DashMap::new(),
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
        let x = Arc::new(executable);
        match descriptor {
            ExecutableDescriptor::Published(hash) => {
                let mut v = self.values.get_mut(key).expect("Path must exist");
                v.executables.entry(hash).or_insert(x);
            },
            ExecutableDescriptor::Storage => {
                let mut v = self.values.entry(key.clone()).or_default();
                v.base_executable.get_or_insert(x);
            },
        };
    }

    pub(crate) fn fetch_code(
        &self,
        key: &K,
        txn_idx: TxnIndex,
    ) -> anyhow::Result<MVCodeOutput<V, X>, MVCodeError> {
        use MVCodeError::*;
        use MVCodeOutput::*;

        match self.values.get(key) {
            Some(v) => match v.read(txn_idx) {
                Ok((module, hash)) => Ok(match v.executables.get(&hash) {
                    Some(x) => Executable((x.clone(), ExecutableDescriptor::Published(hash))),
                    None => Module((module, hash)),
                }),
                Err(NotFound) => v
                    .base_executable
                    .as_ref()
                    .map(|x| Executable((x.clone(), ExecutableDescriptor::Storage)))
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
}

impl<K: Hash + Clone + Eq, V: TransactionWrite, X: Executable> Default for VersionedCode<K, V, X> {
    fn default() -> Self {
        VersionedCode::new()
    }
}
