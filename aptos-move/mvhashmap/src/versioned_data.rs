// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::types::{
    Incarnation, MVDataError, MVDataOutput, ShiftedTxnIndex, TxnIndex, ValueWithLayout,
};
use anyhow::Result;
use aptos_aggregator::delta_change_set::DeltaOp;
use aptos_types::write_set::TransactionWrite;
use claims::assert_some;
use crossbeam::utils::CachePadded;
use dashmap::DashMap;
use move_core_types::value::MoveTypeLayout;
use std::{
    collections::btree_map::{self, BTreeMap},
    fmt::Debug,
    hash::Hash,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
};

pub(crate) const FLAG_DONE: bool = false;
pub(crate) const FLAG_ESTIMATE: bool = true;

/// Every entry in shared multi-version data-structure has an "estimate" flag
/// and some content.
/// TODO: can remove pub(crate) once aggregator V1 is deprecated.
pub(crate) struct Entry<V> {
    /// Actual contents.
    pub(crate) value: V,

    /// Used to mark the entry as a "write estimate". Stored as an atomic so
    /// marking an estimate can proceed w. read lock.
    flag: AtomicBool,
}

/// Represents the content of a single entry in multi-version data-structure.
enum EntryCell<V> {
    /// Recorded in the shared multi-version data-structure for each write. It
    /// has: 1) Incarnation number of the transaction that wrote the entry (note
    /// that TxnIndex is part of the key and not recorded here), 2) actual data
    /// stored in a shared pointer (to ensure ownership and avoid clones).
    Write(Incarnation, ValueWithLayout<V>),

    /// Recorded in the shared multi-version data-structure for each delta.
    /// Option<u128> is a shortcut to aggregated value (to avoid traversing down
    /// beyond this index), which is created after the corresponding txn is committed.
    Delta(DeltaOp, Option<u128>),
}

/// A versioned value internally is represented as a BTreeMap from indices of
/// transactions that update the given access path & the corresponding entries.
struct VersionedValue<V> {
    versioned_map: BTreeMap<ShiftedTxnIndex, CachePadded<Entry<EntryCell<V>>>>,
}

/// Maps each key (access path) to an internal versioned value representation.
pub struct VersionedData<K, V> {
    values: DashMap<K, VersionedValue<V>>,
    total_base_value_size: AtomicU64,
}

fn new_write_entry<V>(incarnation: Incarnation, value: ValueWithLayout<V>) -> Entry<EntryCell<V>> {
    Entry::new(EntryCell::Write(incarnation, value))
}

fn new_delta_entry<V>(data: DeltaOp) -> Entry<EntryCell<V>> {
    Entry::new(EntryCell::Delta(data, None))
}

impl<V> Entry<V> {
    pub(crate) fn new(value: V) -> Entry<V> {
        Entry {
            value,
            flag: AtomicBool::new(FLAG_DONE),
        }
    }

    pub(crate) fn is_estimate(&self) -> bool {
        self.flag.load(Ordering::Relaxed) == FLAG_ESTIMATE
    }

    pub(crate) fn mark_estimate(&self) {
        self.flag.store(FLAG_ESTIMATE, Ordering::Relaxed);
    }
}

