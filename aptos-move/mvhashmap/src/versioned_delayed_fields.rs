// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::types::{AtomicTxnIndex, MVDelayedFieldsError, TxnIndex};
use aptos_aggregator::{
    delayed_change::{ApplyBase, DelayedApplyEntry, DelayedEntry},
    types::{DelayedFieldValue, ReadPosition},
};
use aptos_types::error::{code_invariant_error, PanicError, PanicOr};
use claims::assert_matches;
use crossbeam::utils::CachePadded;
use dashmap::DashMap;
use std::{
    collections::btree_map::{BTreeMap, Entry},
    fmt::Debug,
    hash::Hash,
    iter::DoubleEndedIterator,
    ops::Deref,
    sync::atomic::{AtomicU64, Ordering},
};

pub enum CommitError {
    CodeInvariantError(String),
    ReExecutionNeeded(String),
}

// When an AggregatorEntry (see below) is transformed to an Estimate, internally we store
// a potential bypass (based on the previously stored entry), which may allow a read
// operation to not wait for the corresponding dependency.
#[derive(Clone, Debug, PartialEq)]
enum EstimatedEntry<K: Clone> {
    NoBypass,
    // If applicable, can bypass the Estimate by considering a apply change instead.
    Bypass(DelayedApplyEntry<K>),
}

// There is no explicit deletion as it will be impossible to resolve the ID of a deleted
// aggregator (as the ID will not be contained in a resource anymore). The write on the
// previously holding resource and Block-STM read validation ensures correctness.
#[derive(Debug)]
enum VersionEntry<K: Clone> {
    // If the value is determined by a delta applied to a value read during the execution
    // of the same transaction, the delta may also be kept in the entry. This is useful
    // if speculative execution aborts and the entry is marked as estimate, as the delta
    // may be used to avoid waiting on the Estimate entry. More in comments below.
    Value(DelayedFieldValue, Option<DelayedApplyEntry<K>>),
    // Applies the change on top of the previous entry - either for the same ID corresponding
    // to this change, or for the apply_base_id given by the change, at a specific point defined
    // by the it's ApplyBase.
    Apply(DelayedApplyEntry<K>),
    // Marks the entry as an estimate, indicating that the next incarnation of the
    // transaction is estimated to populate the entry. May contain a bypass internally
    // (allowing a read operation to avoid waiting for the corresponding dependency),
    // encapsulated by the EstimatedEntry.
    Estimate(EstimatedEntry<K>),
}

// A VersionedValue internally contains a BTreeMap from indices of transactions
// that update a given aggregator, alongside the corresponding entries.
#[derive(Debug)]
struct VersionedValue<K: Clone> {
    versioned_map: BTreeMap<TxnIndex, Box<CachePadded<VersionEntry<K>>>>,

    // The value of the given aggregator prior to the block execution. None implies that
    // the aggregator did not exist prior to the block.
    base_value: Option<DelayedFieldValue>,

    // If true, the reads can proceed by using deltas in Estimate entries, if present.
    // The value is optimistically initialized to true, but changed to false when it is
    // observed that a later incarnation changed the value of a delta at the same entry.
    read_estimate_deltas: bool,
}

#[derive(Debug, PartialEq)]
enum VersionedRead<K: Clone> {
    Value(DelayedFieldValue),
    // The transaction index records the index at which the Snapshot was encountered.
    // This is required for the caller to resolve the value of the aggregator (with the
    // recorded id) from which the snapshot was created at the correct version (index).
    DependentApply(K, TxnIndex, DelayedApplyEntry<K>),
}

fn variant_eq<T>(a: &T, b: &T) -> bool {
    std::mem::discriminant(a) == std::mem::discriminant(b)
}

impl<K: Copy + Clone + Debug + Eq> VersionedValue<K> {
    // VersionedValue should only be created when base value of the corresponding aggregator
    // is known & provided to the constructor.
    fn new(base_value: Option<DelayedFieldValue>) -> Self {
        Self {
            versioned_map: BTreeMap::new(),
            base_value,
            // Enable the optimization to not wait on dependencies during reading by default.
            read_estimate_deltas: true,
        }
    }

    fn mark_estimate(&mut self, txn_idx: TxnIndex) {
        use EstimatedEntry::*;
        use VersionEntry::*;

        match self.versioned_map.entry(txn_idx) {
            Entry::Occupied(mut o) => {
                let bypass = match o.get().as_ref().deref() {
                    Value(_, maybe_apply) => maybe_apply.clone().map_or(NoBypass, Bypass),
                    Apply(apply) => Bypass(apply.clone()),
                    Estimate(_) => unreachable!("Entry already marked estimate"),
                };

                o.insert(Box::new(CachePadded::new(Estimate(bypass))));
            },
            Entry::Vacant(_) => unreachable!("Versioned entry must exist when marking as estimate"),
        };
    }

    fn remove_v2(&mut self, txn_idx: TxnIndex) {
        self.versioned_map.remove(&txn_idx);
        // TODO(BlockSTMv2): deal w. V2 & estimates and potentially bring back the check
        // that removed entry must be an estimate (but with PanicError).
    }

    fn remove(&mut self, txn_idx: TxnIndex) {
        let deleted_entry = self.versioned_map.remove(&txn_idx);
        // Entries should only be deleted if the transaction that produced them is
        // aborted and re-executed, but abort must have marked the entry as an Estimate.
        assert_matches!(
            deleted_entry
                .expect("Entry must exist to be removed")
                .as_ref()
                .deref(),
            VersionEntry::Estimate(_),
            "Removed entry must be an Estimate",
        );
        // Incarnation changed output behavior, disable reading through estimates optimization.
        self.read_estimate_deltas = false;
    }

