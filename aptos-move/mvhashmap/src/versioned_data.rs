// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::types::{Incarnation, MVDataError, MVDataOutput, TxnIndex, Version};
use aptos_aggregator::{
    delta_change_set::{deserialize, DeltaOp},
    transaction::AggregatorValue,
};
use aptos_infallible::Mutex;
use aptos_types::write_set::TransactionWrite;
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

const FLAG_DONE: usize = 0;
const FLAG_ESTIMATE: usize = 1;

/// Every entry in shared multi-version data-structure has an "estimate" flag
/// and some content.
struct Entry<V> {
    /// Used to mark the entry as a "write estimate". Even though the entry
    /// lives inside the DashMap and the entry access will have barriers, we
    /// still make the flag Atomic to provide acq/rel semantics on its own.
    flag: AtomicUsize,

    /// Actual contents.
    pub cell: EntryCell<V>,
}

/// Represents the content of a single entry in multi-version data-structure.
enum EntryCell<V> {
    /// Recorded in the shared multi-version data-structure for each write. It
    /// has: 1) Incarnation number of the transaction that wrote the entry (note
    /// that TxnIndex is part of the key and not recorded here), 2) actual data
    /// stored in a shared pointer (to ensure ownership and avoid clones).
    Write(Incarnation, Arc<V>),

    /// Recorded in the shared multi-version data-structure for each delta.
    Delta(DeltaOp),
}

/// A VersionedValue internally contains a BTreeMap from indices of transactions
/// that update the given access path alongside the corresponding entries.
struct VersionedValue<V> {
    versioned_map: BTreeMap<TxnIndex, CachePadded<Entry<V>>>,

    // Note: this can cache base (storage) value in Option<u128> to facilitate
    // aggregator validation & reading in the future, if needed.
    contains_delta: bool,
}

/// Maps each key (access path) to an interal VersionedValue.
pub struct VersionedData<K, V> {
    values: DashMap<K, VersionedValue<V>>,
    delta_keys: Mutex<Vec<K>>,
}

impl<V> Entry<V> {
    pub fn new_write_from(flag: usize, incarnation: Incarnation, data: V) -> Entry<V> {
        Entry {
            flag: AtomicUsize::new(flag),
            cell: EntryCell::Write(incarnation, Arc::new(data)),
        }
    }

    pub fn new_delta_from(flag: usize, data: DeltaOp) -> Entry<V> {
        Entry {
            flag: AtomicUsize::new(flag),
            cell: EntryCell::Delta(data),
        }
    }

    pub fn flag(&self) -> usize {
        self.flag.load(Ordering::Acquire)
    }

    pub fn mark_estimate(&self) {
        self.flag.store(FLAG_ESTIMATE, Ordering::Release);
    }
}

impl<V: TransactionWrite> VersionedValue<V> {
    pub fn new() -> Self {
        Self {
            versioned_map: BTreeMap::new(),
            contains_delta: false,
        }
    }
}

impl<V: TransactionWrite> Default for VersionedValue<V> {
    fn default() -> Self {
        VersionedValue::new()
    }
}

impl<K: Hash + Clone + Eq, V: TransactionWrite> VersionedData<K, V> {
    pub(crate) fn new() -> Self {
        Self {
            values: DashMap::new(),
            delta_keys: Mutex::new(Vec::new()),
        }
    }

    /// Takes the list of recorded keys that had an associated delta entry at any prior point.
    pub fn take_aggregator_keys(&self) -> Vec<K> {
        std::mem::take(&mut self.delta_keys.lock())
    }

    /// For processing outputs - removes the BTreeMap from the MVHashMap for a given
    /// key - the key should be an aggregator key with at least one delta update during
    /// the block execution (such keys are provided by 'aggregator_keys' function).
    /// Returns the aggregator values for each transaction index w. a delta update.
    pub fn take_materialized_deltas(
        &self,
        key: &K,
        base_value: Option<u128>,
    ) -> Vec<(TxnIndex, u128)> {
        let (_, v) = self
            .values
            .remove(key)
            .expect("No entry at MVHashMap for an aggregator key");
        assert!(v.contains_delta, "No delta update at an aggregator key");

        let mut latest_value = base_value;
        v.versioned_map
            .into_iter()
            .filter_map(|(idx, entry)| {
                match &entry.cell {
                    EntryCell::Write(_, data) => {
                        latest_value = data.extract_raw_bytes().map(|bytes| deserialize(&bytes));
                        None
                    },
                    EntryCell::Delta(delta) => {
                        // Apply to the latest value to obtain the materialized delta value.
                        let aggregator_value = delta
                            .apply_to(
                                latest_value
                                    .expect("Failed to apply delta to (non-existent) aggregator"),
                            )
                            .expect("Failed to apply aggregator delta output");

                        latest_value = Some(aggregator_value);
                        Some((idx, aggregator_value))
                    },
                }
            })
            .collect()
    }

