// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::types::{Flag, Incarnation, MVDataError, MVDataOutput, TxnIndex, Version};
use anyhow::Result;
use aptos_aggregator::{delta_change_set::DeltaOp, transaction::AggregatorValue};
use aptos_types::write_set::TransactionWrite;
use crossbeam::utils::CachePadded;
use dashmap::DashMap;
use std::{collections::btree_map::BTreeMap, fmt::Debug, hash::Hash, sync::Arc};

/// Every entry in shared multi-version data-structure has an "estimate" flag
/// and some content.
struct Entry<V> {
    /// Actual contents.
    cell: EntryCell<V>,

    /// Used to mark the entry as a "write estimate".
    flag: Flag,
}

/// Represents the content of a single entry in multi-version data-structure.
enum EntryCell<V> {
    /// Recorded in the shared multi-version data-structure for each write. It
    /// has: 1) Incarnation number of the transaction that wrote the entry (note
    /// that TxnIndex is part of the key and not recorded here), 2) actual data
    /// stored in a shared pointer (to ensure ownership and avoid clones).
    Write(Incarnation, Arc<V>),

    /// Recorded in the shared multi-version data-structure for each delta.
    /// Option<u128> is a shortcut to aggregated value (to avoid traversing down
    /// beyond this index), which is created after the corresponding txn is committed.
    Delta(DeltaOp, Option<u128>),
}

/// A VersionedValue internally contains a BTreeMap from indices of transactions
/// that update the given access path alongside the corresponding entries. It may
/// also contain a base value (value from storage) as u128 if the key corresponds
/// to an aggregator.
struct VersionedValue<V> {
    versioned_map: BTreeMap<TxnIndex, CachePadded<Entry<V>>>,

    // An aggregator value from storage can be here to avoid redundant storage calls.
    aggregator_base_value: Option<u128>,
}

/// Maps each key (access path) to an internal VersionedValue.
pub struct VersionedData<K, V> {
    values: DashMap<K, VersionedValue<V>>,
}

impl<V> Entry<V> {
    fn new_write_from(incarnation: Incarnation, data: V) -> Entry<V> {
        Entry {
            cell: EntryCell::Write(incarnation, Arc::new(data)),
            flag: Flag::Done,
        }
    }

    fn new_delta_from(data: DeltaOp) -> Entry<V> {
        Entry {
            cell: EntryCell::Delta(data, None),
            flag: Flag::Done,
        }
    }

    fn flag(&self) -> Flag {
        self.flag
    }

    fn mark_estimate(&mut self) {
        self.flag = Flag::Estimate;
    }

    // The entry must be a delta, will record the provided value as a base value
    // shortcut (the value in storage before block execution). If a value was already
    // recorded, the new value is asserted for equality.
    fn record_delta_shortcut(&mut self, value: u128) {
        use crate::versioned_data::EntryCell::Delta;

        self.cell = match self.cell {
            Delta(delta_op, maybe_shortcut) => {
                if let Some(prev_value) = maybe_shortcut {
                    assert_eq!(value, prev_value, "Recording different shortcuts");
                }
                Delta(delta_op, Some(value))
            },
            _ => unreachable!("Must contain a delta"),
        }
    }
}

impl<V: TransactionWrite> VersionedValue<V> {
    fn new() -> Self {
        Self {
            versioned_map: BTreeMap::new(),
            aggregator_base_value: None,
        }
    }