    // Insert value computed from speculative transaction execution,
    // potentially overwritting a previous entry.
    fn insert_speculative_value(
        &mut self,
        txn_idx: TxnIndex,
        entry: VersionEntry<K>,
    ) -> Result<(), PanicError> {
        use EstimatedEntry::*;
        use VersionEntry::*;

        assert!(
            !matches!(entry, Estimate(_)),
            "Inserting Estimate is not allowed - must call mark_estimate"
        );

        match self.versioned_map.entry(txn_idx) {
            Entry::Occupied(mut o) => {
                if !match (o.get().as_ref().deref(), &entry) {
                    // These are the cases where the transaction behavior with respect to the
                    // aggregator may change (based on the information recorded in the Estimate).
                    (Estimate(Bypass(apply_l)), Apply(apply_r) | Value(_, Some(apply_r))) => {
                        if variant_eq(apply_l, apply_r) {
                            *apply_l == *apply_r
                        } else {
                            return Err(code_invariant_error(format!(
                                "Storing {:?} for aggregator ID that previously had a different type of entry - {:?}",
                                apply_r, apply_l,
                            )));
                        }
                    },
                    // There was a value without fallback delta bypass before and still.
                    (Estimate(NoBypass), Value(_, None)) => true,
                    // Bypass stored in the estimate does not match the new entry.
                    (Estimate(_), _) => false,

                    (_cur, _new) => {
                        // TODO(BlockSTMv2): V2 currently does not mark estimate.
                        // For V1, used to return Err(code_invariant_error(format!(
                        //    "Replaced entry must be an Estimate, {:?} to {:?}",
                        //    cur, new,
                        //)))
                        true
                    },
                } {
                    // TODO[agg_v2](optimize): See if we want to invalidate, when we change read_estimate_deltas
                    self.read_estimate_deltas = false;
                }
                o.insert(Box::new(CachePadded::new(entry)));
            },
            Entry::Vacant(v) => {
                v.insert(Box::new(CachePadded::new(entry)));
            },
        }
        Ok(())
    }

    fn insert_final_value(&mut self, txn_idx: TxnIndex, value: DelayedFieldValue) {
        use VersionEntry::*;

        match self.versioned_map.entry(txn_idx) {
            Entry::Occupied(mut o) => {
                match o.get().as_ref().deref() {
                    Value(v, _) => assert_eq!(v, &value),
                    Apply(_) => (),
                    _ => unreachable!("When inserting final value, it needs to be either be Apply or have the same value"),
                };
                o.insert(Box::new(CachePadded::new(VersionEntry::Value(value, None))));
            },
            Entry::Vacant(_) => unreachable!("When inserting final value, it needs to be present"),
        };
    }

    // Given a transaction index which should be committed next, returns the latest value
    // below this version, or if no such value exists, then the delayed field must have been
    // created in the same block. In this case predict the value in the first (lowest) entry,
    // or an error if such an entry cannot be found (must be due to speculation). The lowest
    // entry is picked without regards to the indices, as it's for optimistic prediction
    // purposes only (better to have some value than error).
    fn read_latest_predicted_value(
        &self,
        next_idx_to_commit: TxnIndex,
    ) -> Result<DelayedFieldValue, MVDelayedFieldsError> {
        use VersionEntry::*;

        self.versioned_map
            .range(0..next_idx_to_commit)
            .next_back()
            .map_or_else(
                || match &self.base_value {
                    Some(value) => Ok(value.clone()),
                    None => match self.versioned_map.first_key_value() {
                        Some((_, entry)) => match entry.as_ref().deref() {
                            Value(v, _) => Ok(v.clone()),
                            Apply(_) | Estimate(_) => Err(MVDelayedFieldsError::NotFound),
                        },
                        None => Err(MVDelayedFieldsError::NotFound),
                    },
                },
                |(_, entry)| match entry.as_ref().deref() {
                    Value(v, _) => Ok(v.clone()),
                    Apply(_) => {
                        unreachable!("Apply entries may not exist for committed txn indices")
                    },
                    Estimate(_) => unreachable!("Committed entry may not be an Estimate"),
                },
            )
    }

    // For an Aggregator, traverse down from txn_idx and accumulate encountered deltas
    // until resolving it to a value or return an error.
    // Errors of not finding a value to resolve to take precedence over a DeltaApplicationError.
    fn apply_aggregator_change_suffix(
        &self,
        iter: &mut dyn DoubleEndedIterator<Item = (&TxnIndex, &Box<CachePadded<VersionEntry<K>>>)>,
        suffix: &DelayedApplyEntry<K>,
    ) -> Result<VersionedRead<K>, PanicOr<MVDelayedFieldsError>> {
        use DelayedApplyEntry::*;
        use EstimatedEntry::*;
        use VersionEntry::*;

        let mut accumulator = if let AggregatorDelta { delta } = suffix {
            *delta
        } else {
            unreachable!("Only AggregatorDelta accepted in apply_aggregator_change_suffix (i.e. has no apply_base_id)")
        };

        while let Some((idx, entry)) = iter.next_back() {
            let delta = match (entry.as_ref().deref(), self.read_estimate_deltas) {
                (Value(DelayedFieldValue::Aggregator(v), _), _) => {
                    // Apply accumulated delta to resolve the aggregator value.
                    return accumulator
                        .apply_to(*v)
                        .map_err(MVDelayedFieldsError::from_panic_or)
                        .map(DelayedFieldValue::Aggregator)
                        .map(VersionedRead::Value);
                },
                (Value(_, _), _) => {
                    unreachable!("Value not DelayedFieldValue::Aggregator for Aggregator")
                },
                (Apply(AggregatorDelta { delta }), _)
                | (Estimate(Bypass(AggregatorDelta { delta })), true) => *delta,
                (Estimate(NoBypass), _) | (Estimate(_), false) => {
                    // We must wait on Estimates, or a bypass isn't available.
                    return Err(PanicOr::Or(MVDelayedFieldsError::Dependency(*idx)));
                },
                (Apply(_), _) | (Estimate(Bypass(_)), true) => {
                    unreachable!("Apply change type not AggregatorDelta for aggregator")
                },
            };

            // Read hit a delta during traversing the block and aggregating other deltas. We merge the
            // two deltas together. If there is an error, we return appropriate error
            // (DeltaApplicationError or PanicOr::CodeInvariantError
            // (there is no determinism concern as DeltaApplicationError may not occur in committed output).
            accumulator
                .merge_with_previous_delta(delta)
                .map_err(MVDelayedFieldsError::from_panic_or)?;
        }

        // Finally, resolve if needed with the base value.
        self.base_value
            .as_ref()
            .ok_or(PanicOr::Or(MVDelayedFieldsError::NotFound))
            .and_then(|base_value| match base_value {
                DelayedFieldValue::Aggregator(v) => accumulator
                    .apply_to(*v)
                    .map_err(MVDelayedFieldsError::from_panic_or)
                    .map(DelayedFieldValue::Aggregator)
                    .map(VersionedRead::Value),
                _ => Err(PanicOr::from(code_invariant_error(
                    "Found non-DelayedFieldValue::Aggregator base value for aggregator with delta",
                ))),
            })
    }

