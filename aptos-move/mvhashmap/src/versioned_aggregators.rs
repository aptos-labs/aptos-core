// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::types::{AtomicTxnIndex, MVAggregatorsError, TxnIndex};
use aptos_aggregator::{
    aggregator_change_set::{AggregatorApplyChange, ApplyBase},
    delta_change_set::DeltaOp,
    types::AggregatorValue,
};
use claims::{assert_matches, assert_none};
use crossbeam::utils::CachePadded;
use dashmap::DashMap;
use std::{
    collections::btree_map::{BTreeMap, Entry},
    fmt::Debug,
    hash::Hash,
    iter::DoubleEndedIterator,
    sync::atomic::Ordering,
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
    Bypass(AggregatorApplyChange<K>),
}

// There is no explicit deletion as it will be impossible to resolve the ID of a deleted
// aggregator (as the ID will not be contained in a resource anymore). The write on the
// previously holding resource and Block-STM read validation ensures correctness.
#[derive(Debug)]
enum AggregatorEntry<K: Clone> {
    // If the value is determined by a delta applied to a value read during the execution
    // of the same transaction, the delta may also be kept in the entry. This is useful
    // if speculative execution aborts and the entry is marked as estimate, as the delta
    // may be used to avoid waiting on the Estimate entry. More in comments below.
    Value(AggregatorValue, Option<AggregatorApplyChange<K>>),
    // Applies the change on top of the previous entry - either for the same ID corresponding
    // to this change, or for the apply_base_id given by the change, at a specific point defined
    // by the it's ApplyBase.
    Apply(AggregatorApplyChange<K>),
    // Marks the entry as an estimate, indicating that the next incarnation of the
    // transaction is estimated to populate the entry. May contain a bypass internally
    // (allowing a read operation to avoid waiting for the corresponding dependency),
    // encapulated by the EstimatedEntry.
    Estimate(EstimatedEntry<K>),
}

// A VersionedValue internally contains a BTreeMap from indices of transactions
// that update a given aggregator, alongside the corresponding entries.
#[derive(Debug)]
struct VersionedValue<K: Clone> {
    versioned_map: BTreeMap<TxnIndex, CachePadded<AggregatorEntry<K>>>,

    // The value of the given aggregator prior to the block execution. None implies that
    // the aggregator did not exist prior to the block.
    base_value: Option<AggregatorValue>,

    // If true, the reads can proceed by using deltas in Estimate entries, if present.
    // The value is optimistically initialized to true, but changed to false when it is
    // observed that a later incarnation changed the value of a delta at the same entry.
    read_estimate_deltas: bool,
}

#[derive(Debug, PartialEq)]
enum VersionedRead<K: Clone> {
    Value(AggregatorValue),
    // The transaction index records the index at which the Snapshot was encountered.
    // This is required for the caller to resolve the value of the aggregator (with the
    // recorded id) from which the snapshot was created at the correct version (index).
    DependentApply(K, TxnIndex, AggregatorApplyChange<K>),
}

impl<K: Copy + Clone + Debug + Eq> VersionedValue<K> {
    // VersionedValue should only be created when base value of the corresponding aggregator
    // is known & provided to the constructor.
    fn new(base_value: Option<AggregatorValue>) -> Self {
        Self {
            versioned_map: BTreeMap::new(),
            base_value,
            // Enable the optimization to not wait on dependencies during reading by default.
            read_estimate_deltas: true,
        }
    }

    fn mark_estimate(&mut self, txn_idx: TxnIndex) {
        use AggregatorEntry::*;
        use EstimatedEntry::*;

        match self.versioned_map.entry(txn_idx) {
            Entry::Occupied(mut o) => {
                let bypass = match &**o.get() {
                    Value(_, maybe_apply) => maybe_apply.clone().map_or(NoBypass, Bypass),
                    Apply(apply) => Bypass(apply.clone()),
                    Estimate(_) => unreachable!("Entry already marked estimate"),
                };

                o.insert(CachePadded::new(Estimate(bypass)));
            },
            Entry::Vacant(_) => unreachable!("Versioned entry must exist when marking as estimate"),
        };
    }

    fn remove(&mut self, txn_idx: TxnIndex) {
        let deleted_entry = self.versioned_map.remove(&txn_idx);
        // Entries should only be deleted if the transaction that produced them is
        // aborted and re-executed, but abort must have marked the entry as an Estimate.
        assert_matches!(
            &*deleted_entry.expect("Entry must exist to be removed"),
            AggregatorEntry::Estimate(_),
            "Removed entry must be an Estimate",
        );
        // Incarnation changed output behavior, disable reading through estimates optimization.
        self.read_estimate_deltas = false;
    }

