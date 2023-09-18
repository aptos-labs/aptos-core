// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::types::{AtomicTxnIndex, MVAggregatorsError, TxnIndex};
use aptos_aggregator::{delta_change_set::DeltaOp, types::AggregatorID};
use claims::{assert_matches, assert_none};
use crossbeam::utils::CachePadded;
use dashmap::DashMap;
use std::{
    collections::btree_map::{BTreeMap, Entry},
    iter::DoubleEndedIterator,
    sync::atomic::Ordering,
};

// When an AggregatorEntry (see below) is transformed to an Estimate, internally we store
// a potential bypass (based on the previously stored entry), which may allow a read
// operation to not wait for the corresponding dependency.
#[derive(Clone, Copy, Debug, PartialEq)]
enum EstimatedEntry {
    NoBypass,
    // If applicable, can bypass the Estimate by considering a Delta instead.
    DeltaBypass(DeltaOp),
    // If applicable, can bypass the Estimate by considering a Snapshot instead.
    SnapshotBypass(AggregatorID, DeltaOp),
}

// There is no explicit deletion as it will be impossible to resolve the ID of a deleted
// aggregator (as the ID will not be contained in a resource anymore). The write on the
// previously holding resource and Block-STM read validation ensures correctness.
#[derive(Debug)]
enum AggregatorEntry {
    // If the value is determined by a delta applied to a value read during the execution
    // of the same transaction, the delta may also be kept in the entry. This is useful
    // if speculative execution aborts and the entry is marked as estimate, as the delta
    // may be used to avoid waiting on the Estimate entry. More in comments below.
    Value(u128, Option<DeltaOp>),
    Delta(DeltaOp),
    // Applies the delta on top of a 'snapshot' of an aggregator with a given ID (at
    // a corresponding, i.e. one less than where the given entry is stored, version).
    Snapshot(AggregatorID, DeltaOp),
    // Marks the entry as an estimate, indicating that the next incarnation of the
    // transaction is estimated to populate the entry. May contain a bypass internally
    // (allowing a read operation to avoid waiting for the corresponding dependency),
    // encapulated by the EstimatedEntry.
    Estimate(EstimatedEntry),
}

// A VersionedValue internally contains a BTreeMap from indices of transactions
// that update a given aggregator, alongside the corresponding entries.
#[derive(Debug)]
struct VersionedValue {
    versioned_map: BTreeMap<TxnIndex, CachePadded<AggregatorEntry>>,

    // The value of the given aggregator prior to the block execution. None implies that
    // the aggregator did not exist prior to the block.
    base_value: Option<u128>,

    // If true, the reads can proceed by using deltas in Estimate entries, if present.
    // The value is optimistically initialized to true, but changed to false when it is
    // observed that a later incarnation changed the value of a delta at the same entry.
    read_estimate_deltas: bool,
}

#[derive(Debug, PartialEq)]
enum VersionedRead {
    Value(u128),
    // The transaction index records the index at which the Snapshot was encountered.
    // This is required for the caller to resolve the value of the aggregator (with the
    // recorded id) from which the snapshot was created at the correct version (index).
    Snapshot(AggregatorID, TxnIndex, DeltaOp),
}