    // Reads a given aggregator value at a given version (transaction index) and produces
    // a ReadResult if successful, which is either a u128 value, or a snapshot specifying
    // a different aggregator (with ID) at a given version and a delta to apply on top.
    fn read(&self, txn_idx: TxnIndex) -> Result<VersionedRead<K>, PanicOr<MVDelayedFieldsError>> {
        use EstimatedEntry::*;
        use MVDelayedFieldsError::*;
        use VersionEntry::*;

        let mut iter = self.versioned_map.range(0..txn_idx);

        iter.next_back().map_or_else(
            // No entries in versioned map, use base value.
            || {
                self.base_value
                    .clone()
                    .ok_or(PanicOr::Or(NotFound))
                    .map(VersionedRead::Value)
            },
            // Consider the latest entry below the provided version.
            |(idx, entry)| match (entry.as_ref().deref(), self.read_estimate_deltas) {
                (Value(v, _), _) => Ok(VersionedRead::Value(v.clone())),
                (Apply(apply), _) | (Estimate(Bypass(apply)), true) => {
                    apply.get_apply_base_id_option().map_or_else(
                        || self.apply_aggregator_change_suffix(&mut iter, apply),
                        |apply_base| {
                            let (base_id, end_index) = match apply_base {
                                ApplyBase::Previous(id) => (id, *idx),
                                ApplyBase::Current(id) => (id, *idx + 1),
                            };

                            Ok(VersionedRead::DependentApply(
                                base_id,
                                end_index,
                                apply.clone(),
                            ))
                        },
                    )
                },
                (Estimate(NoBypass), _) | (Estimate(_), false) => {
                    Err(PanicOr::Or(Dependency(*idx)))
                },
            },
        )
    }
}

pub trait TVersionedDelayedFieldView<K> {
    fn read(
        &self,
        id: &K,
        txn_idx: TxnIndex,
    ) -> Result<DelayedFieldValue, PanicOr<MVDelayedFieldsError>>;

    /// Returns the committed value from largest transaction index that is smaller than the
    /// given current_txn_idx (read_position defined whether inclusively or exclusively from
    /// the current transaction itself). If such a value does not exist, the value might
    /// be created in the current block, and the value from the first (lowest) entry is taken
    /// as the prediction.
    fn read_latest_predicted_value(
        &self,
        id: &K,
        current_txn_idx: TxnIndex,
        read_position: ReadPosition,
    ) -> Result<DelayedFieldValue, MVDelayedFieldsError>;
}

/// Maps each ID (access path) to an internal VersionedValue, managing versioned updates to the
/// specified delayed field (which handles both Aggregator, and AggregatorSnapshot).
///
/// There are some invariants that the caller must maintain when using VersionedDelayedFeilds:
/// -) 'set_base_value' or 'create_aggregator' must be completed for each ID prior to calling
/// other methods (reading, insert, remove or marking as estimate) for the ID.
/// -) When a transaction is committed, all transactions with lower indices are also considered
/// committed. Before an index is committed, all of its deltas and snapshots must be converted
/// to values by the caller (by recoding the final materialized values).
/// -) When a transaction aborts, its entries must be converted to estimates until the
/// transaction can re-execute with the next incarnation. When the next incarnation finishes
/// and records new entries, all remaining Estimate entries must be removed.
///
/// Another invariant that must be maintained by the caller is that the same delayed field ID
/// throughout the course of the lifetime of the data-structure may not contain a delta and
/// a snapshot - even at different times. In particular, this precludes re-using the same ID
/// between Aggregator and AggregatorSnapshot. It is easy to provide this property from the
/// caller side, even if IDs are re-used (say among incarnations) by e.g. assigning odd and
/// even ids to Aggregators and AggregatorSnapshots, and it allows asserting the uses strictly.
pub struct VersionedDelayedFields<K: Clone> {
    values: DashMap<K, VersionedValue<K>>,

    /// No deltas are allowed below next_idx_to_commit version, as all deltas (and snapshots)
    /// must be materialized and converted to Values during commit.
    next_idx_to_commit: CachePadded<AtomicTxnIndex>,

    total_base_value_size: CachePadded<AtomicU64>,
}

impl<K: Eq + Hash + Clone + Debug + Copy> VersionedDelayedFields<K> {
    /// Part of the big multi-versioned data-structure, which creates different types of
    /// versioned maps (including this one for delayed fields), and delegates access. Hence,
    /// new should only be used from the crate.
    pub(crate) fn empty() -> Self {
        Self {
            values: DashMap::new(),
            next_idx_to_commit: CachePadded::new(AtomicTxnIndex::new(0)),
            total_base_value_size: CachePadded::new(AtomicU64::new(0)),
        }
    }

    pub(crate) fn num_keys(&self) -> usize {
        self.values.len()
    }

    pub(crate) fn total_base_value_size(&self) -> u64 {
        self.total_base_value_size.load(Ordering::Relaxed)
    }

    /// Must be called when an delayed field from storage is resolved, with ID replacing the
    /// base value. This ensures that VersionedValue exists for the delayed field before any
    /// other uses (adding deltas, etc).
    ///
    /// Setting base value multiple times, even concurrently, is okay for the same ID,
    /// because the corresponding value prior to the block is fixed.
    pub fn set_base_value(&self, id: K, base_value: DelayedFieldValue) {
        self.values.entry(id).or_insert_with(|| {
            self.total_base_value_size.fetch_add(
                base_value.get_approximate_memory_size() as u64,
                Ordering::Relaxed,
            );
            VersionedValue::new(Some(base_value))
        });
    }

    /// Must be called when an delayed field creation with a given ID and initial value is
    /// observed in the outputs of txn_idx.
    pub fn initialize_delayed_field(
        &self,
        id: K,
        txn_idx: TxnIndex,
        value: DelayedFieldValue,
    ) -> Result<(), PanicError> {
        let mut created = VersionedValue::new(None);
        created.insert_speculative_value(txn_idx, VersionEntry::Value(value, None))?;

        if self.values.insert(id, created).is_some() {
            Err(code_invariant_error(
                "VersionedValue when initializing delayed field may not already exist for same id",
            ))
        } else {
            Ok(())
        }
    }

    /// Must be called when a snapshot (delta or derived) creation with a given ID
    /// and initial apply is observed.
    /// This should be only called when apply applies on top of different ID.
    pub fn initialize_dependent_delayed_field(
        &self,
        id: K,
        txn_idx: TxnIndex,
        apply: DelayedApplyEntry<K>,
    ) -> Result<(), PanicError> {
        let mut created = VersionedValue::new(None);
        created.insert_speculative_value(txn_idx, VersionEntry::Apply(apply))?;

        if self.values.insert(id, created).is_some() {
            Err(code_invariant_error("VersionedValue when initializing dependent delayed field may not already exist for same id"))
        } else {
            Ok(())
        }
    }