    fn read(&self, txn_idx: TxnIndex) -> anyhow::Result<MVDataOutput<V>, MVDataError> {
        use MVDataError::*;
        use MVDataOutput::*;

        let mut iter = self.versioned_map.range(0..txn_idx);

        // If read encounters a delta, it must traverse the block of transactions
        // (top-down) until it encounters a write or reaches the end of the block.
        // During traversal, all aggregator deltas have to be accumulated together.
        let mut accumulator: Option<Result<DeltaOp, ()>> = None;
        while let Some((idx, entry)) = iter.next_back() {
            if entry.flag() == Flag::Estimate {
                // Found a dependency.
                return Err(Dependency(*idx));
            }

            match (&entry.cell, accumulator.as_mut()) {
                (EntryCell::Write(incarnation, data), None) => {
                    // Resolve to the write if no deltas were applied in between.
                    let write_version = (*idx, *incarnation);
                    return Ok(Versioned(write_version, data.clone()));
                },
                (EntryCell::Write(incarnation, data), Some(accumulator)) => {
                    // Deltas were applied. We must deserialize the value
                    // of the write and apply the aggregated delta accumulator.
                    return match AggregatorValue::from_write(data.as_ref()) {
                        None => {
                            // Resolve to the write if the WriteOp was deletion
                            // (MoveVM will observe 'deletion'). This takes precedence
                            // over any speculative delta accumulation errors on top.
                            let write_version = (*idx, *incarnation);
                            Ok(Versioned(write_version, data.clone()))
                        },
                        Some(value) => {
                            // Panics if the data can't be resolved to an aggregator value.
                            accumulator
                                .map_err(|_| DeltaApplicationFailure)
                                .and_then(|a| {
                                    // Apply accumulated delta to resolve the aggregator value.
                                    a.apply_to(value.into())
                                        .map(|result| Resolved(result))
                                        .map_err(|_| DeltaApplicationFailure)
                                })
                        },
                    };
                },
                (EntryCell::Delta(delta, maybe_shortcut), Some(accumulator)) => {
                    if let Some(shortcut_value) = maybe_shortcut {
                        return accumulator
                            .map_err(|_| DeltaApplicationFailure)
                            .and_then(|a| {
                                // Apply accumulated delta to resolve the aggregator value.
                                a.apply_to(*shortcut_value)
                                    .map(|result| Resolved(result))
                                    .map_err(|_| DeltaApplicationFailure)
                            });
                    }

                    *accumulator = accumulator.and_then(|mut a| {
                        // Read hit a delta during traversing the block and aggregating
                        // other deltas. Merge two deltas together. If Delta application
                        // fails, we record an error, but continue processing (to e.g.
                        // account for the case when the aggregator was deleted).
                        if a.merge_with_previous_delta(*delta).is_err() {
                            Err(())
                        } else {
                            Ok(a)
                        }
                    });
                },
                (EntryCell::Delta(delta, maybe_shortcut), None) => {
                    if let Some(shortcut_value) = maybe_shortcut {
                        return Ok(Resolved(*shortcut_value));
                    }

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
            Some(Ok(accumulator)) => match self.aggregator_base_value {
                Some(base_value) => accumulator
                    .apply_to(base_value)
                    .map(|result| Resolved(result))
                    .map_err(|_| DeltaApplicationFailure),
                None => Err(Unresolved(accumulator)),
            },
            Some(Err(_)) => Err(DeltaApplicationFailure),
            None => Err(NotFound),
        }
    }
}

impl<V: TransactionWrite> Default for VersionedValue<V> {
    fn default() -> Self {
        VersionedValue::new()
    }
}

impl<K: Hash + Clone + Debug + Eq, V: TransactionWrite> VersionedData<K, V> {
    pub(crate) fn new() -> Self {
        Self {
            values: DashMap::new(),
        }
    }

    pub(crate) fn set_aggregator_base_value(&self, key: &K, value: u128) {
        let mut v = self.values.get_mut(key).expect("Path must exist");

        // Record base value. If a value was added by another thread, assert they're equal.
        assert_eq!(*v.aggregator_base_value.get_or_insert(value), value);
    }

    pub(crate) fn add_delta(&self, key: K, txn_idx: TxnIndex, delta: DeltaOp) {
        let mut v = self.values.entry(key).or_default();
        v.versioned_map
            .insert(txn_idx, CachePadded::new(Entry::new_delta_from(delta)));
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

    /// Delete an entry from transaction 'txn_idx' at access path 'key'. Will panic
    /// if the corresponding entry does not exist.
    pub fn delete(&self, key: &K, txn_idx: TxnIndex) {
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
        self.values
            .get(key)
            .map(|v| v.read(txn_idx))
            .unwrap_or(Err(MVDataError::NotFound))
    }

    /// Versioned write of data at a given key (and version).
    pub fn write(&self, key: K, version: Version, data: V) {
        let (txn_idx, incarnation) = version;

        let mut v = self.values.entry(key).or_default();
        let prev_entry = v.versioned_map.insert(
            txn_idx,
            CachePadded::new(Entry::new_write_from(incarnation, data)),
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

    /// When a transaction is committed, this method can be called for its delta outputs to add
    /// a 'shortcut' to the corresponding materialized aggregator value, so any subsequent reads
    /// do not have to traverse below the index. It must be guaranteed by the caller that the
    /// data recorded below this index will not change after the call, and that the corresponding
    /// transaction has indeed produced a delta recorded at the given key.
    ///
    /// If the result is Err(op), it means the base value to apply DeltaOp op hadn't been set.
    pub(crate) fn materialize_delta(&self, key: &K, txn_idx: TxnIndex) -> Result<u128, DeltaOp> {
        let mut v = self.values.get_mut(key).expect("Path must exist");

        // +1 makes sure we include the delta from txn_idx.
        match v.read(txn_idx + 1) {
            Ok(MVDataOutput::Resolved(value)) => {
                v.versioned_map
                    .get_mut(&txn_idx)
                    .expect("Entry by the txn must exist to commit delta")
                    .record_delta_shortcut(value);

                Ok(value)
            },
            Err(MVDataError::Unresolved(op)) => Err(op),
            _ => unreachable!(
                "Must resolve delta at key = {:?}, txn_idx = {}",
                key, txn_idx
            ),
        }
    }
}