    fn insert(&mut self, txn_idx: TxnIndex, entry: AggregatorEntry<K>) {
        use AggregatorApplyChange::*;
        use AggregatorEntry::*;
        use EstimatedEntry::*;

        assert!(
            !matches!(entry, Estimate(_)),
            "Inserting Estimate is not allowed - must call mark_estimate"
        );

        match self.versioned_map.entry(txn_idx) {
            Entry::Occupied(mut o) => {
                if !match (&**o.get(), &entry) {
                    // These are the cases where the transaction behavior with respect to the
                    // aggregator may change (based on the information recorded in the Estimate).
                    (Estimate(Bypass(apply_l)), Apply(apply_r) | Value(_, Some(apply_r))) => match (apply_l, apply_r) {
                        (AggregatorDelta { .. }, AggregatorDelta { .. })
                        | (SnapshotDelta { .. }, SnapshotDelta { .. })
                        | (SnapshotDerived { .. }, SnapshotDerived { .. }) => {
                            println!("Checking {:?} and {:?}, match: {}", apply_l, apply_r, *apply_l == *apply_r);
                            *apply_l == *apply_r
                        },
                        _ => unreachable!(
                            "Storing {:?} for aggregator ID that previously had a different type of entry - {:?}",
                            apply_r, apply_l,
                        ),
                    },
                    // There was a value without fallback delta bypass before and still.
                    (Estimate(NoBypass), Value(_, None)) => true,
                    // Bypass stored in the estimate does not match the new entry.
                    (Estimate(_), _) => false,

                    // The following two cases are acceptable uses to record a value after txn
                    // materialization / commit, as the value will never change.
                    //
                    // value & value pattern is allowed to not be too restrictive to the caller.
                    //
                    // The patterns ensure to avoid panic in the unreachable branch below, and
                    // returning 'true' ensures that the bypass enabling logic is not affected.
                    (Value(val_l, None), Value(val_r, _)) if val_l == val_r => true,
                    (Value(_, None), Apply(_)) => true,

                    (_, _) => unreachable!(
                        "Replaced entry must be an Estimate, \
			 or we should be recording the final committed value"
                    ),
                } {
                    self.read_estimate_deltas = false;
                }
                o.insert(CachePadded::new(entry));
            },
            Entry::Vacant(v) => {
                v.insert(CachePadded::new(entry));
            },
        }
    }

    // Given a transaction index which should be committed next, returns the latest value
    // below this version, or an error if such a value does not exist.
    fn read_latest_committed_value(
        &self,
        next_idx_to_commit: TxnIndex,
    ) -> Result<AggregatorValue, MVAggregatorsError> {
        use AggregatorEntry::*;

        self.versioned_map
            .range(0..next_idx_to_commit)
            .next_back()
            .map_or_else(
                || self.base_value.clone().ok_or(MVAggregatorsError::NotFound),
                |(_, entry)| match &**entry {
                    Value(v, _) => Ok(v.clone()),
                    Apply(_) => {
                        unreachable!("Apply entries may not exist for committed txn indices")
                    },
                    Estimate(_) => unreachable!("Committed entry may not be an Estimate"),
                },
            )
    }

    // Traverse down from txn_idx and accumulate encountered deltas until resolving it to
    // a value or return an error. Errors of not finding a value to resolve to take
    // precedence over a DeltaApplicationError.
    fn apply_delta_suffix(
        &self,
        iter: &mut dyn DoubleEndedIterator<Item = (&TxnIndex, &CachePadded<AggregatorEntry<K>>)>,
        delta: DeltaOp,
    ) -> Result<u128, MVAggregatorsError> {
        use AggregatorApplyChange::*;
        use AggregatorEntry::*;
        use EstimatedEntry::*;

        let mut accumulator = delta;
        while let Some((idx, entry)) = iter.next_back() {
            let delta = match (&**entry, self.read_estimate_deltas) {
                (Value(AggregatorValue::Integer(v), _), _) => {
                    // Apply accumulated delta to resolve the aggregator value.
                    return accumulator
                        .apply_to(*v)
                        .map_err(|_| MVAggregatorsError::DeltaApplicationFailure);
                },
                (Value(_, _), _) => {
                    unreachable!("Value not AggregatorValue::Aggregator for Aggregator")
                },
                (Apply(AggregatorDelta { delta }), _)
                | (Estimate(Bypass(AggregatorDelta { delta })), true) => *delta,
                (Estimate(NoBypass), _) | (Estimate(_), false) => {
                    // We must wait on Estimates, or a bypass isn't available.
                    return Err(MVAggregatorsError::Dependency(*idx));
                },
                (Apply(_), _) | (Estimate(Bypass(_)), true) => {
                    unreachable!("Apply change type not AggregatorDelta for aggregator")
                },
            };

            // Read hit a delta during traversing the block and aggregating other deltas. We merge the
            // two deltas together. If there is an error, we return DeltaApplicationError (there is no
            // determinism concern as DeltaApplicationError may not occur in committed output).
            accumulator
                .merge_with_previous_delta(delta)
                .map_err(|_| MVAggregatorsError::DeltaApplicationFailure)?;
        }

        // Finally, resolve if needed with the base value.
        self.base_value
            .as_ref()
            .ok_or(MVAggregatorsError::NotFound)
            .and_then(|base_value| match base_value {
                AggregatorValue::Integer(v) => accumulator
                    .apply_to(*v)
                    .map_err(|_| MVAggregatorsError::DeltaApplicationFailure),
                _ => Err(MVAggregatorsError::DeltaApplicationFailure),
            })
    }