    pub fn record_change(
        &self,
        id: K,
        txn_idx: TxnIndex,
        change: DelayedEntry<K>,
    ) -> Result<(), PanicOr<MVDelayedFieldsError>> {
        match change {
            DelayedEntry::Create(value) => self.initialize_delayed_field(id, txn_idx, value)?,
            DelayedEntry::Apply(apply) => match &apply {
                DelayedApplyEntry::AggregatorDelta { .. } => self
                    .values
                    .get_mut(&id)
                    .ok_or(PanicOr::Or(MVDelayedFieldsError::NotFound))?
                    .insert_speculative_value(txn_idx, VersionEntry::Apply(apply))?,
                DelayedApplyEntry::SnapshotDelta { .. }
                | DelayedApplyEntry::SnapshotDerived { .. } => {
                    self.initialize_dependent_delayed_field(id, txn_idx, apply)?
                },
            },
        };
        Ok(())
    }

    /// The caller must maintain the invariant that prior to calling the methods below w.
    /// a particular DelayedFieldID, an invocation of either initialize_* (for newly created
    /// delayed fields), or set_base_value (for existing delayed fields) must have been completed.
    pub fn mark_estimate(&self, id: &K, txn_idx: TxnIndex) {
        self.values
            .get_mut(id)
            .expect("VersionedValue for an (resolved) ID must already exist")
            .mark_estimate(txn_idx);
    }

    pub fn remove(&self, id: &K, txn_idx: TxnIndex) {
        self.values
            .get_mut(id)
            .expect("VersionedValue for an (resolved) ID must already exist")
            .remove(txn_idx);
    }

    pub fn remove_v2(&self, id: &K, txn_idx: TxnIndex) {
        self.values
            .get_mut(id)
            .expect("VersionedValue for an (resolved) ID must already exist")
            .remove_v2(txn_idx);
    }

    /// Moves the commit index, and computes exact values for delayed fields having
    /// apply changes in this transaction. After it finishes, all versions at or
    /// before given idx are in Value state.
    ///
    /// Must be called for each transaction index, in order.
    pub fn try_commit(&self, idx_to_commit: TxnIndex, ids: Vec<K>) -> Result<(), CommitError> {
        // we may not need to return values here, we can just read them.
        use DelayedApplyEntry::*;

        if idx_to_commit != self.next_idx_to_commit.load(Ordering::SeqCst) {
            return Err(CommitError::CodeInvariantError(
                "idx_to_commit must be next_idx_to_commit".to_string(),
            ));
        }

        // Track separately, todo_deltas need to be done before todo_derived
        let mut todo_deltas = Vec::new();
        let mut todo_derived = Vec::new();

        for id in ids {
            let mut versioned_value = self
                .values
                .get_mut(&id)
                .expect("Value in commit needs to be in the HashMap");
            let entry_to_commit = versioned_value
                .versioned_map
                .get(&idx_to_commit)
                .expect("Value in commit at that transaction version needs to be in the HashMap");

            let new_entry = match entry_to_commit.as_ref().deref() {
                VersionEntry::Value(_, None) => None,
                // remove delta in the commit
                VersionEntry::Value(v, Some(_)) => Some(v.clone()),
                VersionEntry::Apply(AggregatorDelta { delta }) => {
                    let prev_value = versioned_value.read_latest_predicted_value(idx_to_commit)
                        .map_err(|e| CommitError::CodeInvariantError(format!("Cannot read latest committed value for Apply(AggregatorDelta) during commit: {:?}", e)))?;
                    if let DelayedFieldValue::Aggregator(base) = prev_value {
                        let new_value = delta.apply_to(base).map_err(|e| {
                            CommitError::ReExecutionNeeded(format!(
                                "Failed to apply delta to base: {:?}",
                                e
                            ))
                        })?;
                        Some(DelayedFieldValue::Aggregator(new_value))
                    } else {
                        return Err(CommitError::CodeInvariantError(
                            "Cannot apply delta to non-DelayedField::Aggregator".to_string(),
                        ));
                    }
                },
                VersionEntry::Apply(SnapshotDelta {
                    base_aggregator,
                    delta,
                }) => {
                    todo_deltas.push((id, *base_aggregator, *delta));
                    None
                },
                VersionEntry::Apply(SnapshotDerived {
                    base_snapshot,
                    formula,
                }) => {
                    // Because Derived values can depend on the current value, we need to compute other values before it.
                    todo_derived.push((id, *base_snapshot, formula.clone()));
                    None
                },
                VersionEntry::Estimate(_) => {
                    return Err(CommitError::CodeInvariantError(
                        "Cannot commit an estimate".to_string(),
                    ))
                },
            };

            if let Some(new_entry) = new_entry {
                versioned_value.insert_final_value(idx_to_commit, new_entry);
            }
        }

        for (id, base_aggregator, delta) in todo_deltas {
            let new_entry = {
                let prev_value = self.values
                    .get_mut(&base_aggregator)
                    .ok_or_else(|| CommitError::CodeInvariantError("Cannot find base_aggregator for Apply(SnapshotDelta) during commit".to_string()))?
                    .read_latest_predicted_value(idx_to_commit)
                    .map_err(|e| CommitError::CodeInvariantError(format!("Cannot read latest committed value for base aggregator for ApplySnapshotDelta) during commit: {:?}", e)))?;

                if let DelayedFieldValue::Aggregator(base) = prev_value {
                    let new_value = delta.apply_to(base).map_err(|e| {
                        CommitError::ReExecutionNeeded(format!(
                            "Failed to apply delta to base: {:?}",
                            e
                        ))
                    })?;
                    DelayedFieldValue::Snapshot(new_value)
                } else {
                    return Err(CommitError::CodeInvariantError(
                        "Cannot apply delta to non-DelayedField::Aggregator".to_string(),
                    ));
                }
            };

            let mut versioned_value = self
                .values
                .get_mut(&id)
                .expect("Value in commit needs to be in the HashMap");
            versioned_value.insert_final_value(idx_to_commit, new_entry);
        }

        for (id, base_snapshot, formula) in todo_derived {
            let new_entry = {
                let prev_value = self.values
                    .get_mut(&base_snapshot)
                    .ok_or_else(|| CommitError::CodeInvariantError("Cannot find base_aggregator for Apply(SnapshotDelta) during commit".to_string()))?
                    // Read values committed in this commit
                    .read_latest_predicted_value(idx_to_commit + 1)
                    .map_err(|e| CommitError::CodeInvariantError(format!("Cannot read latest committed value for base aggregator for ApplySnapshotDelta) during commit: {:?}", e)))?;

                if let DelayedFieldValue::Snapshot(base) = prev_value {
                    let new_value = formula.apply_to(base);
                    DelayedFieldValue::Derived(new_value)
                } else {
                    return Err(CommitError::CodeInvariantError(
                        "Cannot apply delta to non-DelayedField::Aggregator".to_string(),
                    ));
                }
            };

            let mut versioned_value = self
                .values
                .get_mut(&id)
                .expect("Value in commit needs to be in the HashMap");
            versioned_value.insert_final_value(idx_to_commit, new_entry);
        }

        // Need to assert, because if not matching we are in an inconsistent state.
        assert_eq!(
            idx_to_commit,
            self.next_idx_to_commit.fetch_add(1, Ordering::SeqCst)
        );

        Ok(())
    }

