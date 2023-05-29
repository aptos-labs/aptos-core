// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::check_change_set::CheckChangeSet;
use aptos_aggregator::delta_change_set::{deserialize, serialize, DeltaChangeSet};
use aptos_state_view::StateView;
use aptos_types::{
    contract_event::ContractEvent,
    write_set::{WriteOp, WriteSet},
};
use move_binary_format::errors::Location;
use move_core_types::vm_status::{err_msg, StatusCode, VMStatus};
use std::collections::btree_map::Entry::{Occupied, Vacant};

/// A change set produced by the VM. Just like VMOutput, this type should
/// be used inside the VM. For storage backends, use ChangeSet.
#[derive(Debug, Clone)]
pub struct VMChangeSet {
    write_set: WriteSet,
    delta_change_set: DeltaChangeSet,
    events: Vec<ContractEvent>,
}

impl VMChangeSet {
    /// Returns an empty change set.
    pub fn empty() -> Self {
        Self {
            write_set: WriteSet::default(),
            delta_change_set: DeltaChangeSet::empty(),
            events: vec![],
        }
    }

    /// Returns a new change set, and checks that it is well-formed.
    pub fn new(
        write_set: WriteSet,
        delta_change_set: DeltaChangeSet,
        events: Vec<ContractEvent>,
        checker: &dyn CheckChangeSet,
    ) -> anyhow::Result<Self, VMStatus> {
        // Check that writes and deltas have disjoint key set.
        let disjoint = delta_change_set
            .iter()
            .all(|(k, _)| write_set.get(k).is_none());
        if !disjoint {
            return Err(VMStatus::Error(
                StatusCode::DATA_FORMAT_ERROR,
                err_msg("DeltaChangeSet and WriteSet are not disjoint."),
            ));
        }

        let change_set = Self {
            write_set,
            delta_change_set,
            events,
        };

        // Check the newly-formed change set.
        checker.check_change_set(&change_set)?;
        Ok(change_set)
    }

    pub fn write_set(&self) -> &WriteSet {
        &self.write_set
    }

    pub fn delta_change_set(&self) -> &DeltaChangeSet {
        &self.delta_change_set
    }

    pub fn events(&self) -> &[ContractEvent] {
        &self.events
    }

    pub fn unpack(self) -> (WriteSet, DeltaChangeSet, Vec<ContractEvent>) {
        (self.write_set, self.delta_change_set, self.events)
    }

    /// Materializes this change set: all deltas are converted into writes and
    /// are combined with existing write set. In case of materialization fails,
    /// an error is returned.
    pub fn try_materialize(self, state_view: &impl StateView) -> anyhow::Result<Self, VMStatus> {
        let (write_set, delta_change_set, events) = self.unpack();

        // Try to materialize deltas and add them to the write set.
        let mut write_set_mut = write_set.into_mut();
        let delta_writes = delta_change_set.take_materialized(state_view)?;
        delta_writes
            .into_iter()
            .for_each(|item| write_set_mut.insert(item));

        let write_set = write_set_mut.freeze().map_err(|_| {
            VMStatus::Error(
                StatusCode::DATA_FORMAT_ERROR,
                err_msg("Failed to freeze write set when converting VMOutput to TransactionOutput"),
            )
        })?;

        Ok(Self {
            write_set,
            delta_change_set: DeltaChangeSet::empty(),
            events,
        })
    }