impl<V> Entry<EntryCell<V>> {
    // The entry must be a delta, will record the provided value as a base value
    // shortcut (the value in storage before block execution). If a value was already
    // recorded, the new value is asserted for equality.
    fn record_delta_shortcut(&mut self, value: u128) {
        use crate::versioned_data::EntryCell::Delta;

        self.value = match self.value {
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

impl<V: TransactionWrite> Default for VersionedValue<V> {
    fn default() -> Self {
        Self {
            versioned_map: BTreeMap::new(),
        }
    }
}

impl<V: TransactionWrite> VersionedValue<V> {
    fn read(&self, txn_idx: TxnIndex) -> anyhow::Result<MVDataOutput<V>, MVDataError> {
        use MVDataError::*;
        use MVDataOutput::*;

        let mut iter = self
            .versioned_map
            .range(ShiftedTxnIndex::zero_idx()..ShiftedTxnIndex::new(txn_idx));

        // If read encounters a delta, it must traverse the block of transactions
        // (top-down) until it encounters a write or reaches the end of the block.
        // During traversal, all aggregator deltas have to be accumulated together.
        let mut accumulator: Option<Result<DeltaOp, ()>> = None;
        while let Some((idx, entry)) = iter.next_back() {
            if entry.is_estimate() {
                // Found a dependency.
                return Err(Dependency(
                    idx.idx().expect("May not depend on storage version"),
                ));
            }

            match (&entry.value, accumulator.as_mut()) {
                (EntryCell::Write(incarnation, data), None) => {
                    // Resolve to the write if no deltas were applied in between.
                    return Ok(Versioned(
                        idx.idx().map(|idx| (idx, *incarnation)),
                        data.clone(),
                    ));
                },
                (EntryCell::Write(incarnation, data), Some(accumulator)) => {
                    // Deltas were applied. We must deserialize the value
                    // of the write and apply the aggregated delta accumulator.
                    let value = data.extract_value_no_layout();
                    return match value
                        .as_u128()
                        .expect("Aggregator value must deserialize to u128")
                    {
                        None => {
                            // Resolve to the write if the WriteOp was deletion
                            // (MoveVM will observe 'deletion'). This takes precedence
                            // over any speculative delta accumulation errors on top.
                            Ok(Versioned(
                                idx.idx().map(|idx| (idx, *incarnation)),
                                data.clone(),
                            ))
                        },
                        Some(value) => {
                            // Panics if the data can't be resolved to an aggregator value.
                            accumulator
                                .map_err(|_| DeltaApplicationFailure)
                                .and_then(|a| {
                                    // Apply accumulated delta to resolve the aggregator value.
                                    a.apply_to(value)
                                        .map(Resolved)
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
                                    .map(Resolved)
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
            Some(Ok(accumulator)) => Err(Unresolved(accumulator)),
            Some(Err(_)) => Err(DeltaApplicationFailure),
            None => Err(Uninitialized),
        }
    }
}

impl<K: Hash + Clone + Debug + Eq, V: TransactionWrite> VersionedData<K, V> {
    pub(crate) fn empty() -> Self {
        Self {
            values: DashMap::new(),
            total_base_value_size: AtomicU64::new(0),
        }
    }

    pub(crate) fn num_keys(&self) -> usize {
        self.values.len()
    }

    pub(crate) fn total_base_value_size(&self) -> u64 {
        self.total_base_value_size.load(Ordering::Relaxed)
    }

    pub fn add_delta(&self, key: K, txn_idx: TxnIndex, delta: DeltaOp) {
        let mut v = self.values.entry(key).or_default();
        v.versioned_map.insert(
            ShiftedTxnIndex::new(txn_idx),
            CachePadded::new(new_delta_entry(delta)),
        );
    }

    /// Mark an entry from transaction 'txn_idx' at access path 'key' as an estimated write
    /// (for future incarnation). Will panic if the entry is not in the data-structure.
    pub fn mark_estimate(&self, key: &K, txn_idx: TxnIndex) {
        let v = self.values.get(key).expect("Path must exist");
        v.versioned_map
            .get(&ShiftedTxnIndex::new(txn_idx))
            .expect("Entry by the txn must exist to mark estimate")
            .mark_estimate();
    }

    /// Delete an entry from transaction 'txn_idx' at access path 'key'. Will panic
    /// if the corresponding entry does not exist.
    pub fn remove(&self, key: &K, txn_idx: TxnIndex) {
        // TODO: investigate logical deletion.
        let mut v = self.values.get_mut(key).expect("Path must exist");
        assert_some!(
            v.versioned_map.remove(&ShiftedTxnIndex::new(txn_idx)),
            "Entry for key / idx must exist to be deleted"
        );
    }

    pub fn fetch_data(
        &self,
        key: &K,
        txn_idx: TxnIndex,
    ) -> anyhow::Result<MVDataOutput<V>, MVDataError> {
        self.values
            .get(key)
            .map(|v| v.read(txn_idx))
            .unwrap_or(Err(MVDataError::Uninitialized))
    }

    pub fn set_base_value(&self, key: K, value: ValueWithLayout<V>) {
        let mut v = self.values.entry(key).or_default();
        // For base value, incarnation is irrelevant, and is always set to 0.

        use btree_map::Entry::*;
        use ValueWithLayout::*;
        match v.versioned_map.entry(ShiftedTxnIndex::zero_idx()) {
            Vacant(v) => {
                if let Some(base_size) = value.bytes_len() {
                    self.total_base_value_size
                        .fetch_add(base_size as u64, Ordering::Relaxed);
                }
                v.insert(CachePadded::new(new_write_entry(0, value)));
            },
            Occupied(mut o) => {
                if let EntryCell::Write(i, existing_value) = &o.get().value {
                    assert!(*i == 0);
                    match (existing_value, &value) {
                        (RawFromStorage(ev), RawFromStorage(v)) => {
                            // Base value from storage needs to be identical
                            // Assert the length of bytes for efficiency (instead of full equality)
                            assert!(v.bytes().map(|b| b.len()) == ev.bytes().map(|b| b.len()))
                        },
                        (Exchanged(_, _), RawFromStorage(_)) => {
                            // Stored value contains more info, nothing to do.
                        },
                        (RawFromStorage(_), Exchanged(_, _)) => {
                            // Received more info, update.
                            o.insert(CachePadded::new(new_write_entry(0, value)));
                        },
                        (Exchanged(ev, e_layout), Exchanged(v, layout)) => {
                            // base value may have already been provided by another transaction
                            // executed simultaneously and asking for the same resource.
                            // Value from storage must be identical, but then delayed field
                            // identifier exchange could've modified it.
                            //
                            // If maybe_layout is None, they are required to be identical
                            // If maybe_layout is Some, there might have been an exchange
                            // Assert the length of bytes for efficiency (instead of full equality)
                            assert_eq!(e_layout.is_some(), layout.is_some());
                            if layout.is_none() {
                                assert_eq!(v.bytes().map(|b| b.len()), ev.bytes().map(|b| b.len()));
                            }
                        },
                    }
                }
            },
        };
    }

    /// Versioned write of data at a given key (and version).
    pub fn write(
        &self,
        key: K,
        txn_idx: TxnIndex,
        incarnation: Incarnation,
        data: Arc<V>,
        maybe_layout: Option<Arc<MoveTypeLayout>>,
    ) {
        let mut v = self.values.entry(key).or_default();
        let prev_entry = v.versioned_map.insert(
            ShiftedTxnIndex::new(txn_idx),
            CachePadded::new(new_write_entry(
                incarnation,
                ValueWithLayout::Exchanged(data, maybe_layout),
            )),
        );

        // Assert that the previous entry for txn_idx, if present, had lower incarnation.
        assert!(prev_entry.map_or(true, |entry| -> bool {
            if let EntryCell::Write(i, _) = entry.value {
                i < incarnation
            } else {
                true
            }
        }));
    }

    /// Versioned write of metadata at a given resource group key (and version). Returns true
    /// if the previously stored metadata has changed as observed by later transactions (e.g.
    /// metadata of a deletion can never be observed by later transactions).
    pub fn write_metadata(
        &self,
        key: K,
        txn_idx: TxnIndex,
        incarnation: Incarnation,
        data: V,
    ) -> bool {
        let arc_data = Arc::new(data);

        let mut v = self.values.entry(key).or_default();
        let prev_entry = v.versioned_map.insert(
            ShiftedTxnIndex::new(txn_idx),
            CachePadded::new(new_write_entry(
                incarnation,
                ValueWithLayout::Exchanged(arc_data.clone(), None),
            )),
        );

        // Changes versioned metadata that was stored.
        prev_entry.map_or(true, |entry| -> bool {
            if let EntryCell::Write(_, existing_v) = &entry.value {
                arc_data.as_state_value_metadata()
                    != existing_v
                        .extract_value_no_layout()
                        .as_state_value_metadata()
            } else {
                unreachable!("Group metadata can't be written at AggregatorV1 key");
            }
        })
    }

    /// When a transaction is committed, this method can be called for its delta outputs to add
    /// a 'shortcut' to the corresponding materialized aggregator value, so any subsequent reads
    /// do not have to traverse below the index. It must be guaranteed by the caller that the
    /// data recorded below this index will not change after the call, and that the corresponding
    /// transaction has indeed produced a delta recorded at the given key.
    ///
    /// If the result is Err(op), it means the base value to apply DeltaOp op hadn't been set.
    pub fn materialize_delta(&self, key: &K, txn_idx: TxnIndex) -> Result<u128, DeltaOp> {
        let mut v = self.values.get_mut(key).expect("Path must exist");

        // +1 makes sure we include the delta from txn_idx.
        match v.read(txn_idx + 1) {
            Ok(MVDataOutput::Resolved(value)) => {
                v.versioned_map
                    .get_mut(&ShiftedTxnIndex::new(txn_idx))
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