    // Reads a given aggregator value at a given version (transaction index) and produces
    // a ReadResult if successful, which is either a u128 value, or a snapshot specifying
    // a different aggregator (with ID) at a given version and a delta to apply on top.
    fn read(&self, txn_idx: TxnIndex) -> Result<VersionedRead<K>, MVAggregatorsError> {
        use AggregatorApplyChange::*;
        use AggregatorEntry::*;
        use EstimatedEntry::*;
        use MVAggregatorsError::*;

        let mut iter = self.versioned_map.range(0..txn_idx);

        iter.next_back().map_or_else(
            // No entries in versioned map, use base value.
            || {
                self.base_value
                    .clone()
                    .ok_or(NotFound)
                    .map(VersionedRead::Value)
            },
            // Consider the latest entry below the provided version.
            |(idx, entry)| match (&**entry, self.read_estimate_deltas) {
                (Value(v, _), _) => Ok(VersionedRead::Value(v.clone())),
                (Apply(apply), _) | (Estimate(Bypass(apply)), true) => {
                    match apply.get_apply_base_id_option() {
                        None => {
                            // If read encounters a delta, it must traverse the block of transactions
                            // (top-down) until it encounters a value or use the base value.
                            if let AggregatorDelta { delta } = apply {
                                self.apply_delta_suffix(&mut iter, *delta)
                                    .map(AggregatorValue::Integer)
                                    .map(VersionedRead::Value)
                            } else {
                                unreachable!("Only AggregatorDelta has no apply_base_id")
                            }
                        },
                        Some(apply_base) => {
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
                    }
                },
                (Estimate(NoBypass), _) | (Estimate(_), false) => Err(Dependency(*idx)),
            },
        )
    }
}

/// Maps each ID (access path) to an internal VersionedValue, managing versioned updates to the
/// specified aggregator (which handles both Aggregator, and AggregatorSnapshot).
///
/// There are some invariants that the caller must maintain when using VersionedAggregators:
/// -) 'set_base_value' or 'create_aggregator' must be completed for each ID prior to calling
/// other methods (reading, insert, remove or marking as estimate) for the ID.
/// -) When a transaction is committed, all transactions with lower indices are also considered
/// committed. Before an index is committed, all of its deltas and snapshots must be converted
/// to values by the caller (by recoding the final materialized values).
/// -) When a transaction aborts, its entries must be converted to estimates until the
/// transaction can re-execute with the next incarnation. When the next incarnation finishes
/// and records new entries, all remaining Estimate entries must be removed.
///
/// Another invariant that must be maintained by the caller is that the same aggregator ID
/// throughout the course of the lifetime of the data-structure may not contain a delta and
/// a snapshot - even at different times. In particular, this precludes re-using the same ID
/// between Aggregator and AggregatorSnapshot. It is easy to provide this property from the
/// caller side, even if IDs are re-used (say among incarnations) by e.g. assigning odd and
/// even ids to Aggregators and AggregatorSnapshots, and it allows asserting the uses strictly.
pub struct VersionedAggregators<K: Clone> {
    values: DashMap<K, VersionedValue<K>>,

    /// No deltas are allowed below next_idx_to_commit version, as all deltas (and snapshots)
    /// must be materialized and converted to Values during commit.
    next_idx_to_commit: AtomicTxnIndex,
}

impl<K: Eq + Hash + Clone + Debug + Copy> VersionedAggregators<K> {
    // TODO: integrate into the rest of the system.
    #[allow(dead_code)]
    /// Part of the big multi-versioned data-structure, which creates different types of
    /// versioned maps (including this one for aggregators), and delegates access. Hence,
    /// new should only be used from the crate.
    pub(crate) fn new() -> Self {
        Self {
            values: DashMap::new(),
            next_idx_to_commit: AtomicTxnIndex::new(0),
        }
    }

    /// Must be called when an aggregator from storage is resolved, with ID replacing the
    /// base value. This ensures that VersionedValue exists for the aggregator before any
    /// other uses (adding deltas, etc).
    ///
    /// Setting base value multiple times, even concurrently, is okay for the same ID,
    /// because the corresponding value prior to the block is fixed.
    pub fn set_base_value(&self, id: K, base_value: AggregatorValue) {
        self.values
            .entry(id)
            .or_insert(VersionedValue::new(Some(base_value)));
    }

    /// Must be called when an aggregator creation with a given ID and initial value is observed
    /// in the outputs of txn_idx.
    pub fn create_aggregator(&self, id: K, txn_idx: TxnIndex, value: AggregatorValue) {
        let mut created = VersionedValue::new(None);
        created.insert(txn_idx, AggregatorEntry::Value(value, None));

        assert_none!(
            self.values.insert(id, created),
            "VerionedValue when creating aggregator ID may not already exist"
        );
    }

    /// The caller must maintain the invariant that prior to calling the methods below w.
    /// a particular aggregator ID, an invocation of either create_aggregator (for newly created
    /// aggregators), or set_base_value (for existing aggregators) must have been completed.

