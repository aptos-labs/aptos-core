// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::types::{
    Incarnation, MVDataError, MVDataOutput, ShiftedTxnIndex, TxnIndex, ValueWithLayout,
};
use anyhow::Result;
use aptos_aggregator::delta_change_set::DeltaOp;
use aptos_infallible::Mutex;
use aptos_types::write_set::TransactionWrite;
use claims::assert_some;
use crossbeam::utils::CachePadded;
use dashmap::DashMap;
use equivalent::Equivalent;
use move_core_types::value::MoveTypeLayout;
use std::{
    collections::{
        btree_map::{self, BTreeMap},
        BTreeSet,
    },
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
    /// stored in a shared pointer (to ensure ownership and avoid clones). and
    /// a mutex-protected set of (txn_idx, incarnation) pairs that have registered
    /// a read of this entry.
    ResourceWrite(
        Incarnation,
        ValueWithLayout<V>,
        Mutex<BTreeSet<(TxnIndex, Incarnation)>>,
    ),

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

fn new_write_entry<V>(
    incarnation: Incarnation,
    value: ValueWithLayout<V>,
    dependencies: BTreeSet<(TxnIndex, Incarnation)>,
) -> Entry<EntryCell<V>> {
    Entry::new(EntryCell::ResourceWrite(
        incarnation,
        value,
        Mutex::new(dependencies),
    ))
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

// TODO(BlockSTMv2): remove TransactionWrite trait requirement from V, even if
// AggregatorV1 code is not removed by definining a more specialized trait that can
// runtime assert in other variants. Add other variants to stored cells that allow
// storing group metadata and group size together at the group key (currently metadata
// is stored in a fake write entry, and group size is stored in MVGroupData map).
impl<V: TransactionWrite + PartialEq> VersionedValue<V> {
    // Extracts read dependencies from the data-structure that are affected by a write
    // of 'data' at 'txn_idx'. Some of these dependencies that remain valid can be
    // relocated by a caller to a different (new) entry in the data-structure. The
    // boolean flag indicates whether the dependency is still valid after the write.
    fn get_affected_read_dependencies<const ONLY_COMPARE_METADATA: bool>(
        &self,
        txn_idx: TxnIndex,
        data: &Arc<V>,
        maybe_layout: &Option<Arc<MoveTypeLayout>>,
    ) -> (BTreeSet<(TxnIndex, Incarnation)>, bool) {
        let mut affected_deps = BTreeSet::new();
        let mut validation_result = true;

        // Look at entries at or below txn_idx
        if let Some((_, entry)) = self
            .versioned_map
            .range(..=ShiftedTxnIndex::new(txn_idx))
            .next_back()
        {
            if let EntryCell::ResourceWrite(_, value, dependencies) = &entry.value {
                // Take dependencies above txn_idx
                let mut deps = dependencies.lock();
                let new_deps = deps.split_off(&(txn_idx + 1, 0));
                if !new_deps.is_empty() {
                    // Only validate if we have dependencies to check.
                    // Validate by comparing both value and layout, or metadata if only_compare_metadata is true
                    validation_result = if let ValueWithLayout::Exchanged(v, l) = value {
                        if ONLY_COMPARE_METADATA {
                            v.as_state_value_metadata() == data.as_state_value_metadata()
                        } else {
                            v == data && l == maybe_layout
                        }
                    } else {
                        false
                    };
                }
                affected_deps = new_deps;
            }
        }
        (affected_deps, validation_result)
    }

    /// Handle dependencies from a removed entry by validating against the next (lower) entry.
    /// The caller MUST ensure that the entry at txn_idx has been removed from versioned_map
    /// before calling this method. If the lower entry does not exist, all dependencies are
    /// considered invalidated. A re-execution of the read can then create a baseline sentinel
    /// entry, if needed. Data is the value that was stored in the removed entry.
    fn handle_removed_dependencies<const ONLY_COMPARE_METADATA: bool>(
        &mut self,
        txn_idx: TxnIndex,
        mut dependencies: BTreeSet<(TxnIndex, Incarnation)>,
        data: &Arc<V>,
        maybe_layout: &Option<Arc<MoveTypeLayout>>,
    ) -> BTreeSet<(TxnIndex, Incarnation)> {
        // If we have dependencies and a next (lower) entry exists, validate against it.
        if !dependencies.is_empty() {
            if let Some((idx, next_entry)) = self
                .versioned_map
                .range(..=ShiftedTxnIndex::new(txn_idx))
                .next_back()
            {
                assert_ne!(
                    idx.idx(),
                    Ok(txn_idx),
                    "Entry at txn_idx must be removed before calling handle_removed_dependencies"
                );

                if let EntryCell::ResourceWrite(_, ValueWithLayout::Exchanged(v, l), next_deps) =
                    &next_entry.value
                {
                    let validation_passed = if ONLY_COMPARE_METADATA {
                        v.as_state_value_metadata() == data.as_state_value_metadata()
                    } else {
                        v == data && l == maybe_layout
                    };

                    if validation_passed {
                        // If validation passed, add dependencies to next entry and clear them.
                        next_deps.lock().extend(std::mem::take(&mut dependencies));
                    }
                }
            }
        }
        dependencies
    }

    // 'maybe_reader_incarnation' is None for BlockSTMv1 and always set for BlockSTMv2.
    fn read(
        &self,
        reader_txn_idx: TxnIndex,
        maybe_reader_incarnation: Option<Incarnation>,
    ) -> Result<MVDataOutput<V>, MVDataError> {
        use MVDataError::*;
        use MVDataOutput::*;

        let mut iter = self
            .versioned_map
            .range(ShiftedTxnIndex::zero_idx()..ShiftedTxnIndex::new(reader_txn_idx));

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
                (EntryCell::ResourceWrite(incarnation, data, dependencies), None) => {
                    // Record the read dependency (only in V2 case, not to add contention to V1).
                    if let Some(reader_incarnation) = maybe_reader_incarnation {
                        dependencies
                            .lock()
                            .insert((reader_txn_idx, reader_incarnation));
                    }

                    // Resolve to the write if no deltas were applied in between.
                    return Ok(Versioned(
                        idx.idx().map(|idx| (idx, *incarnation)),
                        data.clone(),
                    ));
                },
                (EntryCell::ResourceWrite(incarnation, data, _), Some(accumulator)) => {
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

impl<K: Hash + Clone + Debug + Eq, V: TransactionWrite + PartialEq> VersionedData<K, V> {
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
    /// Use the Equivalent trait for key lookup. This avoids having to clone the key when
    /// only a reference is needed.
    pub fn mark_estimate<Q>(&self, key: &Q, txn_idx: TxnIndex)
    where
        Q: Equivalent<K> + Hash,
    {
        // Use dashmap's get method which accepts a reference when Borrow is implemented
        // The equivalent crate automatically implements the right traits.
        let v = self.values.get(key).expect("Path must exist");
        v.versioned_map
            .get(&ShiftedTxnIndex::new(txn_idx))
            .expect("Entry by the txn must exist to mark estimate")
            .mark_estimate();
    }

    /// Delete an entry from transaction 'txn_idx' at access path 'key'. Will panic
    /// if the corresponding entry does not exist.
    pub fn remove<Q>(&self, key: &Q, txn_idx: TxnIndex)
    where
        Q: Equivalent<K> + Hash,
    {
        // TODO: investigate logical deletion.
        let mut v = self.values.get_mut(key).expect("Path must exist");
        assert_some!(
            v.versioned_map.remove(&ShiftedTxnIndex::new(txn_idx)),
            "Entry for key / idx must exist to be deleted"
        );
    }

    /// Delete an entry from transaction 'txn_idx' at access path 'key' for BlockSTMv2.
    /// Returns read dependencies from the entry that are no longer valid, panics if
    /// the entry does not exist.
    pub fn remove_v2<Q, const ONLY_COMPARE_METADATA: bool>(
        &self,
        key: &Q,
        txn_idx: TxnIndex,
    ) -> BTreeSet<(TxnIndex, Incarnation)>
    where
        Q: Equivalent<K> + Hash,
    {
        let mut v = self.values.get_mut(key).expect("Path must exist");

        // Get the entry to be removed
        let removed_entry = v
            .versioned_map
            .remove(&ShiftedTxnIndex::new(txn_idx))
            .expect("Entry for key / idx must exist to be deleted");

        if let EntryCell::ResourceWrite(_, ValueWithLayout::Exchanged(data, layout), deps) =
            &removed_entry.value
        {
            let removed_deps = std::mem::take(&mut *deps.lock());
            v.handle_removed_dependencies::<ONLY_COMPARE_METADATA>(
                txn_idx,
                removed_deps,
                data,
                layout,
            )
        } else {
            BTreeSet::new()
        }
    }

    pub fn fetch_data<Q>(
        &self,
        key: &Q,
        txn_idx: TxnIndex,
    ) -> anyhow::Result<MVDataOutput<V>, MVDataError>
    where
        Q: Equivalent<K> + Hash,
    {
        self.values
            .get(key)
            .map(|v| v.read(txn_idx, None))
            .unwrap_or(Err(MVDataError::Uninitialized))
    }

    // TODO(BlockSTMv2): Have a dispatch or dedicated interfaces for reading data,
    // metadata, size, and exists predicate. Return the appropriately cast value, record
    // the kind of each read dependency, then validate accordingly on write / removal.
    pub fn fetch_data_v2<Q>(
        &self,
        key: &Q,
        txn_idx: TxnIndex,
        incarnation: Incarnation,
    ) -> Result<MVDataOutput<V>, MVDataError>
    where
        Q: Equivalent<K> + Hash,
    {
        self.values
            .get(key)
            .map(|v| v.read(txn_idx, Some(incarnation)))
            .unwrap_or(Err(MVDataError::Uninitialized))
    }

    // The caller needs to repeat the read after set_base_value (concurrent caller might have
    // exchanged and stored a different delayed field ID).
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
                v.insert(CachePadded::new(new_write_entry(0, value, BTreeSet::new())));
            },
            Occupied(mut o) => {
                if let EntryCell::ResourceWrite(i, existing_value, dependencies) = &o.get().value {
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
                            let dependencies = std::mem::take(&mut *dependencies.lock());
                            // Received more info, update, but keep the same dependencies.
                            // TODO(BlockSTMv2): Once we support dependency kind, here we could check
                            // that carried over dependencies can be only size & metadata.
                            o.insert(CachePadded::new(new_write_entry(0, value, dependencies)));
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

    fn write_impl(
        versioned_values: &mut VersionedValue<V>,
        txn_idx: TxnIndex,
        incarnation: Incarnation,
        value: ValueWithLayout<V>,
        dependencies: BTreeSet<(TxnIndex, Incarnation)>,
    ) {
        let prev_entry = versioned_values.versioned_map.insert(
            ShiftedTxnIndex::new(txn_idx),
            CachePadded::new(new_write_entry(incarnation, value, dependencies)),
        );

        // Assert that the previous entry for txn_idx, if present, had lower incarnation.
        assert!(prev_entry.map_or(true, |entry| -> bool {
            if let EntryCell::ResourceWrite(i, _, _) = entry.value {
                i < incarnation
            } else {
                true
            }
        }));
    }

    pub fn write(
        &self,
        key: K,
        txn_idx: TxnIndex,
        incarnation: Incarnation,
        data: Arc<V>,
        maybe_layout: Option<Arc<MoveTypeLayout>>,
    ) {
        let mut v = self.values.entry(key).or_default();
        Self::write_impl(
            &mut v,
            txn_idx,
            incarnation,
            ValueWithLayout::Exchanged(data, maybe_layout),
            BTreeSet::new(),
        );
    }

    /// Write a value at a given key (and version) for BlockSTMv2.
    /// Returns invalidated affected read dependencies (dependencies that failed push validation).
    pub fn write_v2<const ONLY_COMPARE_METADATA: bool>(
        &self,
        key: K,
        txn_idx: TxnIndex,
        incarnation: Incarnation,
        data: Arc<V>,
        maybe_layout: Option<Arc<MoveTypeLayout>>,
    ) -> BTreeSet<(TxnIndex, Incarnation)> {
        let mut v = self.values.entry(key).or_default();

        let (affected_dependencies, validation_passed) = v
            .get_affected_read_dependencies::<ONLY_COMPARE_METADATA>(txn_idx, &data, &maybe_layout);

        // If validation passed, keep the dependencies (pass to write_impl), o.w. return them to the caller.
        let (deps_to_pass, deps_to_return) = if validation_passed {
            (affected_dependencies, BTreeSet::new())
        } else {
            (BTreeSet::new(), affected_dependencies)
        };

        Self::write_impl(
            &mut v,
            txn_idx,
            incarnation,
            ValueWithLayout::Exchanged(data, maybe_layout),
            deps_to_pass,
        );

        deps_to_return
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
                BTreeSet::new(),
            )),
        );

        // Changes versioned metadata that was stored.
        prev_entry.map_or(true, |entry| -> bool {
            if let EntryCell::ResourceWrite(_, existing_v, _) = &entry.value {
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
        match v.read(txn_idx + 1, None) {
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

    #[cfg(test)]
    pub(crate) fn get_dependencies(
        &self,
        key: &K,
        shifted_txn_idx: ShiftedTxnIndex,
    ) -> BTreeSet<(TxnIndex, Incarnation)> {
        match &self
            .values
            .get(key)
            .expect("Entry must exist for the given key")
            .versioned_map
            .get(&shifted_txn_idx)
            .expect("Entry must exist for the given txn_idx")
            .value
        {
            EntryCell::ResourceWrite(_, _, deps) => deps.lock().clone(),
            _ => unreachable!("Dependencies can only be recorded for resource writes"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::StorageVersion;
    use aptos_aggregator::{bounded_math::SignedU128, delta_math::DeltaHistory};
    use aptos_types::{
        on_chain_config::CurrentTimeMicroseconds,
        state_store::state_value::{StateValue, StateValueMetadata},
        write_set::{TransactionWrite, WriteOpKind},
    };
    use bytes::Bytes;
    use claims::assert_ok_eq;
    use fail::FailScenario;
    use test_case::test_case;

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct TestValueWithMetadata {
        value: u64,
        metadata: u64,
    }

    impl TestValueWithMetadata {
        fn new(value: u64, metadata: u64) -> Self {
            Self { value, metadata }
        }
    }

    impl TransactionWrite for TestValueWithMetadata {
        fn bytes(&self) -> Option<&Bytes> {
            unimplemented!("Irrelevant for the test")
        }

        fn write_op_kind(&self) -> WriteOpKind {
            unimplemented!("Irrelevant for the test")
        }

        fn from_state_value(_maybe_state_value: Option<StateValue>) -> Self {
            unimplemented!("Irrelevant for the test")
        }

        fn as_state_value(&self) -> Option<StateValue> {
            unimplemented!("Irrelevant for the test")
        }

        fn set_bytes(&mut self, _bytes: Bytes) {
            unimplemented!("Irrelevant for the test")
        }

        fn as_state_value_metadata(&self) -> Option<StateValueMetadata> {
            Some(StateValueMetadata::legacy(
                self.metadata,
                &CurrentTimeMicroseconds {
                    microseconds: self.metadata,
                },
            ))
        }
    }

    fn get_deps_from_entry(
        entry: &Entry<EntryCell<TestValueWithMetadata>>,
    ) -> BTreeSet<(TxnIndex, Incarnation)> {
        if let EntryCell::ResourceWrite(_, _, deps) = &entry.value {
            deps.lock().clone()
        } else {
            unreachable!()
        }
    }

    #[test_case(1, BTreeSet::from([(2, 0), (3, 0), (3, 1), (7, 1)]), true; "deps > 1 from idx 0 write, pass validation")]
    #[test_case(7, BTreeSet::from([(8, 1), (9, 0), (10, 0), (10, 2)]), false; "deps > 7 from idx 7 write, fail validation")]
    #[test_case(5, BTreeSet::from([(7, 1)]), true; "deps > 5 from write at idx 0, pass validation")]
    #[test_case(0, BTreeSet::from([(1, 0), (2, 0), (3, 0), (3, 1), (7, 1)]), true; "all deps > 0 from idx 0 write, pass validation")]
    #[test_case(9, BTreeSet::from([(10, 0), (10, 2)]), false; "deps > 9 from write at idx 7, fail validation")]
    #[test_case(12, BTreeSet::from([]), true; "entries >= idx 12 - no deps, pass validation")]
    #[test_case(7, BTreeSet::from([(8, 1), (9, 0), (10, 0), (10, 2)]), false; "all deps from write at idx 7, fail validation")]
    fn test_get_affected_read_dependencies(
        idx: TxnIndex,
        expected_deps: BTreeSet<(TxnIndex, Incarnation)>,
        expected_validation_result: bool,
    ) {
        let mut v = VersionedValue::<TestValueWithMetadata>::default();

        // Setup: Create some writes with dependencies.
        let deps_idx0 = BTreeSet::from([(1, 0), (2, 0)]);
        let deps_idx7 = BTreeSet::from([(8, 1), (9, 0), (10, 2)]);

        v.versioned_map.insert(
            ShiftedTxnIndex::new(0),
            CachePadded::new(new_write_entry(
                0,
                ValueWithLayout::Exchanged(Arc::new(TestValueWithMetadata::new(10, 100)), None),
                deps_idx0,
            )),
        );
        v.versioned_map.insert(
            ShiftedTxnIndex::new(7),
            CachePadded::new(new_write_entry(
                0,
                ValueWithLayout::Exchanged(Arc::new(TestValueWithMetadata::new(20, 200)), None),
                deps_idx7,
            )),
        );

        // Add some dependencies via read() calls.
        let _ = v.read(3, Some(0)); // This adds (3, 0) to latest write <= 3 (write at idx 0).
        let _ = v.read(3, Some(1)); // Add another incarnation of txn 3.
        let _ = v.read(7, Some(1)); // This adds (7, 1) to write at idx 0.
        let _ = v.read(8, Some(1)); // This adds (8, 1) to write at idx 7 (duplicate with existing).
        let _ = v.read(10, Some(0)); // Add lower incarnation after we'll add a higher one.
        let _ = v.read(10, Some(2)); // Add higher incarnation first.

        // Get pre-call state of dependencies.
        let mut recorded_deps_idx0 =
            get_deps_from_entry(v.versioned_map.get(&ShiftedTxnIndex::new(0)).unwrap());
        let mut recorded_deps_idx7 =
            get_deps_from_entry(v.versioned_map.get(&ShiftedTxnIndex::new(7)).unwrap());

        // Get the actual dependencies and verify they match expected.
        let (affected_deps, validation_passed) = v.get_affected_read_dependencies::<false>(
            idx,
            &Arc::new(TestValueWithMetadata::new(10, 100)),
            &None,
        );
        assert_eq!(
            affected_deps, expected_deps,
            "Dependencies above idx don't match expected."
        );
        assert_eq!(
            validation_passed, expected_validation_result,
            "Validation result doesn't match expected."
        );

        // Verify that the remaining dependencies in entries match what we expect.
        if idx < 7 {
            let (remaining_deps, _) = v.get_affected_read_dependencies::<false>(
                6,
                &Arc::new(TestValueWithMetadata::new(10, 100)),
                &None,
            );
            assert!(remaining_deps.is_empty());
            recorded_deps_idx0.retain(|(txn_idx, _)| *txn_idx <= idx);
        } else {
            recorded_deps_idx7.retain(|(txn_idx, _)| *txn_idx <= idx);
        }

        let final_deps_idx0 =
            get_deps_from_entry(v.versioned_map.get(&ShiftedTxnIndex::new(0)).unwrap());
        assert_eq!(
            final_deps_idx0, recorded_deps_idx0,
            "Dependencies in write at idx 0 don't match expected."
        );

        let final_deps_idx7 =
            get_deps_from_entry(v.versioned_map.get(&ShiftedTxnIndex::new(7)).unwrap());
        assert_eq!(
            final_deps_idx7, recorded_deps_idx7,
            "Dependencies in write at idx 7 don't match expected."
        );
    }

    #[test]
    fn test_get_affected_read_dependencies_delta_only() {
        let mut v = VersionedValue::<TestValueWithMetadata>::default();
        v.versioned_map.insert(
            ShiftedTxnIndex::new(0),
            CachePadded::new(new_delta_entry(DeltaOp::new(
                SignedU128::Positive(10),
                1000,
                DeltaHistory {
                    max_achieved_positive_delta: 10,
                    min_achieved_negative_delta: 0,
                    min_overflow_positive_delta: None,
                    max_underflow_negative_delta: None,
                },
            ))),
        );
        v.versioned_map.insert(
            ShiftedTxnIndex::new(5),
            CachePadded::new(new_delta_entry(DeltaOp::new(
                SignedU128::Positive(20),
                1000,
                DeltaHistory {
                    max_achieved_positive_delta: 20,
                    min_achieved_negative_delta: 0,
                    min_overflow_positive_delta: None,
                    max_underflow_negative_delta: None,
                },
            ))),
        );
        let (deps, _) = v.get_affected_read_dependencies::<false>(
            3,
            &Arc::new(TestValueWithMetadata::new(10, 100)),
            &None,
        );
        assert_eq!(deps, BTreeSet::new());
    }

    #[test]
    fn test_value_metadata_layout_comparison() {
        macro_rules! test_metadata_layout_case {
            ($only_compare_metadata:expr) => {
                // Test all combinations of value/metadata/layout comparison parameters
                for same_value in [true, false] {
                    for same_metadata in [true, false] {
                        for same_layout in [true, false] {
                            let mut v = VersionedValue::<TestValueWithMetadata>::default();

                            // Setup: Create a write with value 10, metadata 100 and one dependency
                            let deps = BTreeSet::from([(1, 0)]);
                            let layout = if same_layout { None } else { Some(Arc::new(MoveTypeLayout::Bool)) };
                            v.versioned_map.insert(
                                ShiftedTxnIndex::new(0),
                                CachePadded::new(new_write_entry(0, ValueWithLayout::Exchanged(Arc::new(TestValueWithMetadata::new(10, 100)), layout), deps)),
                            );

                            // Create test value based on parameters
                            let test_value = TestValueWithMetadata::new(
                                if same_value { 10 } else { 20 },
                                if same_metadata { 100 } else { 200 }
                            );

                            // Compute expected validation result
                            let expected_validation = if $only_compare_metadata {
                                same_metadata
                            } else {
                                same_value && same_metadata && same_layout
                            };

                            // Test get_affected_read_dependencies
                            let (deps, validation_passed) = v.get_affected_read_dependencies::<{ $only_compare_metadata }>(
                                0,
                                &Arc::new(test_value.clone()),
                                &None,
                            );

                            // Verify results
                            assert_eq!(
                                validation_passed,
                                expected_validation,
                                "Validation failed for same_value={}, same_metadata={}, only_compare_metadata={}, same_layout={}",
                                same_value, same_metadata, $only_compare_metadata, same_layout
                            );
                            assert_eq!(
                                deps,
                                BTreeSet::from([(1, 0)]),
                                "Dependencies don't match for same_value={}, same_metadata={}, only_compare_metadata={}, same_layout={}",
                                same_value, same_metadata, $only_compare_metadata, same_layout
                            );

                            // Test handle_removed_dependencies
                            let remaining_deps = v.handle_removed_dependencies::<{ $only_compare_metadata }>(
                                1,
                                BTreeSet::from([(2, 0)]),
                                &Arc::new(test_value),
                                &None,
                            );

                            if expected_validation {
                                assert!(remaining_deps.is_empty());
                                // Verify that (2,0) is recorded in 0-th entry
                                assert_eq!(get_deps_from_entry(v.versioned_map.get(&ShiftedTxnIndex::new(0)).unwrap()), BTreeSet::from([(2, 0)]));
                            } else {
                                assert_eq!(remaining_deps, BTreeSet::from([(2, 0)]));
                                // Verify that dependencies are empty in 0-th entry
                                assert_eq!(get_deps_from_entry(v.versioned_map.get(&ShiftedTxnIndex::new(0)).unwrap()).len(), 0);
                            }
                        }
                    }
                }
            };
        }

        // Test both cases
        test_metadata_layout_case!(true);
        test_metadata_layout_case!(false);
    }

    #[test]
    fn test_raw_from_storage_validation() {
        macro_rules! test_raw_from_storage_case {
            ($only_compare_metadata:expr) => {
                let mut v = VersionedValue::<TestValueWithMetadata>::default();

                // Setup: Create a write with RawFromStorage value and one dependency
                let deps = BTreeSet::from([(1, 0)]);
                v.versioned_map.insert(
                    ShiftedTxnIndex::new(0),
                    CachePadded::new(new_write_entry(0, ValueWithLayout::RawFromStorage(Arc::new(TestValueWithMetadata::new(10, 100))), deps)),
                );

                // Test get_affected_read_dependencies with Exchanged value
                let (deps, validation_passed) = v.get_affected_read_dependencies::<{ $only_compare_metadata }>(
                    0,
                    &Arc::new(TestValueWithMetadata::new(10, 100)),
                    &None,
                );

                // Verify results - validation should fail even with same value and metadata
                assert!(!validation_passed, "Validation should fail when comparing with RawFromStorage (only_compare_metadata={})", $only_compare_metadata);
                assert_eq!(deps, BTreeSet::from([(1, 0)]), "Dependencies should be returned even when validation fails (only_compare_metadata={})", $only_compare_metadata);

                // Test handle_removed_dependencies
                let remaining_deps = v.handle_removed_dependencies::<{ $only_compare_metadata }>(
                    1,
                    BTreeSet::from([(2, 0)]),
                    &Arc::new(TestValueWithMetadata::new(10, 100)),
                    &None,
                );

                // Verify that dependencies are not passed and returned
                assert_eq!(
                    remaining_deps,
                    BTreeSet::from([(2, 0)]),
                    "Dependencies should be returned when validation fails (only_compare_metadata={})",
                    $only_compare_metadata
                );
                assert_eq!(
                    get_deps_from_entry(v.versioned_map.get(&ShiftedTxnIndex::new(0)).unwrap()).len(),
                    0,
                    "Dependencies should not be passed to next entry when validation fails (only_compare_metadata={})",
                    $only_compare_metadata
                );
            };
        }

        // Test both cases
        test_raw_from_storage_case!(true);
        test_raw_from_storage_case!(false);
    }

    #[test]
    #[should_panic(
        expected = "Entry at txn_idx must be removed before calling handle_removed_dependencies"
    )]
    fn test_handle_removed_dependencies_panic() {
        let mut v = VersionedValue::<TestValueWithMetadata>::default();

        // Setup: Create a write entry
        v.versioned_map.insert(
            ShiftedTxnIndex::new(0),
            CachePadded::new(new_write_entry(
                0,
                ValueWithLayout::Exchanged(Arc::new(TestValueWithMetadata::new(10, 100)), None),
                BTreeSet::new(),
            )),
        );

        v.handle_removed_dependencies::<false>(
            0,
            BTreeSet::from([(2, 0)]),
            &Arc::new(TestValueWithMetadata::new(10, 100)),
            &None,
        );
    }

    #[test]
    fn test_remove_v2_storage_version() {
        let mut v = VersionedValue::<TestValueWithMetadata>::default();

        // Setup: Create an irrelevant (higher index) write entry.
        v.versioned_map.insert(
            ShiftedTxnIndex::new(3),
            CachePadded::new(new_write_entry(
                0,
                ValueWithLayout::Exchanged(Arc::new(TestValueWithMetadata::new(10, 100)), None),
                BTreeSet::new(),
            )),
        );

        let dependencies = BTreeSet::from([(2, 0)]);
        assert_eq!(
            dependencies,
            v.handle_removed_dependencies::<true>(
                1,
                dependencies.clone(),
                &Arc::new(TestValueWithMetadata::new(10, 100)),
                &None,
            )
        );
        assert_eq!(
            dependencies,
            v.handle_removed_dependencies::<false>(
                1,
                dependencies.clone(),
                &Arc::new(TestValueWithMetadata::new(10, 100)),
                &None,
            )
        );

        // Now insert a new entry at index 0 that will retain the dependencies.
        v.versioned_map.insert(
            ShiftedTxnIndex::new(0),
            CachePadded::new(new_write_entry(
                0,
                ValueWithLayout::Exchanged(Arc::new(TestValueWithMetadata::new(10, 100)), None),
                BTreeSet::new(),
            )),
        );
        assert_eq!(
            v.handle_removed_dependencies::<true>(
                1,
                dependencies.clone(),
                &Arc::new(TestValueWithMetadata::new(10, 100)),
                &None,
            )
            .len(),
            0
        );
        assert_eq!(
            get_deps_from_entry(v.versioned_map.get(&ShiftedTxnIndex::new(0)).unwrap()),
            dependencies
        );

        let dependencies = BTreeSet::from([(2, 1)]);
        assert_eq!(
            v.handle_removed_dependencies::<false>(
                1,
                dependencies.clone(),
                &Arc::new(TestValueWithMetadata::new(10, 100)),
                &None,
            )
            .len(),
            0
        );
        assert_eq!(
            get_deps_from_entry(v.versioned_map.get(&ShiftedTxnIndex::new(0)).unwrap()),
            BTreeSet::from([(2, 0), (2, 1)])
        );
    }

    fn check_versioned_data_deps(
        versioned_data: &VersionedData<(), TestValueWithMetadata>,
        shifted_txn_idx: ShiftedTxnIndex,
        expected_deps: BTreeSet<(TxnIndex, Incarnation)>,
    ) {
        assert_eq!(
            get_deps_from_entry(
                versioned_data
                    .values
                    .get(&())
                    .unwrap()
                    .versioned_map
                    .get(&shifted_txn_idx)
                    .unwrap()
            ),
            expected_deps
        );
    }

    #[test]
    fn test_base_value_dep_transfer() {
        let versioned_data = VersionedData::<(), TestValueWithMetadata>::empty();

        let scenario = FailScenario::setup();
        assert!(fail::has_failpoints());
        // Failpoint returns 10 as bytes length.
        fail::cfg("value_with_layout_bytes_len", "return").unwrap();
        assert!(!fail::list().is_empty());

        versioned_data.set_base_value(
            (),
            ValueWithLayout::Exchanged(Arc::new(TestValueWithMetadata::new(10, 100)), None),
        );
        assert_eq!(versioned_data.total_base_value_size(), 10);
        scenario.teardown();

        assert_ok_eq!(
            versioned_data.fetch_data_v2(&(), 5, 1),
            MVDataOutput::Versioned(
                Err(StorageVersion),
                ValueWithLayout::Exchanged(Arc::new(TestValueWithMetadata::new(10, 100)), None),
            ),
        );
        check_versioned_data_deps(
            &versioned_data,
            ShiftedTxnIndex::zero_idx(),
            BTreeSet::from([(5, 1)]),
        );

        versioned_data.write_v2::<true>(
            (),
            1,
            1,
            Arc::new(TestValueWithMetadata::new(10, 100)),
            None,
        );
        check_versioned_data_deps(
            &versioned_data,
            ShiftedTxnIndex::zero_idx(),
            BTreeSet::new(),
        );
        check_versioned_data_deps(
            &versioned_data,
            ShiftedTxnIndex::new(1),
            BTreeSet::from([(5, 1)]),
        );

        versioned_data.write_v2::<true>(
            (),
            3,
            0,
            Arc::new(TestValueWithMetadata::new(10, 100)),
            None,
        );
        check_versioned_data_deps(
            &versioned_data,
            ShiftedTxnIndex::zero_idx(),
            BTreeSet::new(),
        );
        check_versioned_data_deps(&versioned_data, ShiftedTxnIndex::new(1), BTreeSet::new());
        check_versioned_data_deps(
            &versioned_data,
            ShiftedTxnIndex::new(3),
            BTreeSet::from([(5, 1)]),
        );

        assert_ok_eq!(
            versioned_data.fetch_data_v2(&(), 2, 0),
            MVDataOutput::Versioned(
                Ok((1, 1)),
                ValueWithLayout::Exchanged(Arc::new(TestValueWithMetadata::new(10, 100)), None),
            ),
        );
        assert_eq!(versioned_data.remove_v2::<_, false>(&(), 3).len(), 0);
        assert_eq!(versioned_data.remove_v2::<_, true>(&(), 1).len(), 0);
        check_versioned_data_deps(
            &versioned_data,
            ShiftedTxnIndex::zero_idx(),
            BTreeSet::from([(2, 0), (5, 1)]),
        );
    }

    #[test]
    #[should_panic(expected = "Entry for key / idx must exist to be deleted")]
    fn test_remove_v2_panic_no_entry() {
        let versioned_data = VersionedData::<(), TestValueWithMetadata>::empty();

        // Add an entry at index 0
        versioned_data.write(
            (),
            0,
            0,
            Arc::new(TestValueWithMetadata::new(10, 100)),
            None,
        );

        // Try to remove a non-existent entry at index 1
        versioned_data.remove_v2::<_, false>(&(), 1);
    }
}