    /// Squashes `next` change set on top of this change set. The squashed
    /// change set is then checked using the `checker`.
    pub fn squash(
        self,
        next: Self,
        checker: &dyn CheckChangeSet,
    ) -> anyhow::Result<Self, VMStatus> {
        use WriteOp::*;

        // First, obtain write sets, delta change sets and events of this and other
        // change sets.
        let (next_write_set, next_delta_change_set, next_events) = next.unpack();
        let (write_set, mut delta_change_set, mut events) = self.unpack();
        let mut write_set_mut = write_set.into_mut();

        // We are modifying current sets, so grab a mutable reference for them.
        let delta_ops = delta_change_set.as_inner_mut();
        let write_ops = write_set_mut.as_inner_mut();

        // First, squash incoming deltas.
        for (key, next_delta_op) in next_delta_change_set.into_iter() {
            if let Some(write_op) = write_ops.get_mut(&key) {
                // In this case, delta follows a write op.
                match write_op {
                    Creation(data)
                    | Modification(data)
                    | CreationWithMetadata { data, .. }
                    | ModificationWithMetadata { data, .. } => {
                        // Apply delta on top of creation or modification.
                        let base: u128 = deserialize(data);
                        let value = next_delta_op
                            .apply_to(base)
                            .map_err(|e| e.finish(Location::Undefined).into_vm_status())?;
                        *data = serialize(&value);
                    },
                    Deletion | DeletionWithMetadata { .. } => {
                        // This case (applying a delta to deleted item) should
                        // never happen. Let's still return an error instead of
                        // panicking.
                        return Err(VMStatus::Error(
                            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                            err_msg("Cannot squash delta which was already deleted."),
                        ));
                    },
                }
            } else {
                // Otherwise, this is a either a new delta or an additional delta
                // for the same state key.
                match delta_ops.entry(key) {
                    Occupied(entry) => {
                        // In this case, we need to merge the new incoming delta
                        // to the existing delta, ensuring the strict ordering.
                        entry
                            .into_mut()
                            .merge_with_next_delta(next_delta_op)
                            .map_err(|e| e.finish(Location::Undefined).into_vm_status())?;
                    },
                    Vacant(entry) => {
                        // We see this delta for the first time, so simply add it
                        // to the set.
                        entry.insert(next_delta_op);
                    },
                }
            }
        }

        // Next, squash write ops.
        for (key, next_write_op) in next_write_set.into_iter() {
            match write_ops.entry(key) {
                Occupied(mut entry) => {
                    // Squashing creation and deletion is a no-op. In that case, we
                    // have to remove the old write op from the write set.
                    let noop = !WriteOp::squash(entry.get_mut(), next_write_op).map_err(|e| {
                        VMStatus::Error(
                            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                            err_msg(format!("Error while squashing two write ops: {}.", e)),
                        )
                    })?;
                    if noop {
                        entry.remove();
                    }
                },
                Vacant(entry) => {
                    // This is a new write op. It can overwrite a delta so we
                    // have to make sure we remove such a delta from the set in
                    // this case.
                    let removed_delta = delta_change_set.remove(entry.key());

                    // We cannot create after modification with a delta!
                    if removed_delta.is_some() && next_write_op.is_creation() {
                        return Err(VMStatus::Error(
                            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                            err_msg("Cannot create a resource after modification with a delta."),
                        ));
                    }

                    entry.insert(next_write_op);
                },
            }
        }

        let write_set = write_set_mut.freeze().map_err(|_| {
            VMStatus::Error(
                StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                err_msg("Error when freezing squashed write sets."),
            )
        })?;

        // Squash events.
        events.extend(next_events);

        Self::new(write_set, delta_change_set, events, checker)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_aggregator::delta_change_set::{delta_add, DeltaOp};
    use aptos_types::{state_store::state_key::StateKey, write_set::WriteSetMut};
    use claims::{assert_matches, assert_ok};

    /// A mock for testing. Always succeeds on checking a change set.
    struct NoOpChangeSetChecker;

    impl CheckChangeSet for NoOpChangeSetChecker {
        fn check_change_set(&self, _change_set: &VMChangeSet) -> anyhow::Result<(), VMStatus> {
            Ok(())
        }
    }

    // Test specification for all possible combinations.
    // +--------------+----------------+----------------+-----------------+
    // |   state key  |  change set 1  |  change set 2  |    squashed     |
    // +--------------+----------------+----------------+-----------------+
    // |      0       |    create 0    |                |    create 0     |
    // |      1       |    modify 1    |                |    modify 1     |
    // |      2       |    delete      |                |    delete       |
    // |      3       |                |    create 103  |    create 103   |
    // |      4       |                |    modify 104  |    modify 104   |
    // |      5       |                |    delete      |    delete       |
    // |      6       |    create 6    |    create 106  |    ERROR        |
    // |      7       |    create 7    |    modify 107  |    create 107   |
    // |      8       |    create 8    |    delete      |                 |
    // |      9       |    modify 9    |    create 109  |    ERROR        |
    // |      10      |    modify 10   |    modify 110  |    modify 110   |
    // |      11      |    modify 11   |    delete      |    delete       |
    // |      12      |    delete      |    create 112  |    modify 112   |
    // |      13      |    delete      |    modify 113  |    ERROR        |
    // |      14      |    delete      |    delete      |    ERROR        |
    // +--------------+----------------+----------------+-----------------+
    // |      15      |    +15         |                |    +15          |
    // |      16      |                |    +116        |    +116         |
    // |      17      |    +17         |    +117        |    +134         |
    // +--------------+----------------+----------------+-----------------+
    // |      18      |    create 18   |    +118        |    create 136   |
    // |      19      |    modify 19   |    +119        |    modify 138   |
    // |      20      |    delete      |    +120        |    ERROR        |
    // +--------------+----------------+----------------+-----------------+
    // |      21      |    +21         |    create 121  |    ERROR        |
    // |      22      |    +22         |    modify 122  |    modify 122   |
    // |      23      |    +23         |    delete      |    delete       |
    // +--------------+----------------+----------------+-----------------+

    fn key(id: u128) -> StateKey {
        StateKey::raw(format!("key-{}", id).into_bytes())
    }

    fn create(value: u128) -> WriteOp {
        WriteOp::Creation(serialize(&value))
    }

    fn modify(value: u128) -> WriteOp {
        WriteOp::Modification(serialize(&value))
    }

    fn delete() -> WriteOp {
        WriteOp::Deletion
    }

    fn add(value: u128) -> DeltaOp {
        // Limit doesn't matter here, so set it to be relatively high.
        delta_add(value, 100000)
    }

    fn get_write_op(change_set: &VMChangeSet, id: u128) -> WriteOp {
        change_set.write_set.get(&key(id)).unwrap().clone()
    }

    fn contains_write_op(change_set: &VMChangeSet, id: u128) -> bool {
        change_set.write_set.get(&key(id)).is_some()
    }

    fn get_delta_op(change_set: &VMChangeSet, id: u128) -> DeltaOp {
        *change_set.delta_change_set.get(&key(id)).unwrap()
    }

    fn contains_delta_op(change_set: &VMChangeSet, id: u128) -> bool {
        change_set.delta_change_set.get(&key(id)).is_some()
    }

    fn build_change_set(ws: WriteSet, ds: DeltaChangeSet) -> VMChangeSet {
        VMChangeSet::new(ws, ds, vec![], &NoOpChangeSetChecker).unwrap()
    }

    fn build_change_sets_for_test() -> (VMChangeSet, VMChangeSet) {
        // Create write sets and delta change sets.
        let mut write_set_1 = WriteSetMut::default();
        let mut write_set_2 = WriteSetMut::default();
        let mut delta_change_set_1 = DeltaChangeSet::empty();
        let mut delta_change_set_2 = DeltaChangeSet::empty();

        // Populate sets according to the spec. Skip keys which lead to
        // errors because we test them separately.
        write_set_1.insert((key(0), create(0)));
        write_set_1.insert((key(1), modify(1)));
        write_set_1.insert((key(2), delete()));
        write_set_2.insert((key(3), create(103)));
        write_set_2.insert((key(4), modify(104)));
        write_set_2.insert((key(5), delete()));

        write_set_1.insert((key(7), create(7)));
        write_set_2.insert((key(7), modify(107)));
        write_set_1.insert((key(8), create(8)));
        write_set_2.insert((key(8), delete()));

        write_set_1.insert((key(10), modify(10)));
        write_set_2.insert((key(10), modify(110)));
        write_set_1.insert((key(11), modify(111)));
        write_set_2.insert((key(11), delete()));
        write_set_1.insert((key(12), delete()));
        write_set_2.insert((key(12), create(112)));

        delta_change_set_1.insert((key(15), add(15)));
        delta_change_set_2.insert((key(16), add(116)));
        delta_change_set_1.insert((key(17), add(17)));
        delta_change_set_2.insert((key(17), add(117)));
        write_set_1.insert((key(18), create(18)));
        delta_change_set_2.insert((key(18), add(118)));
        write_set_1.insert((key(19), modify(19)));
        delta_change_set_2.insert((key(19), add(119)));

        delta_change_set_1.insert((key(22), add(22)));
        write_set_2.insert((key(22), modify(122)));
        delta_change_set_1.insert((key(23), add(23)));
        write_set_2.insert((key(23), delete()));

        let write_set_1 = write_set_1.freeze().unwrap();
        let write_set_2 = write_set_2.freeze().unwrap();
        (
            build_change_set(write_set_1, delta_change_set_1),
            build_change_set(write_set_2, delta_change_set_2),
        )
    }

    #[test]
    fn test_successful_squash() {
        let (change_set_1, change_set_2) = build_change_sets_for_test();

        // Check squash is indeed successful.
        let res = change_set_1.squash(change_set_2, &NoOpChangeSetChecker);
        assert_ok!(res.clone());
        let change_set = res.unwrap();

        // create 0 + ___ = create 0
        assert_eq!(get_write_op(&change_set, 0), create(0));
        assert!(!contains_delta_op(&change_set, 0));

        // modify 1 + ___ = modify 1
        assert_eq!(get_write_op(&change_set, 1), modify(1));
        assert!(!contains_delta_op(&change_set, 1));

        // delete + ___ = delete
        assert_eq!(get_write_op(&change_set, 2), delete());
        assert!(!contains_delta_op(&change_set, 2));

        // ___ + create 103 = create 103
        assert_eq!(get_write_op(&change_set, 3), create(103));
        assert!(!contains_delta_op(&change_set, 3));

        // ___ + modify 103 = modify 103
        assert_eq!(get_write_op(&change_set, 4), modify(104));
        assert!(!contains_delta_op(&change_set, 4));

        // ___ + delete = delete
        assert_eq!(get_write_op(&change_set, 5), delete());
        assert!(!contains_delta_op(&change_set, 5));

        // create 7 + modify 107 = create 107
        assert_eq!(get_write_op(&change_set, 7), create(107));
        assert!(!contains_delta_op(&change_set, 7));

        // create 8 + delete = ___
        assert!(!contains_write_op(&change_set, 8));
        assert!(!contains_delta_op(&change_set, 8));

        // modify 10 + modify 110 = modify 110
        assert_eq!(get_write_op(&change_set, 10), modify(110));
        assert!(!contains_delta_op(&change_set, 10));

        // modify 10 + delete = delete
        assert_eq!(get_write_op(&change_set, 11), delete());
        assert!(!contains_delta_op(&change_set, 11));

        // delete + create 112 = create 112
        assert_eq!(get_write_op(&change_set, 12), modify(112));
        assert!(!contains_delta_op(&change_set, 12));

        // +15 + ___ = +15
        assert!(!contains_write_op(&change_set, 15));
        assert_eq!(get_delta_op(&change_set, 15), add(15));

        // ___ + +116 = +116
        assert!(!contains_write_op(&change_set, 16));
        assert_eq!(get_delta_op(&change_set, 16), add(116));

        // +17 + +117 = +134
        assert!(!contains_write_op(&change_set, 17));
        assert_eq!(get_delta_op(&change_set, 17), add(134));

        // create 18 + +118 = create 136
        assert_eq!(get_write_op(&change_set, 18), create(136));
        assert!(!contains_delta_op(&change_set, 18));

        // modify 19 + +119 = modify 138
        assert_eq!(get_write_op(&change_set, 19), modify(138));
        assert!(!contains_delta_op(&change_set, 19));

        // +22 + modify 122 = modify 122
        assert_eq!(get_write_op(&change_set, 22), modify(122));
        assert!(!contains_delta_op(&change_set, 22));

        // +23 + delete = delete
        assert_eq!(get_write_op(&change_set, 23), delete());
        assert!(!contains_delta_op(&change_set, 23));
    }

    #[test]
    fn test_unsuccessful_squash_1() {
        let mut write_set_1 = WriteSetMut::default();
        let mut write_set_2 = WriteSetMut::default();

        // create 6 + create 106 throws an error
        write_set_1.insert((key(6), create(6)));
        write_set_2.insert((key(6), create(106)));

        let write_set_1 = write_set_1.freeze().unwrap();
        let write_set_2 = write_set_2.freeze().unwrap();

        let change_set_1 = build_change_set(write_set_1, DeltaChangeSet::empty());
        let change_set_2 = build_change_set(write_set_2, DeltaChangeSet::empty());
        let res = change_set_1.squash(change_set_2, &NoOpChangeSetChecker);
        assert_matches!(
            res,
            Err(VMStatus::Error(
                StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                Some(_),
            ))
        );
    }

    #[test]
    fn test_unsuccessful_squash_modify_create() {
        let mut write_set_1 = WriteSetMut::default();
        let mut write_set_2 = WriteSetMut::default();

        // modify 9 + create 109 throws an error
        write_set_1.insert((key(9), modify(9)));
        write_set_2.insert((key(9), create(109)));

        let write_set_1 = write_set_1.freeze().unwrap();
        let write_set_2 = write_set_2.freeze().unwrap();

        let change_set_1 = build_change_set(write_set_1, DeltaChangeSet::empty());
        let change_set_2 = build_change_set(write_set_2, DeltaChangeSet::empty());
        let res = change_set_1.squash(change_set_2, &NoOpChangeSetChecker);
        assert_matches!(
            res,
            Err(VMStatus::Error(
                StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                Some(_),
            ))
        );
    }

    #[test]
    fn test_unsuccessful_squash_delete_modify() {
        let mut write_set_1 = WriteSetMut::default();
        let mut write_set_2 = WriteSetMut::default();

        // delete + modify 113 throws an error
        write_set_1.insert((key(13), delete()));
        write_set_2.insert((key(13), modify(113)));

        let write_set_1 = write_set_1.freeze().unwrap();
        let write_set_2 = write_set_2.freeze().unwrap();

        let change_set_1 = build_change_set(write_set_1, DeltaChangeSet::empty());
        let change_set_2 = build_change_set(write_set_2, DeltaChangeSet::empty());
        let res = change_set_1.squash(change_set_2, &NoOpChangeSetChecker);
        assert_matches!(
            res,
            Err(VMStatus::Error(
                StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                Some(_),
            ))
        );
    }

    #[test]
    fn test_unsuccessful_squash_delete_delete() {
        let mut write_set_1 = WriteSetMut::default();
        let mut write_set_2 = WriteSetMut::default();

        // delete + delete throws an error
        write_set_1.insert((key(14), delete()));
        write_set_2.insert((key(14), delete()));

        let write_set_1 = write_set_1.freeze().unwrap();
        let write_set_2 = write_set_2.freeze().unwrap();

        let change_set_1 = build_change_set(write_set_1, DeltaChangeSet::empty());
        let change_set_2 = build_change_set(write_set_2, DeltaChangeSet::empty());
        let res = change_set_1.squash(change_set_2, &NoOpChangeSetChecker);
        assert_matches!(
            res,
            Err(VMStatus::Error(
                StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                Some(_),
            ))
        );
    }

    #[test]
    fn test_unsuccessful_squash_delete_delta() {
        let mut write_set_1 = WriteSetMut::default();
        let mut delta_change_set_2 = DeltaChangeSet::empty();

        // delete + +120 throws an error
        write_set_1.insert((key(20), delete()));
        delta_change_set_2.insert((key(20), add(120)));

        let write_set_1 = write_set_1.freeze().unwrap();
        let change_set_1 = build_change_set(write_set_1, DeltaChangeSet::empty());
        let change_set_2 = build_change_set(WriteSet::default(), delta_change_set_2);
        let res = change_set_1.squash(change_set_2, &NoOpChangeSetChecker);
        assert_matches!(
            res,
            Err(VMStatus::Error(
                StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                Some(_),
            ))
        );
    }

    #[test]
    fn test_unsuccessful_squash_delta_create() {
        let mut write_set_2 = WriteSetMut::default();
        let mut delta_change_set_1 = DeltaChangeSet::empty();

        // +21 + create 122 throws an error
        delta_change_set_1.insert((key(21), add(21)));
        write_set_2.insert((key(21), create(121)));

        let write_set_2 = write_set_2.freeze().unwrap();
        let change_set_1 = build_change_set(WriteSet::default(), delta_change_set_1);
        let change_set_2 = build_change_set(write_set_2, DeltaChangeSet::empty());
        let res = change_set_1.squash(change_set_2, &NoOpChangeSetChecker);
        assert_matches!(
            res,
            Err(VMStatus::Error(
                StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                Some(_),
            ))
        );
    }
}
