// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{change_set::VMChangeSet, check_change_set::CheckChangeSet, output::VMOutput};
use aptos_aggregator::delta_change_set::{delta_add, serialize, DeltaOp};
use aptos_types::{
    fee_statement::FeeStatement,
    state_store::state_key::StateKey,
    transaction::{ExecutionStatus, TransactionStatus},
    write_set::WriteOp,
};
use move_core_types::vm_status::VMStatus;
use std::collections::HashMap;

pub(crate) struct MockChangeSetChecker;

impl CheckChangeSet for MockChangeSetChecker {
    fn check_change_set(&self, _change_set: &VMChangeSet) -> anyhow::Result<(), VMStatus> {
        Ok(())
    }
}

macro_rules! as_state_key {
    ($k:ident) => {
        StateKey::raw($k.to_string().into_bytes())
    };
}
pub(crate) use as_state_key;

macro_rules! as_bytes {
    ($v:ident) => {
        serialize(&$v)
    };
}

pub(crate) fn mock_create(k: impl ToString, v: u128) -> (StateKey, WriteOp) {
    (as_state_key!(k), WriteOp::Creation(as_bytes!(v)))
}

pub(crate) fn mock_modify(k: impl ToString, v: u128) -> (StateKey, WriteOp) {
    (as_state_key!(k), WriteOp::Modification(as_bytes!(v)))
}

pub(crate) fn mock_delete(k: impl ToString) -> (StateKey, WriteOp) {
    (as_state_key!(k), WriteOp::Deletion)
}

pub(crate) fn mock_add(k: impl ToString, v: u128) -> (StateKey, DeltaOp) {
    const DUMMY_LIMIT: u128 = 1000;
    (as_state_key!(k), delta_add(v, DUMMY_LIMIT))
}

pub(crate) fn build_change_set(
    resource_write_set: impl IntoIterator<Item = (StateKey, WriteOp)>,
    module_write_set: impl IntoIterator<Item = (StateKey, WriteOp)>,
    aggregator_write_set: impl IntoIterator<Item = (StateKey, WriteOp)>,
    aggregator_delta_set: impl IntoIterator<Item = (StateKey, DeltaOp)>,
) -> VMChangeSet {
    VMChangeSet::new(
        HashMap::from_iter(resource_write_set),
        HashMap::from_iter(module_write_set),
        HashMap::from_iter(aggregator_write_set),
        HashMap::from_iter(aggregator_delta_set),
        vec![],
        &MockChangeSetChecker,
    )
    .unwrap()
}

// For testing, output has always a success execution status and uses 100 gas units.
pub(crate) fn build_vm_output(
    resource_write_set: impl IntoIterator<Item = (StateKey, WriteOp)>,
    module_write_set: impl IntoIterator<Item = (StateKey, WriteOp)>,
    aggregator_write_set: impl IntoIterator<Item = (StateKey, WriteOp)>,
    aggregator_delta_set: impl IntoIterator<Item = (StateKey, DeltaOp)>,
) -> VMOutput {
    const GAS_USED: u64 = 100;
    const STATUS: TransactionStatus = TransactionStatus::Keep(ExecutionStatus::Success);
    VMOutput::new(
        build_change_set(
            resource_write_set,
            module_write_set,
            aggregator_write_set,
            aggregator_delta_set,
        ),
        FeeStatement::new(GAS_USED, GAS_USED, 0, 0, 0),
        STATUS,
    )
}
