// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crossbeam::utils::CachePadded;
use dashmap::DashMap;
use std::{
    collections::btree_map::BTreeMap,
    hash::Hash,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

#[cfg(test)]
mod unit_tests;

// TODO: re-use definitions with the scheduler.
pub type TxnIndex = usize;
pub type Incarnation = usize;
pub type Version = (TxnIndex, Incarnation);

const FLAG_DONE: usize = 0;
const FLAG_ESTIMATE: usize = 1;

/// Type of entry, recorded in the shared multi-version data-structure for each write.
struct WriteCell<V> {
    /// Used to mark the entry as a "write estimate".
    flag: AtomicUsize,
    /// Incarnation number of the transaction that wrote the entry. Note that
    /// TxnIndex is part of the key and not recorded here.
    incarnation: Incarnation,
    /// Actual data stored in a shared pointer (to ensure ownership and avoid clones).
    data: Arc<V>,
}

impl<V> WriteCell<V> {
    pub fn new_from(flag: usize, incarnation: Incarnation, data: V) -> WriteCell<V> {
        WriteCell {
            flag: AtomicUsize::new(flag),
            incarnation,
            data: Arc::new(data),
        }
    }

    pub fn flag(&self) -> usize {
        self.flag.load(Ordering::SeqCst)
    }

    pub fn mark_estimate(&self) {
        self.flag.store(FLAG_ESTIMATE, Ordering::SeqCst);
    }
}

/// Main multi-version data-structure used by threads to read/write during parallel
/// execution. Maps each access path to an interal BTreeMap that contains the indices
/// of transactions that write at the given access path alongside the corresponding
/// entries of WriteCell type.
///
/// Concurrency is managed by DashMap, i.e. when a method accesses a BTreeMap at a
/// given key, it holds exclusive access and doesn't need to explicitly synchronize
/// with other reader/writers.
pub struct MVHashMap<K, V> {
    data: DashMap<K, BTreeMap<TxnIndex, CachePadded<WriteCell<V>>>>,
}

impl<K: Hash + Clone + Eq, V> MVHashMap<K, V> {
    pub fn new() -> MVHashMap<K, V> {
        MVHashMap {
            data: DashMap::new(),
        }
    }

    /// Write a versioned data at a specified key. If the WriteCell entry is overwritten,
    /// asserts that the new incarnation is strictly higher.
    pub fn write(&self, key: &K, version: Version, data: V) {
        let (txn_idx, incarnation) = version;

        let mut map = self.data.entry(key.clone()).or_insert(BTreeMap::new());
        let prev_cell = map.insert(
            txn_idx,
            CachePadded::new(WriteCell::new_from(FLAG_DONE, incarnation, data)),
        );

        // Assert that the previous entry for txn_idx, if present, had lower incarnation.
        assert!(prev_cell
            .map(|cell| cell.incarnation < incarnation)
            .unwrap_or(true));
    }

    /// Mark an entry from transaction 'txn_idx' at access path 'key' as an estimated write
    /// (for future incarnation). Will panic if the entry is not in the data-structure.
    pub fn mark_estimate(&self, key: &K, txn_idx: TxnIndex) {
        let map = self.data.get(key).expect("Path must exist");
        map.get(&txn_idx)
            .expect("Entry by txn must exist")
            .mark_estimate();
    }

    /// Delete an entry from transaction 'txn_idx' at access path 'key'. Will panic
    /// if the access path has never been written before.
    pub fn delete(&self, key: &K, txn_idx: TxnIndex) {
        // TODO: investigate logical deletion.
        let mut map = self.data.get_mut(key).expect("Path must exist");
        map.remove(&txn_idx);
    }

    /// read may return Ok((Arc<V>, txn_idx, incarnation)), Err(dep_txn_idx) for
    /// a dependency of transaction dep_txn_idx or Err(None) when no prior entry is found.
    pub fn read(&self, key: &K, txn_idx: TxnIndex) -> Result<(Version, Arc<V>), Option<TxnIndex>> {
        match self.data.get(key) {
            Some(tree) => {
                // Find the dependency
                let mut iter = tree.range(0..txn_idx);
                if let Some((idx, write_cell)) = iter.next_back() {
                    let flag = write_cell.flag();

                    if flag == FLAG_ESTIMATE {
                        // Found a dependency.
                        Err(Some(*idx))
                    } else {
                        debug_assert!(flag == FLAG_DONE);
                        // The entry is populated, return its contents.
                        let write_version = (*idx, write_cell.incarnation);
                        Ok((write_version, write_cell.data.clone()))
                    }
                } else {
                    Err(None)
                }
            }
            None => Err(None),
        }
    }
}