    fn read_checked_depth(
        &self,
        id: &K,
        txn_idx: TxnIndex,
        cur_depth: usize,
    ) -> Result<DelayedFieldValue, PanicOr<MVDelayedFieldsError>> {
        let read_res = self
            .values
            .get(id)
            .ok_or(PanicOr::Or(MVDelayedFieldsError::NotFound))?
            .read(txn_idx)?;
        // The lock on id is out of scope.

        match read_res {
            VersionedRead::Value(v) => Ok(v),
            VersionedRead::DependentApply(dependent_id, dependent_txn_idx, apply) => {
                if &dependent_id == id {
                    return Err(code_invariant_error(format!(
                        "ID of dependent apply {:?} is same as self {:?}",
                        VersionedRead::DependentApply(dependent_id, dependent_txn_idx, apply),
                        id
                    ))
                    .into());
                }

                if cur_depth > 2 {
                    return Err(code_invariant_error(format!(
                        "Recursive depth for read(), {:?} for {:?}",
                        VersionedRead::DependentApply(dependent_id, dependent_txn_idx, apply),
                        id
                    ))
                    .into());
                }
                // Read the source aggregator of snapshot.
                let source_value =
                    self.read_checked_depth(&dependent_id, dependent_txn_idx, cur_depth + 1)?;

                apply
                    .apply_to_base(source_value)
                    .map_err(MVDelayedFieldsError::from_panic_or)
            },
        }
    }

    pub fn remove_all_at_or_after_for_epilogue(
        &self,
        txn_idx: TxnIndex,
        epilogue_txn_idx: TxnIndex,
    ) {
        for mut entry in self.values.iter_mut() {
            entry.value_mut().versioned_map.split_off(&txn_idx);
        }
        self.next_idx_to_commit
            .store(epilogue_txn_idx, Ordering::SeqCst);
    }
}