    pub fn read(&self, id: K, txn_idx: TxnIndex) -> Result<AggregatorValue, MVAggregatorsError> {
        let read_res = self
            .values
            .get(&id)
            .expect("VersionedValue for an (resolved) ID must already exist")
            .read(txn_idx)?;
        // The lock on id is out of scope.

        match read_res {
            VersionedRead::Value(v) => Ok(v),
            VersionedRead::DependentApply(dependend_id, dependent_txn_idx, apply) => {
                // Read the source aggregator of snapshot.
                let source_value = self.read(dependend_id, dependent_txn_idx)?;
                // TODO distinguish between delta application and code invariant broken errors
                apply
                    .apply_to_base(source_value)
                    .map_err(|_| MVAggregatorsError::DeltaApplicationFailure)
            },
        }
    }

    /// This method is intended to be called during transaction execution (e.g. for getting
    /// a rough value of an aggregator cheaply for branch prediction). Hence, the 'calling'
    /// transaction may not be committed yet, and there is no reason to provide txn_idx.
    pub fn read_latest_committed_value(
        &self,
        id: K,
    ) -> Result<AggregatorValue, MVAggregatorsError> {
        self.values
            .get_mut(&id)
            .expect("VersionedValue for an (resolved) ID must already exist")
            .read_latest_committed_value(self.next_idx_to_commit.load(Ordering::Relaxed))
    }

    /// If a value was derived from applying delta to a speculatively read value, we also
    /// provide a delta. This is useful for the optimization where if the txn aborts and
    /// the entry is marked as an estimate, reads may be able to bypass the Estimate entry
    /// by optimistically applying the previous delta.
    ///
    /// Record value can also be used to finalize committed values in the data-structure,
    /// in order to avoid potentially costly delta traversals in reads. Due to a use in
    /// read_latest_committed_value, called frequently (as a part of aggregator implementation),
    /// Upon commit Snapshot and Delta entries are all required to be replaced with Values.
    pub fn record_value(
        &self,
        id: K,
        txn_idx: TxnIndex,
        value: AggregatorValue,
        maybe_apply: Option<AggregatorApplyChange<K>>,
    ) {
        self.values
            .get_mut(&id)
            .expect("VersionedValue for an (resolved) ID must already exist")
            .insert(txn_idx, AggregatorEntry::Value(value, maybe_apply));
    }

    pub fn record_apply(&self, id: K, txn_idx: TxnIndex, apply: AggregatorApplyChange<K>) {
        self.values
            .get_mut(&id)
            .expect("VersionedValue for an (resolved) ID must already exist")
            .insert(txn_idx, AggregatorEntry::Apply(apply));
    }

    pub fn mark_estimate(&self, id: K, txn_idx: TxnIndex) {
        self.values
            .get_mut(&id)
            .expect("VersionedValue for an (resolved) ID must already exist")
            .mark_estimate(txn_idx);
    }

    pub fn remove(&self, id: K, txn_idx: TxnIndex) {
        self.values
            .get_mut(&id)
            .expect("VersionedValue for an (resolved) ID must already exist")
            .remove(txn_idx);
    }