impl VersionedValue {
    // VersionedValue should only be created when base value of the corresponding aggregator
    // is known & provided to the constructor.
    fn new(base_value: Option<u128>) -> Self {
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
                    Value(_, maybe_delta) => maybe_delta.map_or(NoBypass, DeltaBypass),
                    Delta(delta) => DeltaBypass(*delta),
                    Snapshot(id, delta) => SnapshotBypass(*id, *delta),
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

    fn insert(&mut self, txn_idx: TxnIndex, entry: AggregatorEntry) {
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
                    (Estimate(SnapshotBypass(id_l, delta_l)), Snapshot(id_r, delta_r)) => {
                        *id_l == *id_r && *delta_l == *delta_r
                    },
                    (Estimate(DeltaBypass(delta_l)), Delta(delta_r) | Value(_, Some(delta_r))) => {
                        *delta_l == *delta_r
                    },
                    // Deltas appear only for Aggregators, while Snapshots appear only
                    // for AggregatorSnapshots.
                    (Estimate(DeltaBypass(_)), Snapshot(_, _)) => unreachable!(
                        "Storing snapshot for aggregator ID \
			 that previously contained a delta"
                    ),
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
                    (Value(_, None), Delta(_)) => true,

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
    ) -> Result<u128, MVAggregatorsError> {
        use AggregatorEntry::*;

        self.versioned_map
            .range(0..next_idx_to_commit)
            .next_back()
            .map_or(
                self.base_value.ok_or(MVAggregatorsError::NotFound),
                |(_, entry)| match &**entry {
                    Value(v, _) => Ok(*v),
                    Snapshot(_, _) | Delta(_) => unreachable!(
                        "Snapshot or Delta entries may not exist for committed txn indices"
                    ),
                    Estimate(_) => unreachable!("Committed entry may not be an Estimate"),
                },
            )
    }

    // Gate the estimate bypass logic by whether that functionality is enabled
    // (read_estimate_deltas flag) for the aggregator.
    fn applicable_bypass(&self, bypass: &EstimatedEntry) -> EstimatedEntry {
        if self.read_estimate_deltas {
            *bypass
        } else {
            EstimatedEntry::NoBypass
        }
    }

    // Traverse down from txn_idx and accumulate encountered deltas until resolving it to
    // a value or return an error. Errors of not finding a value to resolve to take
    // precedence over a DeltaApplicationError.
    fn apply_delta_suffix(
        &self,
        iter: &mut dyn DoubleEndedIterator<Item = (&TxnIndex, &CachePadded<AggregatorEntry>)>,
        delta: DeltaOp,
    ) -> Result<u128, MVAggregatorsError> {
        use AggregatorEntry::*;
        use EstimatedEntry::*;

        let mut accumulator = delta;
        while let Some((idx, entry)) = iter.next_back() {
            let delta = match &**entry {
                Value(v, _) => {
                    // Apply accumulated delta to resolve the aggregator value.
                    return accumulator
                        .apply_to(*v)
                        .map_err(|_| MVAggregatorsError::DeltaApplicationFailure);
                },
                Delta(delta) => *delta,
                Snapshot(_, _) => {
                    unreachable!("Snapshots and Deltas may not exist for the same Aggregator ID")
                },
                Estimate(EstimatedEntry::SnapshotBypass(_, _)) => {
                    unreachable!("Bypass previously stored for Aggregator ID that contains deltas")
                },
                Estimate(bypass) => match self.applicable_bypass(bypass) {
                    NoBypass => {
                        // We must wait on Estimates, or a bypass isn't available.
                        return Err(MVAggregatorsError::Dependency(*idx));
                    },
                    DeltaBypass(delta) => delta,
                    SnapshotBypass(_, _) => unreachable!(
                        "Snapshot bypass previously stored for an \
			     Aggregator ID that contains deltas"
                    ),
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
            .ok_or(MVAggregatorsError::NotFound)
            .and_then(|base_value| {
                accumulator
                    .apply_to(base_value)
                    .map_err(|_| MVAggregatorsError::DeltaApplicationFailure)
            })
    }

    // Reads a given aggregator value at a given version (transaction index) and produces
    // a ReadResult if successful, which is either a u128 value, or a snapshot specifying
    // a different aggregator (with ID) at a given version and a delta to apply on top.
    fn read(&self, txn_idx: TxnIndex) -> Result<VersionedRead, MVAggregatorsError> {
        use AggregatorEntry::*;
        use EstimatedEntry::*;
        use MVAggregatorsError::*;

        let mut iter = self.versioned_map.range(0..txn_idx);

        iter.next_back().map_or(
            // No entries in versioned map, use base value.
            self.base_value.ok_or(NotFound).map(VersionedRead::Value),
            // Consider the latest entry below the provided version.
            |(idx, entry)| match &**entry {
                Value(v, _) => Ok(VersionedRead::Value(*v)),
                // If read encounters a delta, it must traverse the block of transactions
                // (top-down) until it encounters a value or use the base value.
                Delta(delta) => self
                    .apply_delta_suffix(&mut iter, *delta)
                    .map(VersionedRead::Value),
                Snapshot(id, delta) => Ok(VersionedRead::Snapshot(*id, *idx, *delta)),
                Estimate(bypass) => match self.applicable_bypass(bypass) {
                    DeltaBypass(delta) => self
                        .apply_delta_suffix(&mut iter, delta)
                        .map(VersionedRead::Value),
                    SnapshotBypass(id, delta) => Ok(VersionedRead::Snapshot(id, *idx, delta)),
                    NoBypass => Err(Dependency(*idx)),
                },
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
pub struct VersionedAggregators {
    values: DashMap<AggregatorID, VersionedValue>,

    /// No deltas are allowed below next_idx_to_commit version, as all deltas (and snapshots)
    /// must be materialized and converted to Values during commit.
    next_idx_to_commit: AtomicTxnIndex,
}

impl VersionedAggregators {
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
    pub fn set_base_value(&self, id: AggregatorID, base_value: u128) {
        self.values
            .entry(id)
            .or_insert(VersionedValue::new(Some(base_value)));
    }

    /// Must be called when an aggregator creation with a given ID and initial value is observed
    /// in the outputs of txn_idx.
    pub fn create_aggregator(&self, id: AggregatorID, txn_idx: TxnIndex, value: u128) {
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

    pub fn read(&self, id: AggregatorID, txn_idx: TxnIndex) -> Result<u128, MVAggregatorsError> {
        let read_res = self
            .values
            .get(&id)
            .expect("VersionedValue for an (resolved) ID must already exist")
            .read(txn_idx)?;
        // The lock on id is out of scope.

        match read_res {
            VersionedRead::Value(v) => Ok(v),
            VersionedRead::Snapshot(source_id, source_idx, delta) => {
                // Read the source aggregator of snapshot.

                self.values
                    .get(&source_id)
                    .expect("VersionedValue for an (resolved) ID must already exist")
                    .read(source_idx)
                    .and_then(|source_r| match source_r {
                        VersionedRead::Value(source_v) => delta
                            .apply_to(source_v)
                            .map_err(|_| MVAggregatorsError::DeltaApplicationFailure),
                        VersionedRead::Snapshot(_, _, _) => {
                            unreachable!("Snapshot in source aggregator of AggregatorSnapshot")
                        },
                    })
            },
        }
    }

    /// This method is intended to be called during transaction execution (e.g. for getting
    /// a rough value of an aggregator cheaply for branch prediction). Hence, the 'calling'
    /// transaction may not be committed yet, and there is no reason to provide txn_idx.
    pub fn read_latest_committed_value(
        &self,
        id: AggregatorID,
    ) -> Result<u128, MVAggregatorsError> {
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
        id: AggregatorID,
        txn_idx: TxnIndex,
        value: u128,
        maybe_delta: Option<DeltaOp>,
    ) {
        self.values
            .get_mut(&id)
            .expect("VersionedValue for an (resolved) ID must already exist")
            .insert(txn_idx, AggregatorEntry::Value(value, maybe_delta));
    }

    pub fn record_delta(&self, id: AggregatorID, txn_idx: TxnIndex, delta: DeltaOp) {
        self.values
            .get_mut(&id)
            .expect("VersionedValue for an (resolved) ID must already exist")
            .insert(txn_idx, AggregatorEntry::Delta(delta));
    }

    pub fn record_snapshot(
        &self,
        id: AggregatorID,
        txn_idx: TxnIndex,
        source_id: AggregatorID,
        delta: DeltaOp,
    ) {
        self.values
            .get_mut(&id)
            .expect("VersionedValue for an (resolved) ID must already exist")
            .insert(txn_idx, AggregatorEntry::Snapshot(source_id, delta));
    }

    pub fn mark_estimate(&self, id: AggregatorID, txn_idx: TxnIndex) {
        self.values
            .get_mut(&id)
            .expect("VersionedValue for an (resolved) ID must already exist")
            .mark_estimate(txn_idx);
    }

    pub fn remove(&self, id: AggregatorID, txn_idx: TxnIndex) {
        self.values
            .get_mut(&id)
            .expect("VersionedValue for an (resolved) ID must already exist")
            .remove(txn_idx);
    }

    pub fn update_committed_idx(&self, committed_idx: TxnIndex) {
        self.next_idx_to_commit
            .fetch_max(committed_idx + 1, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use aptos_aggregator::{
        bounded_math::SignedU128, delta_change_set::DeltaOp, delta_math::DeltaHistory,
    };
    use claims::{assert_err_eq, assert_ok_eq, assert_some};
    use test_case::test_case;

    // Different type acronyms used for generating different test cases.
    const NO_ENTRY: usize = 0;
    const VALUE: usize = 1;
    const DELTA: usize = 2;
    const SNAPSHOT: usize = 3;
    const ESTIMATE: usize = 4;

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

    fn aggregator_entry(type_index: usize) -> Option<AggregatorEntry> {
        match type_index {
            NO_ENTRY => None,
            VALUE => Some(AggregatorEntry::Value(10, None)),
            DELTA => Some(AggregatorEntry::Delta(test_delta())),
            SNAPSHOT => Some(AggregatorEntry::Snapshot(
                AggregatorID::new(2),
                test_delta(),
            )),
            ESTIMATE => Some(AggregatorEntry::Estimate(EstimatedEntry::NoBypass)),
            _ => unreachable!("Wrong type index in test"),
        }
    }

    #[should_panic]
    #[test_case(NO_ENTRY)]
    #[test_case(VALUE)]
    #[test_case(DELTA)]
    #[test_case(SNAPSHOT)]
    #[test_case(ESTIMATE)]
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
        v.insert(3, aggregator_entry(VALUE).unwrap());
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
        v.insert(3, aggregator_entry(ESTIMATE).unwrap());
    }

    #[test]
    fn estimate_bypass() {
        let mut v = VersionedValue::new(None);
        v.insert(2, aggregator_entry(VALUE).unwrap());
        v.insert(3, AggregatorEntry::Value(15, Some(test_delta())));
        v.insert(4, aggregator_entry(DELTA).unwrap());
        v.insert(6, aggregator_entry(SNAPSHOT).unwrap());
        v.insert(10, AggregatorEntry::Value(15, Some(test_delta())));

        // Delta + Value(15)
        assert_ok_eq!(v.read(5), VersionedRead::Value(45));

        v.mark_estimate(3);
        let val_bypass = v.versioned_map.get(&3);
        assert_some!(val_bypass);
        assert_matches!(
            &**val_bypass.unwrap(),
            AggregatorEntry::Estimate(EstimatedEntry::DeltaBypass(_))
        );
        // Delta(30) + Value delta bypass(30) + Value(10)
        assert_ok_eq!(v.read(5), VersionedRead::Value(70));

        v.mark_estimate(4);
        let delta_bypass = v.versioned_map.get(&4);
        assert_some!(delta_bypass);
        assert_matches!(
            &**delta_bypass.unwrap(),
            AggregatorEntry::Estimate(EstimatedEntry::DeltaBypass(_))
        );
        // Delta bypass(30) + Value delta bypass(30) + Value(10)
        assert_ok_eq!(v.read(5), VersionedRead::Value(70));

        v.mark_estimate(2);
        let val_no_bypass = v.versioned_map.get(&2);
        assert_some!(val_no_bypass);
        assert_matches!(
            &**val_no_bypass.unwrap(),
            AggregatorEntry::Estimate(EstimatedEntry::NoBypass)
        );
        assert_err_eq!(v.read(6), MVAggregatorsError::Dependency(2));

        v.mark_estimate(6);
        let snapshot_bypass = v.versioned_map.get(&6);
        assert_some!(snapshot_bypass);
        assert_matches!(
            &**snapshot_bypass.unwrap(),
            AggregatorEntry::Estimate(EstimatedEntry::SnapshotBypass(_, _))
        );
        assert_ok_eq!(
            v.read(8),
            VersionedRead::Snapshot(AggregatorID::new(2), 6, test_delta())
        );

        // Next, ensure read_estimate_deltas remains true if entries are overwritten
        // with matching deltas. Check at each point to not rely on the invariant that
        // read_estimate_deltas can only become false from true.
        v.insert(2, aggregator_entry(VALUE).unwrap());
        assert!(v.read_estimate_deltas);
        v.insert(3, AggregatorEntry::Value(15, Some(test_delta())));
        assert!(v.read_estimate_deltas);
        v.insert(4, aggregator_entry(DELTA).unwrap());
        assert!(v.read_estimate_deltas);
        v.insert(6, aggregator_entry(SNAPSHOT).unwrap());
        assert!(v.read_estimate_deltas);

        // Previously value with delta fallback was converted to the delta bypass in
        // the Estimate. It can match a delta too and not disable read_estimate_deltas.
        v.mark_estimate(10);
        v.insert(10, aggregator_entry(DELTA).unwrap());
        assert!(v.read_estimate_deltas);
    }

    #[should_panic]
    #[test_case(NO_ENTRY)]
    #[test_case(VALUE)]
    #[test_case(DELTA)]
    #[test_case(SNAPSHOT)]
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
        v.insert(3, aggregator_entry(VALUE).unwrap());
        v.mark_estimate(3);
        v.remove(3);
        assert!(!v.read_estimate_deltas);
    }

    #[should_panic]
    #[test_case(DELTA)]
    #[test_case(SNAPSHOT)]
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
    #[test_case(DELTA)]
    #[test_case(SNAPSHOT)]
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
        v.insert(3, aggregator_entry(VALUE).unwrap());
        v.mark_estimate(3);
        let _ = v.read_latest_committed_value(11);
    }

    #[test]
    fn read_latest_committed_value() {
        let mut v = VersionedValue::new(Some(5));
        v.insert(2, aggregator_entry(VALUE).unwrap());
        v.insert(4, AggregatorEntry::Value(15, Some(test_delta())));

        assert_ok_eq!(v.read_latest_committed_value(5), 15);
        assert_ok_eq!(v.read_latest_committed_value(4), 10);
        assert_ok_eq!(v.read_latest_committed_value(2), 5);
    }

    #[test]
    fn read_delta_chain() {
        let mut v = VersionedValue::new(Some(5));
        v.insert(4, aggregator_entry(DELTA).unwrap());
        v.insert(8, aggregator_entry(DELTA).unwrap());
        v.insert(12, aggregator_entry(DELTA).unwrap());
        v.insert(16, aggregator_entry(DELTA).unwrap());

        assert_ok_eq!(v.read(0), VersionedRead::Value(5));
        assert_ok_eq!(v.read(5), VersionedRead::Value(35));
        assert_ok_eq!(v.read(9), VersionedRead::Value(65));
        assert_ok_eq!(v.read(13), VersionedRead::Value(95));
        assert_ok_eq!(v.read(17), VersionedRead::Value(125));
    }

    #[test]
    fn read_errors() {
        let mut v = VersionedValue::new(None);
        v.insert(2, aggregator_entry(VALUE).unwrap());

        assert_err_eq!(v.read(1), MVAggregatorsError::NotFound);

        v.insert(8, AggregatorEntry::Delta(negative_delta()));
        assert_err_eq!(v.read(9), MVAggregatorsError::DeltaApplicationFailure);
        // Ensure without underflow there would not be a failure.

        v.insert(4, aggregator_entry(DELTA).unwrap()); // adds 30.
        assert_ok_eq!(v.read(9), VersionedRead::Value(10));

        v.insert(6, AggregatorEntry::Value(35, None));
        assert_ok_eq!(v.read(9), VersionedRead::Value(5));

        v.mark_estimate(2);
        assert_err_eq!(v.read(3), MVAggregatorsError::Dependency(2));
    }

    #[test]
    fn applicable_bypass_test() {
        use EstimatedEntry::*;

        let mut v = VersionedValue::new(None);
        let delta_bypass = DeltaBypass(test_delta());
        let snapshot_bypass = SnapshotBypass(AggregatorID::new(5), negative_delta());

        assert_eq!(v.applicable_bypass(&NoBypass), NoBypass);
        assert_eq!(v.applicable_bypass(&delta_bypass), delta_bypass);
        assert_eq!(v.applicable_bypass(&snapshot_bypass), snapshot_bypass);

        v.read_estimate_deltas = false;
        assert_eq!(v.applicable_bypass(&NoBypass), NoBypass);
        assert_eq!(v.applicable_bypass(&delta_bypass), NoBypass);
        assert_eq!(v.applicable_bypass(&snapshot_bypass), NoBypass);
    }
}
