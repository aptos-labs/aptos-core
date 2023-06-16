// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{change_set::VMChangeSet, check_change_set::CheckChangeSet, output::VMOutput};
use aptos_aggregator::delta_change_set::{serialize, DeltaChangeSet, DeltaOp};
use aptos_types::{
    fee_statement::FeeStatement,
    state_store::state_key::StateKey,
    transaction::{ExecutionStatus, TransactionStatus},
    write_set::{WriteOp, WriteSetMut},
};
use move_core_types::vm_status::VMStatus;

/// A mock for testing. Always succeeds on checking a change set.
pub(crate) struct NoOpChangeSetChecker;

impl CheckChangeSet for NoOpChangeSetChecker {
    fn check_change_set(&self, _change_set: &VMChangeSet) -> anyhow::Result<(), VMStatus> {
        Ok(())
    }
}

/// Returns a new state key for the given index. Allows to associate indices
/// with state keys.
pub(crate) fn key(id: u128) -> StateKey {
    StateKey::raw(format!("key-{}", id).into_bytes())
}

/// Returns a new write op which creates an integer value.
pub(crate) fn create(value: u128) -> WriteOp {
    WriteOp::Creation(serialize(&value))
}

/// Returns a new write op which modifies an integer value.
pub(crate) fn modify(value: u128) -> WriteOp {
    WriteOp::Modification(serialize(&value))
}

/// Returns a new write op which deletes a value.
pub(crate) fn delete() -> WriteOp {
    WriteOp::Deletion
}

/// Returns a write op from the change set stored at state key corresponding
/// to an id.
pub(crate) fn get_write_op(change_set: &VMChangeSet, id: u128) -> WriteOp {
    change_set.write_set().get(&key(id)).unwrap().clone()
}

/// Returns true if there is a write op in the change set for the state key
/// corresponding to an id.
pub(crate) fn contains_write_op(change_set: &VMChangeSet, id: u128) -> bool {
    change_set.write_set().get(&key(id)).is_some()
}

/// Returns a delta op from the change set stored at state key corresponding
/// to an id.
pub(crate) fn get_delta_op(change_set: &VMChangeSet, id: u128) -> DeltaOp {
    *change_set.delta_change_set().get(&key(id)).unwrap()
}

/// Returns true if there is a delta op in the change set for the state key
/// corresponding to an id.
pub(crate) fn contains_delta_op(change_set: &VMChangeSet, id: u128) -> bool {
    change_set.delta_change_set().get(&key(id)).is_some()
}

/// Returns a new change set built from writes and deltas.
pub(crate) fn build_change_set(
    write_set: WriteSetMut,
    delta_change_set: DeltaChangeSet,
) -> VMChangeSet {
    VMChangeSet::new(
        write_set.freeze().unwrap(),
        delta_change_set,
        vec![],
        &NoOpChangeSetChecker,
    )
    .unwrap()
}

/// Returns a new VMOutput built from writes and deltas. The output has always a
/// success execution status and uses 100 gas units (values are not significant
/// for testing purposes).
pub(crate) fn build_vm_output(
    write_set: WriteSetMut,
    delta_change_set: DeltaChangeSet,
) -> VMOutput {
    const GAS_USED: u64 = 100;
    const STATUS: TransactionStatus = TransactionStatus::Keep(ExecutionStatus::Success);
    VMOutput::new(
        build_change_set(write_set, delta_change_set),
        FeeStatement::new(GAS_USED, GAS_USED, 0, 0, 0),
        STATUS,
    )
}