impl<K: Eq + Hash + Clone + Debug + Copy> TVersionedDelayedFieldView<K>
    for VersionedDelayedFields<K>
{
    fn read(
        &self,
        id: &K,
        txn_idx: TxnIndex,
    ) -> Result<DelayedFieldValue, PanicOr<MVDelayedFieldsError>> {
        self.read_checked_depth(id, txn_idx, 0)
    }

    /// Returns the committed value from largest transaction index that is
    /// smaller than the given current_txn_idx (read_position defined whether
    /// inclusively or exclusively from the current transaction itself).
    fn read_latest_predicted_value(
        &self,
        id: &K,
        current_txn_idx: TxnIndex,
        read_position: ReadPosition,
    ) -> Result<DelayedFieldValue, MVDelayedFieldsError> {
        self.values
            .get_mut(id)
            .ok_or(MVDelayedFieldsError::NotFound)
            .and_then(|v| {
                v.read_latest_predicted_value(
                    match read_position {
                        ReadPosition::BeforeCurrentTxn => current_txn_idx,
                        ReadPosition::AfterCurrentTxn => current_txn_idx + 1,
                    }
                    .min(self.next_idx_to_commit.load(Ordering::Relaxed)),
                )
            })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use aptos_aggregator::{
        bounded_math::SignedU128, delta_change_set::DeltaOp, delta_math::DeltaHistory,
    };
    use aptos_types::delayed_fields::SnapshotToStringFormula;
    use claims::{assert_err_eq, assert_ok_eq, assert_some};
    use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
    use test_case::test_case;

    // Different type acronyms used for generating different test cases.
    const NO_ENTRY: usize = 0;
    const VALUE_AGGREGATOR: usize = 1;
    const VALUE_SNAPSHOT: usize = 2;
    const VALUE_DERIVED: usize = 3;
    const APPLY_AGGREGATOR: usize = 4;
    const APPLY_SNAPSHOT: usize = 5;
    const APPLY_DERIVED: usize = 6;
    const ESTIMATE_NO_BYPASS: usize = 7;

    // For compactness, in tests where the Delta contents do not matter.
    fn test_delta() -> DeltaOp {
        DeltaOp::new(SignedU128::Positive(30), 1000, DeltaHistory {
            max_achieved_positive_delta: 30,
            min_achieved_negative_delta: 0,
            max_underflow_negative_delta: None,
            min_overflow_positive_delta: None,
        })
    }

    fn negative_delta() -> DeltaOp {
        DeltaOp::new(SignedU128::Negative(30), 1000, DeltaHistory {
            max_achieved_positive_delta: 0,
            min_achieved_negative_delta: 30,
            min_overflow_positive_delta: None,
            max_underflow_negative_delta: None,
        })
    }

    fn test_formula() -> SnapshotToStringFormula {
        SnapshotToStringFormula::Concat {
            prefix: vec![70],
            suffix: vec![90],
        }
    }

    fn aggregator_entry(type_index: usize) -> Option<VersionEntry<DelayedFieldID>> {
        match type_index {
            NO_ENTRY => None,
            VALUE_AGGREGATOR => Some(VersionEntry::Value(DelayedFieldValue::Aggregator(10), None)),
            VALUE_SNAPSHOT => Some(VersionEntry::Value(DelayedFieldValue::Snapshot(13), None)),
            VALUE_DERIVED => Some(VersionEntry::Value(
                DelayedFieldValue::Derived(vec![70, 80, 90]),
                None,
            )),
            APPLY_AGGREGATOR => Some(VersionEntry::Apply(DelayedApplyEntry::AggregatorDelta {
                delta: test_delta(),
            })),
            APPLY_SNAPSHOT => Some(VersionEntry::Apply(DelayedApplyEntry::SnapshotDelta {
                base_aggregator: DelayedFieldID::new_for_test_for_u64(2),
                delta: test_delta(),
            })),
            APPLY_DERIVED => Some(VersionEntry::Apply(DelayedApplyEntry::SnapshotDerived {
                base_snapshot: DelayedFieldID::new_for_test_for_u64(3),
                formula: test_formula(),
            })),
            ESTIMATE_NO_BYPASS => Some(VersionEntry::Estimate(EstimatedEntry::NoBypass)),
            _ => unreachable!("Wrong type index in test"),
        }
    }

    fn aggregator_entry_aggregator_value_and_delta(
        value: u128,
        delta: DeltaOp,
    ) -> VersionEntry<DelayedFieldID> {
        VersionEntry::Value(
            DelayedFieldValue::Aggregator(value),
            Some(DelayedApplyEntry::AggregatorDelta { delta }),
        )
    }

    fn aggregator_entry_snapshot_value_and_delta(
        value: u128,
        delta: DeltaOp,
        base_aggregator: DelayedFieldID,
    ) -> VersionEntry<DelayedFieldID> {
        VersionEntry::Value(
            DelayedFieldValue::Snapshot(value),
            Some(DelayedApplyEntry::SnapshotDelta {
                base_aggregator,
                delta,
            }),
        )
    }

    fn aggregator_entry_derived_value_and_delta(
        value: Vec<u8>,
        formula: SnapshotToStringFormula,
        base_snapshot: DelayedFieldID,
    ) -> VersionEntry<DelayedFieldID> {
        VersionEntry::Value(
            DelayedFieldValue::Derived(value),
            Some(DelayedApplyEntry::SnapshotDerived {
                base_snapshot,
                formula,
            }),
        )
    }

    macro_rules! assert_read_aggregator_value {
        ($cond:expr, $expected:expr) => {
            assert_ok_eq!(
                $cond,
                VersionedRead::Value(DelayedFieldValue::Aggregator($expected))
            );
        };
    }

    macro_rules! assert_read_snapshot_value {
        ($cond:expr, $expected:expr) => {
            assert_ok_eq!(
                $cond,
                VersionedRead::Value(DelayedFieldValue::Snapshot($expected))
            );
        };
    }

    macro_rules! assert_read_derived_value {
        ($cond:expr, $expected:expr) => {
            assert_ok_eq!(
                $cond,
                VersionedRead::Value(DelayedFieldValue::Derived($expected))
            );
        };
    }

    macro_rules! assert_read_snapshot_dependent_apply {
        ($cond:expr, $expected_id:expr, $expected_txn_index:expr, $expected_delta:expr) => {
            assert_ok_eq!(
                $cond,
                VersionedRead::DependentApply(
                    DelayedFieldID::new_for_test_for_u64($expected_id),
                    $expected_txn_index,
                    DelayedApplyEntry::SnapshotDelta {
                        base_aggregator: DelayedFieldID::new_for_test_for_u64($expected_id),
                        delta: $expected_delta
                    }
                )
            );
        };
    }

    macro_rules! assert_read_derived_dependent_apply {
        ($cond:expr, $expected_id:expr, $expected_txn_index:expr, $expected_formula:expr) => {
            assert_ok_eq!(
                $cond,
                VersionedRead::DependentApply(
                    DelayedFieldID::new_for_test_for_u64($expected_id),
                    $expected_txn_index,
                    DelayedApplyEntry::SnapshotDerived {
                        base_snapshot: DelayedFieldID::new_for_test_for_u64($expected_id),
                        formula: $expected_formula
                    }
                )
            );
        };
    }

    #[should_panic]
    #[test_case(NO_ENTRY)]
    #[test_case(VALUE_AGGREGATOR)]
    #[test_case(VALUE_SNAPSHOT)]
    #[test_case(VALUE_DERIVED)]
    #[test_case(APPLY_AGGREGATOR)]
    #[test_case(APPLY_SNAPSHOT)]
    #[test_case(APPLY_DERIVED)]
    #[test_case(ESTIMATE_NO_BYPASS)]
    // Insert all possible entries at a wrong txn_idx, ensure mark_estimate panics.
    fn mark_estimate_no_entry(type_index: usize) {
        let mut v = VersionedValue::new(None);
        if let Some(entry) = aggregator_entry(type_index) {
            v.insert_speculative_value(10, entry).unwrap();
        }
        if let Some(entry) = aggregator_entry(type_index) {
            v.insert_speculative_value(3, entry).unwrap();
        }
        v.mark_estimate(5);
    }

    #[should_panic]
    #[test]
    fn mark_estimate_wrong_entry() {
        let mut v = VersionedValue::new(None);
        v.insert_speculative_value(3, aggregator_entry(VALUE_AGGREGATOR).unwrap())
            .unwrap();
        v.mark_estimate(3);

        // Marking an Estimate (first we confirm) as estimate is not allowed.
        assert_matches!(
            v.versioned_map
                .get(&3)
                .expect("Expecting an Estimate entry")
                .as_ref()
                .deref(),
            VersionEntry::Estimate(EstimatedEntry::NoBypass)
        );
        v.mark_estimate(3);
    }

    #[should_panic]
    // Inserting estimates isn't allowed, must use mark_estimate.
    #[test]
    fn insert_estimate() {
        let mut v = VersionedValue::new(None);
        v.insert_speculative_value(3, aggregator_entry(ESTIMATE_NO_BYPASS).unwrap())
            .unwrap();
    }

    #[test]
    fn estimate_bypass() {
        let mut v = VersionedValue::new(None);
        v.insert_speculative_value(2, aggregator_entry(VALUE_AGGREGATOR).unwrap())
            .unwrap();
        v.insert_speculative_value(
            3,
            aggregator_entry_aggregator_value_and_delta(15, test_delta()),
        )
        .unwrap();
        v.insert_speculative_value(4, aggregator_entry(APPLY_AGGREGATOR).unwrap())
            .unwrap();
        v.insert_speculative_value(
            10,
            aggregator_entry_aggregator_value_and_delta(15, test_delta()),
        )
        .unwrap();

        // Delta + Value(15)
        assert_read_aggregator_value!(v.read(5), 45);

        v.mark_estimate(3);
        let val_bypass = v.versioned_map.get(&3);
        assert_some!(val_bypass);
        assert_matches!(
            val_bypass.unwrap().as_ref().deref(),
            VersionEntry::Estimate(EstimatedEntry::Bypass(
                DelayedApplyEntry::AggregatorDelta { .. }
            ))
        );
        // Delta(30) + Value delta bypass(30) + Value(10)
        assert_read_aggregator_value!(v.read(5), 70);

        v.mark_estimate(4);
        let delta_bypass = v.versioned_map.get(&4);
        assert_some!(delta_bypass);
        assert_matches!(
            delta_bypass.unwrap().as_ref().deref(),
            VersionEntry::Estimate(EstimatedEntry::Bypass(
                DelayedApplyEntry::AggregatorDelta { .. }
            ))
        );
        // Delta bypass(30) + Value delta bypass(30) + Value(10)
        assert_read_aggregator_value!(v.read(5), 70);

        v.mark_estimate(2);
        let val_no_bypass = v.versioned_map.get(&2);
        assert_some!(val_no_bypass);
        assert_matches!(
            val_no_bypass.unwrap().as_ref().deref(),
            VersionEntry::Estimate(EstimatedEntry::NoBypass)
        );
        assert_err_eq!(v.read(5), PanicOr::Or(MVDelayedFieldsError::Dependency(2)));

        // Next, ensure read_estimate_deltas remains true if entries are overwritten
        // with matching deltas. Check at each point to not rely on the invariant that
        // read_estimate_deltas can only become false from true.
        v.insert_speculative_value(2, aggregator_entry(VALUE_AGGREGATOR).unwrap())
            .unwrap();
        assert!(v.read_estimate_deltas);
        v.insert_speculative_value(
            3,
            aggregator_entry_aggregator_value_and_delta(15, test_delta()),
        )
        .unwrap();
        assert!(v.read_estimate_deltas);
        v.insert_speculative_value(4, aggregator_entry(APPLY_AGGREGATOR).unwrap())
            .unwrap();
        assert!(v.read_estimate_deltas);

        // Previously value with delta fallback was converted to the delta bypass in
        // the Estimate. It can match a delta too and not disable read_estimate_deltas.
        v.mark_estimate(10);
        v.insert_speculative_value(10, aggregator_entry(APPLY_AGGREGATOR).unwrap())
            .unwrap();
        assert!(v.read_estimate_deltas);
    }

    #[test]
    fn estimate_logic_and_bypass_snapshot() {
        {
            let mut v = VersionedValue::new(None);

            v.insert_speculative_value(6, aggregator_entry(VALUE_SNAPSHOT).unwrap())
                .unwrap();
            assert_read_snapshot_value!(v.read(7), 13);

            v.mark_estimate(6);
            let val_no_bypass = v.versioned_map.get(&6);
            assert_some!(val_no_bypass);
            assert_matches!(
                val_no_bypass.unwrap().as_ref().deref(),
                VersionEntry::Estimate(EstimatedEntry::NoBypass)
            );
            assert_err_eq!(v.read(7), PanicOr::Or(MVDelayedFieldsError::Dependency(6)));
        }

        {
            let mut v = VersionedValue::new(None);
            v.insert_speculative_value(
                8,
                aggregator_entry_snapshot_value_and_delta(
                    13,
                    test_delta(),
                    DelayedFieldID::new_for_test_for_u64(2),
                ),
            )
            .unwrap();

            assert_read_snapshot_value!(v.read(9), 13);

            v.mark_estimate(8);
            let snapshot_bypass = v.versioned_map.get(&8);
            assert_some!(snapshot_bypass);
            assert_matches!(
                snapshot_bypass.unwrap().as_ref().deref(),
                VersionEntry::Estimate(EstimatedEntry::Bypass(
                    DelayedApplyEntry::SnapshotDelta { .. }
                ))
            );

            assert_read_snapshot_dependent_apply!(v.read(9), 2, 8, test_delta());

            v.insert_speculative_value(6, aggregator_entry(VALUE_SNAPSHOT).unwrap())
                .unwrap();
            assert!(v.read_estimate_deltas);
        }

        {
            // old value shouldn't affect snapshot computation, as it depends on aggregator value.
            let mut v = VersionedValue::new(Some(DelayedFieldValue::Snapshot(3)));
            v.insert_speculative_value(10, aggregator_entry(APPLY_SNAPSHOT).unwrap())
                .unwrap();

            assert_read_snapshot_dependent_apply!(v.read(12), 2, 10, test_delta());
        }
    }

    #[test]
    fn estimate_logic_and_bypass_derive() {
        {
            let mut v = VersionedValue::new(None);

            v.insert_speculative_value(6, aggregator_entry(VALUE_DERIVED).unwrap())
                .unwrap();
            assert_read_derived_value!(v.read(7), vec![70, 80, 90]);

            v.mark_estimate(6);
            let val_no_bypass = v.versioned_map.get(&6);
            assert_some!(val_no_bypass);
            assert_matches!(
                val_no_bypass.unwrap().as_ref().deref(),
                VersionEntry::Estimate(EstimatedEntry::NoBypass)
            );
            assert_err_eq!(v.read(7), PanicOr::Or(MVDelayedFieldsError::Dependency(6)));
        }

        {
            let mut v = VersionedValue::new(None);
            v.insert_speculative_value(
                8,
                aggregator_entry_derived_value_and_delta(
                    vec![70, 80, 90],
                    test_formula(),
                    DelayedFieldID::new_for_test_for_u64(3),
                ),
            )
            .unwrap();

            assert_read_derived_value!(v.read(10), vec![70, 80, 90]);

            v.mark_estimate(8);
            let snapshot_bypass = v.versioned_map.get(&8);
            assert_some!(snapshot_bypass);
            assert_matches!(
                snapshot_bypass.unwrap().as_ref().deref(),
                VersionEntry::Estimate(EstimatedEntry::Bypass(
                    DelayedApplyEntry::SnapshotDerived { .. }
                ))
            );

            assert_read_derived_dependent_apply!(v.read(10), 3, 9, test_formula());

            v.insert_speculative_value(6, aggregator_entry(VALUE_SNAPSHOT).unwrap())
                .unwrap();
            assert!(v.read_estimate_deltas);
        }

        {
            // old value shouldn't affect derived computation, as it depends on Snapshot value.
            let mut v = VersionedValue::new(Some(DelayedFieldValue::Derived(vec![80])));
            v.insert_speculative_value(10, aggregator_entry(APPLY_DERIVED).unwrap())
                .unwrap();

            assert_read_derived_dependent_apply!(v.read(12), 3, 11, test_formula());
        }
    }

    #[should_panic]
    #[test_case(NO_ENTRY)]
    #[test_case(VALUE_AGGREGATOR)]
    #[test_case(VALUE_SNAPSHOT)]
    #[test_case(VALUE_DERIVED)]
    #[test_case(APPLY_AGGREGATOR)]
    #[test_case(APPLY_SNAPSHOT)]
    #[test_case(APPLY_DERIVED)]
    fn remove_non_estimate(type_index: usize) {
        let mut v = VersionedValue::new(None);
        if let Some(entry) = aggregator_entry(type_index) {
            v.insert_speculative_value(10, entry).unwrap();
        }
        v.remove(10);
    }

    #[test]
    fn remove_estimate() {
        let mut v = VersionedValue::new(None);
        v.insert_speculative_value(3, aggregator_entry(VALUE_AGGREGATOR).unwrap())
            .unwrap();
        v.mark_estimate(3);
        v.remove(3);
        assert!(!v.read_estimate_deltas);
    }

    #[should_panic]
    #[test_case(APPLY_AGGREGATOR)]
    #[test_case(APPLY_SNAPSHOT)]
    #[test_case(APPLY_DERIVED)]
    fn read_committed_not_value(type_index: usize) {
        let mut v = VersionedValue::new(None);
        if let Some(entry) = aggregator_entry(type_index) {
            v.insert_speculative_value(10, entry).unwrap();
        }
        let _ = v.read_latest_predicted_value(11);
    }

    #[test_case(APPLY_AGGREGATOR)]
    #[test_case(APPLY_SNAPSHOT)]
    #[test_case(APPLY_DERIVED)]
    fn read_first_entry_not_value(type_index: usize) {
        let mut v = VersionedValue::new(None);
        assert_matches!(
            v.read_latest_predicted_value(11),
            Err(MVDelayedFieldsError::NotFound)
        );

        if let Some(entry) = aggregator_entry(type_index) {
            v.insert_speculative_value(12, entry).unwrap();
        }
        assert_matches!(
            v.read_latest_predicted_value(11),
            Err(MVDelayedFieldsError::NotFound)
        );
    }

    #[test]
    fn read_first_entry_value() {
        let mut v = VersionedValue::new(None);
        v.insert_speculative_value(13, aggregator_entry(APPLY_AGGREGATOR).unwrap())
            .unwrap();
        v.insert_speculative_value(12, aggregator_entry(VALUE_AGGREGATOR).unwrap())
            .unwrap();

        assert_matches!(
            v.read_latest_predicted_value(11),
            Ok(DelayedFieldValue::Aggregator(10))
        );

        v.insert_speculative_value(
            9,
            VersionEntry::Value(DelayedFieldValue::Aggregator(9), None),
        )
        .unwrap();
        assert_matches!(
            v.read_latest_predicted_value(11),
            Ok(DelayedFieldValue::Aggregator(9))
        );
    }

    #[should_panic]
    #[test]
    fn read_committed_estimate() {
        let mut v = VersionedValue::new(None);
        v.insert_speculative_value(3, aggregator_entry(VALUE_AGGREGATOR).unwrap())
            .unwrap();
        v.mark_estimate(3);
        let _ = v.read_latest_predicted_value(11);
    }

    #[test]
    fn read_latest_predicted_value() {
        let mut v = VersionedValue::new(Some(DelayedFieldValue::Aggregator(5)));
        v.insert_speculative_value(2, aggregator_entry(VALUE_AGGREGATOR).unwrap())
            .unwrap();
        v.insert_speculative_value(
            4,
            aggregator_entry_aggregator_value_and_delta(15, test_delta()),
        )
        .unwrap();

        assert_ok_eq!(
            v.read_latest_predicted_value(5),
            DelayedFieldValue::Aggregator(15)
        );
        assert_ok_eq!(
            v.read_latest_predicted_value(4),
            DelayedFieldValue::Aggregator(10)
        );
        assert_ok_eq!(
            v.read_latest_predicted_value(2),
            DelayedFieldValue::Aggregator(5)
        );
    }

    #[test]
    fn read_delta_chain() {
        let mut v = VersionedValue::new(Some(DelayedFieldValue::Aggregator(5)));
        v.insert_speculative_value(4, aggregator_entry(APPLY_AGGREGATOR).unwrap())
            .unwrap();
        v.insert_speculative_value(8, aggregator_entry(APPLY_AGGREGATOR).unwrap())
            .unwrap();
        v.insert_speculative_value(12, aggregator_entry(APPLY_AGGREGATOR).unwrap())
            .unwrap();
        v.insert_speculative_value(16, aggregator_entry(APPLY_AGGREGATOR).unwrap())
            .unwrap();

        assert_read_aggregator_value!(v.read(0), 5);
        assert_read_aggregator_value!(v.read(5), 35);
        assert_read_aggregator_value!(v.read(9), 65);
        assert_read_aggregator_value!(v.read(13), 95);
        assert_read_aggregator_value!(v.read(17), 125);
    }

    #[test]
    fn read_errors() {
        let mut v = VersionedValue::new(None);
        v.insert_speculative_value(2, aggregator_entry(VALUE_AGGREGATOR).unwrap())
            .unwrap();

        assert_err_eq!(v.read(1), PanicOr::Or(MVDelayedFieldsError::NotFound));

        v.insert_speculative_value(
            8,
            VersionEntry::Apply(DelayedApplyEntry::AggregatorDelta {
                delta: negative_delta(),
            }),
        )
        .unwrap();
        assert_err_eq!(
            v.read(9),
            PanicOr::Or(MVDelayedFieldsError::DeltaApplicationFailure)
        );
        // Ensure without underflow there would not be a failure.

        v.insert_speculative_value(4, aggregator_entry(APPLY_AGGREGATOR).unwrap())
            .unwrap(); // adds 30.
        assert_read_aggregator_value!(v.read(9), 10);

        v.insert_speculative_value(
            6,
            VersionEntry::Value(DelayedFieldValue::Aggregator(35), None),
        )
        .unwrap();
        assert_read_aggregator_value!(v.read(9), 5);

        v.mark_estimate(2);
        assert_err_eq!(v.read(3), PanicOr::Or(MVDelayedFieldsError::Dependency(2)));
    }

    // TODO[agg_v2](tests): add tests for try-commit
}