    pub(crate) fn add_delta(&self, key: &K, txn_idx: TxnIndex, delta: DeltaOp) {
        let mut v = self.values.entry(key.clone()).or_default();
        v.versioned_map.insert(
            txn_idx,
            CachePadded::new(Entry::new_delta_from(FLAG_DONE, delta)),
        );

        if !v.contains_delta {
            v.contains_delta = true;
            self.delta_keys.lock().push(key.clone());
        }
    }

    pub(crate) fn mark_estimate(&self, key: &K, txn_idx: TxnIndex) {
        let v = self.values.get(key).expect("Path must exist");
        v.versioned_map
            .get(&txn_idx)
            .expect("Entry by the txn must exist to mark estimate")
            .mark_estimate();
    }

    pub(crate) fn delete(&self, key: &K, txn_idx: TxnIndex) {
        // TODO: investigate logical deletion.
        let mut v = self.values.get_mut(key).expect("Path must exist");
        assert!(
            v.versioned_map.remove(&txn_idx).is_some(),
            "Entry must exist to be deleted"
        );
    }

    pub(crate) fn fetch_data(
        &self,
        key: &K,
        txn_idx: TxnIndex,
    ) -> anyhow::Result<MVDataOutput<V>, MVDataError> {
        use MVDataError::*;
        use MVDataOutput::*;

        match self.values.get(key) {
            Some(v) => {
                let mut iter = v.versioned_map.range(0..txn_idx);

                // If read encounters a delta, it must traverse the block of transactions
                // (top-down) until it encounters a write or reaches the end of the block.
                // During traversal, all aggregator deltas have to be accumulated together.
                let mut accumulator: Option<Result<DeltaOp, ()>> = None;
                while let Some((idx, entry)) = iter.next_back() {
                    let flag = entry.flag();

                    if flag == FLAG_ESTIMATE {
                        // Found a dependency.
                        return Err(Dependency(*idx));
                    }

                    // The entry should be populated.
                    debug_assert!(flag == FLAG_DONE);

                    match (&entry.cell, accumulator.as_mut()) {
                        (EntryCell::Write(incarnation, data), None) => {
                            // Resolve to the write if no deltas were applied in between.
                            let write_version = (*idx, *incarnation);
                            return Ok(Versioned(write_version, data.clone()));
                        },
                        (EntryCell::Write(incarnation, data), Some(accumulator)) => {
                            // Deltas were applied. We must deserialize the value
                            // of the write and apply the aggregated delta accumulator.

                            // None if data represents deletion. Otherwise, panics if the
                            // data can't be resolved to an aggregator value.
                            let maybe_value = AggregatorValue::from_write(data.as_ref());

                            if maybe_value.is_none() {
                                // Resolve to the write if the WriteOp was deletion
                                // (MoveVM will observe 'deletion'). This takes precedence
                                // over any speculative delta accumulation errors on top.
                                let write_version = (*idx, *incarnation);
                                return Ok(Versioned(write_version, data.clone()));
                            }
                            return accumulator.map_err(|_| DeltaApplicationFailure).and_then(
                                |a| {
                                    // Apply accumulated delta to resolve the aggregator value.
                                    a.apply_to(maybe_value.unwrap().into())
                                        .map(|result| Resolved(result))
                                        .map_err(|_| DeltaApplicationFailure)
                                },
                            );
                        },
                        (EntryCell::Delta(delta), Some(accumulator)) => {
                            *accumulator = accumulator.and_then(|mut a| {
                                // Read hit a delta during traversing the block and aggregating
                                // other deltas. Merge two deltas together. If Delta application
                                // fails, we record an error, but continue processing (to e.g.
                                // account for the case when the aggregator was deleted).
                                if a.merge_onto(*delta).is_err() {
                                    Err(())
                                } else {
                                    Ok(a)
                                }
                            });
                        },
                        (EntryCell::Delta(delta), None) => {
                            // Read hit a delta and must start accumulating.
                            // Initialize the accumulator and continue traversal.
                            accumulator = Some(Ok(*delta))
                        },
                    }
                }

                // It can happen that while traversing the block and resolving
                // deltas the actual written value has not been seen yet (i.e.
                // it is not added as an entry to the data-structure).
                match accumulator {
                    Some(Ok(accumulator)) => Err(Unresolved(accumulator)),
                    Some(Err(_)) => Err(DeltaApplicationFailure),
                    None => Err(NotFound),
                }
            },
            None => Err(NotFound),
        }
    }

    pub(crate) fn write(&self, key: &K, version: Version, data: V) {
        let (txn_idx, incarnation) = version;

        let mut v = self.values.entry(key.clone()).or_default();
        let prev_entry = v.versioned_map.insert(
            txn_idx,
            CachePadded::new(Entry::new_write_from(FLAG_DONE, incarnation, data)),
        );

        // Assert that the previous entry for txn_idx, if present, had lower incarnation.
        assert!(prev_entry.map_or(true, |entry| -> bool {
            if let EntryCell::Write(i, _) = entry.cell {
                i < incarnation
            } else {
                true
            }
        }));
    }
}