    pub fn try_commit(&self, idx_to_commit: TxnIndex, ids: Vec<K>) -> Result<(), CommitError> {
        // we may not need to return values here, we can just read them.
        use AggregatorApplyChange::*;

        if idx_to_commit != self.next_idx_to_commit.load(Ordering::SeqCst) {
            return Err(CommitError::CodeInvariantError(
                "idx_to_commit must be next_idx_to_commit".to_string(),
            ));
        }

        let mut derived_ids = Vec::new();

        for id in ids {
            let mut versioned_value = self
                .values
                .get_mut(&id)
                .expect("Value in commit needs to be in the HashMap");
            let entry_to_commit = versioned_value
                .versioned_map
                .get(&idx_to_commit)
                .expect("Value in commit at that transaction version needs to be in the HashMap");

            let new_entry = match &**entry_to_commit {
                AggregatorEntry::Value(_, _) => None,
                AggregatorEntry::Apply(AggregatorDelta { delta }) => {
                    let prev_value = versioned_value.read_latest_committed_value(idx_to_commit)
                        .map_err(|e| CommitError::CodeInvariantError(format!("Cannot read latest committed value for Apply(AggregatorDelta) during commit: {:?}", e)))?;
                    if let AggregatorValue::Integer(base) = prev_value {
                        let new_value = delta.apply_to(base).map_err(|e| {
                            CommitError::ReExecutionNeeded(format!(
                                "Failed to apply delta to base: {:?}",
                                e
                            ))
                        })?;
                        Some(AggregatorEntry::Value(
                            AggregatorValue::Integer(new_value),
                            Some(AggregatorDelta { delta: *delta }),
                        ))
                    } else {
                        return Err(CommitError::CodeInvariantError(
                            "Cannot apply delta to non-AggregatorValue::Integer".to_string(),
                        ));
                    }
                },
                AggregatorEntry::Apply(SnapshotDelta {
                    base_aggregator,
                    delta,
                }) => {
                    let prev_value = self.values
                        .get_mut(base_aggregator)
                        .ok_or_else(|| CommitError::CodeInvariantError("Cannot find base_aggregator for Apply(SnapshotDelta) during commit".to_string()))?
                        .read_latest_committed_value(idx_to_commit)
                        .map_err(|e| CommitError::CodeInvariantError(format!("Cannot read latest committed value for base aggregator for ApplySnapshotDelta) during commit: {:?}", e)))?;

                    if let AggregatorValue::Integer(base) = prev_value {
                        let new_value = delta.apply_to(base).map_err(|e| {
                            CommitError::ReExecutionNeeded(format!(
                                "Failed to apply delta to base: {:?}",
                                e
                            ))
                        })?;
                        Some(AggregatorEntry::Value(
                            AggregatorValue::Integer(new_value),
                            Some(SnapshotDelta {
                                base_aggregator: *base_aggregator,
                                delta: *delta,
                            }),
                        ))
                    } else {
                        return Err(CommitError::CodeInvariantError(
                            "Cannot apply delta to non-AggregatorValue::Integer".to_string(),
                        ));
                    }
                },
                AggregatorEntry::Apply(SnapshotDerived { .. }) => {
                    // Because Derived values can depend on the current value, we need to compute other values before it.
                    derived_ids.push(id);
                    None
                },
                AggregatorEntry::Estimate(_) => {
                    return Err(CommitError::CodeInvariantError(
                        "Cannot commit an estimate".to_string(),
                    ))
                },
            };

            if let Some(new_entry) = new_entry {
                versioned_value.insert(idx_to_commit, new_entry);
            }
        }

        for id in derived_ids {
            let mut versioned_value = self
                .values
                .get_mut(&id)
                .expect("Value in commit needs to be in the HashMap");
            let entry_to_commit = versioned_value
                .versioned_map
                .get(&idx_to_commit)
                .expect("Value in commit at that transaction version needs to be in the HashMap");
            let new_entry = match &**entry_to_commit {
                AggregatorEntry::Apply(SnapshotDerived {
                    base_snapshot,
                    formula,
                }) => {
                    let prev_value = self.values
                        .get_mut(base_snapshot)
                        .ok_or_else(|| CommitError::CodeInvariantError("Cannot find base_aggregator for Apply(SnapshotDelta) during commit".to_string()))?
                        // Read values committed in this commit
                        .read_latest_committed_value(idx_to_commit + 1)
                        .map_err(|e| CommitError::CodeInvariantError(format!("Cannot read latest committed value for base aggregator for ApplySnapshotDelta) during commit: {:?}", e)))?;

                    if let AggregatorValue::Integer(base) = prev_value {
                        let new_value = formula.apply(base);
                        AggregatorEntry::Value(
                            AggregatorValue::String(new_value),
                            Some(SnapshotDerived {
                                base_snapshot: *base_snapshot,
                                formula: formula.clone(),
                            }),
                        )
                    } else {
                        return Err(CommitError::CodeInvariantError(
                            "Cannot apply delta to non-AggregatorValue::Integer".to_string(),
                        ));
                    }
                },
                _ => unreachable!("We've only added derived values into derived_ids"),
            };

            versioned_value.insert(idx_to_commit, new_entry);
        }

        self.next_idx_to_commit
            .compare_exchange(
                idx_to_commit,
                idx_to_commit + 1,
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
            .map_err(|v| {
                CommitError::CodeInvariantError(format!(
                    "Cannot update next_idx_to_commit, it must be idx_to_commit ({}), but is {}",
                    idx_to_commit, v
                ))
            })?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use aptos_aggregator::{
        bounded_math::SignedU128,
        delta_change_set::DeltaOp,
        delta_math::DeltaHistory,
        types::{AggregatorID, SnapshotToStringFormula},
    };
    use claims::{assert_err_eq, assert_ok_eq, assert_some};
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

    fn aggregator_entry(type_index: usize) -> Option<AggregatorEntry<AggregatorID>> {
        match type_index {
            NO_ENTRY => None,
            VALUE_AGGREGATOR => Some(AggregatorEntry::Value(AggregatorValue::Integer(10), None)),
            VALUE_SNAPSHOT => Some(AggregatorEntry::Value(AggregatorValue::Integer(13), None)),
            VALUE_DERIVED => Some(AggregatorEntry::Value(
                AggregatorValue::String(vec![70, 80, 90]),
                None,
            )),
            APPLY_AGGREGATOR => Some(AggregatorEntry::Apply(
                AggregatorApplyChange::AggregatorDelta {
                    delta: test_delta(),
                },
            )),
            APPLY_SNAPSHOT => Some(AggregatorEntry::Apply(
                AggregatorApplyChange::SnapshotDelta {
                    base_aggregator: AggregatorID::new(2),
                    delta: test_delta(),
                },
            )),
            APPLY_DERIVED => Some(AggregatorEntry::Apply(
                AggregatorApplyChange::SnapshotDerived {
                    base_snapshot: AggregatorID::new(3),
                    formula: test_formula(),
                },
            )),
            ESTIMATE_NO_BYPASS => Some(AggregatorEntry::Estimate(EstimatedEntry::NoBypass)),
            _ => unreachable!("Wrong type index in test"),
        }
    }

    fn aggregator_entry_aggregator_value_and_delta(
        value: u128,
        delta: DeltaOp,
    ) -> AggregatorEntry<AggregatorID> {
        AggregatorEntry::Value(
            AggregatorValue::Integer(value),
            Some(AggregatorApplyChange::AggregatorDelta { delta }),
        )
    }

    fn aggregator_entry_snapshot_value_and_delta(
        value: u128,
        delta: DeltaOp,
        base_aggregator: AggregatorID,
    ) -> AggregatorEntry<AggregatorID> {
        AggregatorEntry::Value(
            AggregatorValue::Integer(value),
            Some(AggregatorApplyChange::SnapshotDelta {
                base_aggregator,
                delta,
            }),
        )
    }

    fn aggregator_entry_derived_value_and_delta(
        value: Vec<u8>,
        formula: SnapshotToStringFormula,
        base_snapshot: AggregatorID,
    ) -> AggregatorEntry<AggregatorID> {
        AggregatorEntry::Value(
            AggregatorValue::String(value),
            Some(AggregatorApplyChange::SnapshotDerived {
                base_snapshot,
                formula,
            }),
        )
    }

    macro_rules! assert_read_integer_value {
        ($cond:expr, $expected:expr) => {
            assert_ok_eq!(
                $cond,
                VersionedRead::Value(AggregatorValue::Integer($expected))
            );
        };
    }

    macro_rules! assert_read_string_value {
        ($cond:expr, $expected:expr) => {
            assert_ok_eq!(
                $cond,
                VersionedRead::Value(AggregatorValue::String($expected))
            );
        };
    }

    macro_rules! assert_read_snapshot_dependent_apply {
        ($cond:expr, $expected_id:expr, $expected_txn_index:expr, $expected_delta:expr) => {
            assert_ok_eq!(
                $cond,
                VersionedRead::DependentApply(
                    AggregatorID::new($expected_id),
                    $expected_txn_index,
                    AggregatorApplyChange::SnapshotDelta {
                        base_aggregator: AggregatorID::new($expected_id),
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
                    AggregatorID::new($expected_id),
                    $expected_txn_index,
                    AggregatorApplyChange::SnapshotDerived {
                        base_snapshot: AggregatorID::new($expected_id),
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
            v.insert(10, entry);
        }
        if let Some(entry) = aggregator_entry(type_index) {
            v.insert(3, entry);
        }
        v.mark_estimate(5);
    }

    #[should_panic]
    #[test]
    fn mark_estimate_wrong_entry() {
        let mut v = VersionedValue::new(None);
        v.insert(3, aggregator_entry(VALUE_AGGREGATOR).unwrap());
        v.mark_estimate(3);

        // Marking an Estimate (first we confirm) as estimate is not allowed.
        assert_matches!(
            &**v.versioned_map
                .get(&3)
                .expect("Expecting an Estimate entry"),
            AggregatorEntry::Estimate(EstimatedEntry::NoBypass)
        );
        v.mark_estimate(3);
    }

    #[should_panic]
    // Inserting estimates isn't allowed, must use mark_estimate.
    #[test]
    fn insert_estimate() {
        let mut v = VersionedValue::new(None);
        v.insert(3, aggregator_entry(ESTIMATE_NO_BYPASS).unwrap());
    }

    #[test]
    fn estimate_bypass() {
        let mut v = VersionedValue::new(None);
        v.insert(2, aggregator_entry(VALUE_AGGREGATOR).unwrap());
        v.insert(
            3,
            aggregator_entry_aggregator_value_and_delta(15, test_delta()),
        );
        v.insert(4, aggregator_entry(APPLY_AGGREGATOR).unwrap());
        v.insert(
            10,
            aggregator_entry_aggregator_value_and_delta(15, test_delta()),
        );

        // Delta + Value(15)
        assert_read_integer_value!(v.read(5), 45);

        v.mark_estimate(3);
        let val_bypass = v.versioned_map.get(&3);
        assert_some!(val_bypass);
        assert_matches!(
            &**val_bypass.unwrap(),
            AggregatorEntry::Estimate(EstimatedEntry::Bypass(
                AggregatorApplyChange::AggregatorDelta { .. }
            ))
        );
        // Delta(30) + Value delta bypass(30) + Value(10)
        assert_read_integer_value!(v.read(5), 70);

        v.mark_estimate(4);
        let delta_bypass = v.versioned_map.get(&4);
        assert_some!(delta_bypass);
        assert_matches!(
            &**delta_bypass.unwrap(),
            AggregatorEntry::Estimate(EstimatedEntry::Bypass(
                AggregatorApplyChange::AggregatorDelta { .. }
            ))
        );
        // Delta bypass(30) + Value delta bypass(30) + Value(10)
        assert_read_integer_value!(v.read(5), 70);

        v.mark_estimate(2);
        let val_no_bypass = v.versioned_map.get(&2);
        assert_some!(val_no_bypass);
        assert_matches!(
            &**val_no_bypass.unwrap(),
            AggregatorEntry::Estimate(EstimatedEntry::NoBypass)
        );
        assert_err_eq!(v.read(5), MVAggregatorsError::Dependency(2));

        // Next, ensure read_estimate_deltas remains true if entries are overwritten
        // with matching deltas. Check at each point to not rely on the invariant that
        // read_estimate_deltas can only become false from true.
        v.insert(2, aggregator_entry(VALUE_AGGREGATOR).unwrap());
        assert!(v.read_estimate_deltas);
        v.insert(
            3,
            aggregator_entry_aggregator_value_and_delta(15, test_delta()),
        );
        assert!(v.read_estimate_deltas);
        v.insert(4, aggregator_entry(APPLY_AGGREGATOR).unwrap());
        assert!(v.read_estimate_deltas);

        // Previously value with delta fallback was converted to the delta bypass in
        // the Estimate. It can match a delta too and not disable read_estimate_deltas.
        v.mark_estimate(10);
        v.insert(10, aggregator_entry(APPLY_AGGREGATOR).unwrap());
        assert!(v.read_estimate_deltas);
    }

    #[test]
    fn estimate_logic_and_bypass_snapshot() {
        {
            let mut v = VersionedValue::new(None);

            v.insert(6, aggregator_entry(VALUE_SNAPSHOT).unwrap());
            assert_read_integer_value!(v.read(7), 13);

            v.mark_estimate(6);
            let val_no_bypass = v.versioned_map.get(&6);
            assert_some!(val_no_bypass);
            assert_matches!(
                &**val_no_bypass.unwrap(),
                AggregatorEntry::Estimate(EstimatedEntry::NoBypass)
            );
            assert_err_eq!(v.read(7), MVAggregatorsError::Dependency(6));
        }

        {
            let mut v = VersionedValue::new(None);
            v.insert(
                8,
                aggregator_entry_snapshot_value_and_delta(13, test_delta(), AggregatorID::new(2)),
            );

            assert_read_integer_value!(v.read(9), 13);

            v.mark_estimate(8);
            let snapshot_bypass = v.versioned_map.get(&8);
            assert_some!(snapshot_bypass);
            assert_matches!(
                &**snapshot_bypass.unwrap(),
                AggregatorEntry::Estimate(EstimatedEntry::Bypass(
                    AggregatorApplyChange::SnapshotDelta { .. }
                ))
            );

            assert_read_snapshot_dependent_apply!(v.read(9), 2, 8, test_delta());

            v.insert(6, aggregator_entry(VALUE_SNAPSHOT).unwrap());
            assert!(v.read_estimate_deltas);
        }

        {
            // old value shouldn't affect snapshot computation, as it depends on aggregator value.
            let mut v = VersionedValue::new(Some(AggregatorValue::Integer(3)));
            v.insert(10, aggregator_entry(APPLY_SNAPSHOT).unwrap());

            assert_read_snapshot_dependent_apply!(v.read(12), 2, 10, test_delta());
        }
    }

    #[test]
    fn estimate_logic_and_bypass_derive() {
        {
            let mut v = VersionedValue::new(None);

            v.insert(6, aggregator_entry(VALUE_DERIVED).unwrap());
            assert_read_string_value!(v.read(7), vec![70, 80, 90]);

            v.mark_estimate(6);
            let val_no_bypass = v.versioned_map.get(&6);
            assert_some!(val_no_bypass);
            assert_matches!(
                &**val_no_bypass.unwrap(),
                AggregatorEntry::Estimate(EstimatedEntry::NoBypass)
            );
            assert_err_eq!(v.read(7), MVAggregatorsError::Dependency(6));
        }

        {
            let mut v = VersionedValue::new(None);
            v.insert(
                8,
                aggregator_entry_derived_value_and_delta(
                    vec![70, 80, 90],
                    test_formula(),
                    AggregatorID::new(3),
                ),
            );

            assert_read_string_value!(v.read(10), vec![70, 80, 90]);

            v.mark_estimate(8);
            let snapshot_bypass = v.versioned_map.get(&8);
            assert_some!(snapshot_bypass);
            assert_matches!(
                &**snapshot_bypass.unwrap(),
                AggregatorEntry::Estimate(EstimatedEntry::Bypass(
                    AggregatorApplyChange::SnapshotDerived { .. }
                ))
            );

            assert_read_derived_dependent_apply!(v.read(10), 3, 9, test_formula());

            v.insert(6, aggregator_entry(VALUE_SNAPSHOT).unwrap());
            assert!(v.read_estimate_deltas);
        }

        {
            // old value shouldn't affect derived computation, as it depends on Snapshot value.
            let mut v = VersionedValue::new(Some(AggregatorValue::String(vec![80])));
            v.insert(10, aggregator_entry(APPLY_DERIVED).unwrap());

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
            v.insert(10, entry);
        }
        v.remove(10);
    }

    #[test]
    fn remove_estimate() {
        let mut v = VersionedValue::new(None);
        v.insert(3, aggregator_entry(VALUE_AGGREGATOR).unwrap());
        v.mark_estimate(3);
        v.remove(3);
        assert!(!v.read_estimate_deltas);
    }

    #[should_panic]
    #[test_case(APPLY_AGGREGATOR)]
    #[test_case(APPLY_SNAPSHOT)]
    #[test_case(APPLY_DERIVED)]
    fn insert_twice_no_value(type_index: usize) {
        let mut v = VersionedValue::new(None);
        if let Some(entry) = aggregator_entry(type_index) {
            v.insert(10, entry);
        }
        // Should fail because inserting can only overwrite an Estimate entry or
        // be inserting a Value when the transaction commits.
        if let Some(entry) = aggregator_entry(type_index) {
            v.insert(10, entry);
        }
    }

    #[should_panic]
    #[test_case(APPLY_AGGREGATOR)]
    #[test_case(APPLY_SNAPSHOT)]
    #[test_case(APPLY_DERIVED)]
    fn read_committed_not_value(type_index: usize) {
        let mut v = VersionedValue::new(None);
        if let Some(entry) = aggregator_entry(type_index) {
            v.insert(10, entry);
        }
        let _ = v.read_latest_committed_value(11);
    }

    #[should_panic]
    #[test]
    fn read_committed_estimate() {
        let mut v = VersionedValue::new(None);
        v.insert(3, aggregator_entry(VALUE_AGGREGATOR).unwrap());
        v.mark_estimate(3);
        let _ = v.read_latest_committed_value(11);
    }

    #[test]
    fn read_latest_committed_value() {
        let mut v = VersionedValue::new(Some(AggregatorValue::Integer(5)));
        v.insert(2, aggregator_entry(VALUE_AGGREGATOR).unwrap());
        v.insert(
            4,
            aggregator_entry_aggregator_value_and_delta(15, test_delta()),
        );

        assert_ok_eq!(
            v.read_latest_committed_value(5),
            AggregatorValue::Integer(15)
        );
        assert_ok_eq!(
            v.read_latest_committed_value(4),
            AggregatorValue::Integer(10)
        );
        assert_ok_eq!(
            v.read_latest_committed_value(2),
            AggregatorValue::Integer(5)
        );
    }

    #[test]
    fn read_delta_chain() {
        let mut v = VersionedValue::new(Some(AggregatorValue::Integer(5)));
        v.insert(4, aggregator_entry(APPLY_AGGREGATOR).unwrap());
        v.insert(8, aggregator_entry(APPLY_AGGREGATOR).unwrap());
        v.insert(12, aggregator_entry(APPLY_AGGREGATOR).unwrap());
        v.insert(16, aggregator_entry(APPLY_AGGREGATOR).unwrap());

        assert_read_integer_value!(v.read(0), 5);
        assert_read_integer_value!(v.read(5), 35);
        assert_read_integer_value!(v.read(9), 65);
        assert_read_integer_value!(v.read(13), 95);
        assert_read_integer_value!(v.read(17), 125);
    }

    #[test]
    fn read_errors() {
        let mut v = VersionedValue::new(None);
        v.insert(2, aggregator_entry(VALUE_AGGREGATOR).unwrap());

        assert_err_eq!(v.read(1), MVAggregatorsError::NotFound);

        v.insert(
            8,
            AggregatorEntry::Apply(AggregatorApplyChange::AggregatorDelta {
                delta: negative_delta(),
            }),
        );
        assert_err_eq!(v.read(9), MVAggregatorsError::DeltaApplicationFailure);
        // Ensure without underflow there would not be a failure.

        v.insert(4, aggregator_entry(APPLY_AGGREGATOR).unwrap()); // adds 30.
        assert_read_integer_value!(v.read(9), 10);

        v.insert(
            6,
            AggregatorEntry::Value(AggregatorValue::Integer(35), None),
        );
        assert_read_integer_value!(v.read(9), 5);

        v.mark_estimate(2);
        assert_err_eq!(v.read(3), MVAggregatorsError::Dependency(2));
    }

    // #[test]
    // fn applicable_bypass_test() {
    //     use EstimatedEntry::*;

    //     let mut v = VersionedValue::new(None);
    //     let delta_bypass = DeltaBypass(test_delta());
    //     let snapshot_bypass = SnapshotBypass(AggregatorID::new(5), negative_delta());

    //     assert_eq!(v.applicable_bypass(&NoBypass), NoBypass);
    //     assert_eq!(v.applicable_bypass(&delta_bypass), delta_bypass);
    //     assert_eq!(v.applicable_bypass(&snapshot_bypass), snapshot_bypass);

    //     v.read_estimate_deltas = false;
    //     assert_eq!(v.applicable_bypass(&NoBypass), NoBypass);
    //     assert_eq!(v.applicable_bypass(&delta_bypass), NoBypass);
    //     assert_eq!(v.applicable_bypass(&snapshot_bypass), NoBypass);
    // }
}
