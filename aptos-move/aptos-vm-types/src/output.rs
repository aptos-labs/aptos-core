// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::change_set::VMChangeSet;
use aptos_state_view::StateView;
use aptos_types::{
    fee_statement::FeeStatement,
    state_store::state_key::StateKey,
    transaction::{TransactionOutput, TransactionStatus},
    write_set::WriteOp,
};
use move_core_types::vm_status::VMStatus;

/// Output produced by the VM after executing a transaction.
///
/// **WARNING**: This type should only be used inside the VM. For storage backends,
/// use `TransactionOutput`.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct VMOutput {
    change_set: VMChangeSet,
    fee_statement: FeeStatement,
    status: TransactionStatus,
}

impl VMOutput {
    pub fn new(
        change_set: VMChangeSet,
        fee_statement: FeeStatement,
        status: TransactionStatus,
    ) -> Self {
        Self {
            change_set,
            fee_statement,
            status,
        }
    }

    pub fn empty_with_status(status: TransactionStatus) -> Self {
        Self {
            change_set: VMChangeSet::empty(),
            fee_statement: FeeStatement::zero(),
            status,
        }
    }

    pub fn unpack(self) -> (VMChangeSet, u64, TransactionStatus) {
        (self.change_set, self.fee_statement.gas_used(), self.status)
    }

    pub fn unpack_with_fee_statement(self) -> (VMChangeSet, FeeStatement, TransactionStatus) {
        (self.change_set, self.fee_statement, self.status)
    }

    pub fn change_set(&self) -> &VMChangeSet {
        &self.change_set
    }

    pub fn gas_used(&self) -> u64 {
        self.fee_statement.gas_used()
    }

    pub fn fee_statement(&self) -> &FeeStatement {
        &self.fee_statement
    }

    pub fn status(&self) -> &TransactionStatus {
        &self.status
    }

    /// Materializes delta sets.
    /// Guarantees that if deltas are materialized successfully, the output
    /// has an empty delta set.
    pub fn try_materialize(self, state_view: &impl StateView) -> anyhow::Result<Self, VMStatus> {
        // First, check if output of transaction should be discarded or delta
        // change set is empty. In both cases, we do not need to apply any
        // deltas and can return immediately.
        if self.status().is_discarded() || self.change_set().aggregator_v1_delta_set().is_empty() {
            return Ok(self);
        }

        let (change_set, fee_statement, status) = self.unpack_with_fee_statement();
        let materialized_change_set = change_set.try_materialize(state_view)?;
        Ok(VMOutput::new(
            materialized_change_set,
            fee_statement,
            status,
        ))
    }

    /// Same as `try_materialize` but also constructs `TransactionOutput`.
    pub fn try_into_transaction_output(
        self,
        state_view: &impl StateView,
    ) -> anyhow::Result<TransactionOutput, VMStatus> {
        let materialized_output = self.try_materialize(state_view)?;
        debug_assert!(
            materialized_output
                .change_set()
                .aggregator_v1_delta_set()
                .is_empty(),
            "Aggregator deltas must be empty after materialization."
        );
        let (vm_change_set, gas_used, status) = materialized_output.unpack();
        let (write_set, events) = vm_change_set.try_into_storage_change_set()?.into_inner();
        Ok(TransactionOutput::new(write_set, events, gas_used, status))
    }

    /// Similar to `try_into_transaction_output` but deltas are materialized
    /// externally by the caller beforehand.
    pub fn into_transaction_output_with_materialized_deltas(
        mut self,
        materialized_deltas: Vec<(StateKey, WriteOp)>,
    ) -> TransactionOutput {
        // We should have a materialized delta for every delta in the output.
        assert_eq!(
            materialized_deltas.len(),
            self.change_set().aggregator_v1_delta_set().len()
        );
        debug_assert!(
            materialized_deltas
                .iter()
                .all(|(k, _)| self.change_set().aggregator_v1_delta_set().contains_key(k)),
            "Materialized aggregator writes contain a key which does not exist in delta set."
        );
        self.change_set
            .extend_aggregator_write_set(materialized_deltas.into_iter());

        let (vm_change_set, gas_used, status) = self.unpack();
        let (write_set, events) = vm_change_set
            .into_storage_change_set_unchecked()
            .into_inner();
        TransactionOutput::new(write_set, events, gas_used, status)
    }
}
