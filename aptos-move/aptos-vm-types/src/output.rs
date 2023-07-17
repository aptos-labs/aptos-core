// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::change_set::VMChangeSet;
use aptos_aggregator::delta_change_set::DeltaChangeSet;
use aptos_state_view::StateView;
use aptos_types::{
    contract_event::ContractEvent,
    fee_statement::FeeStatement,
    state_store::state_key::StateKey,
    transaction::{TransactionOutput, TransactionStatus},
    write_set::{WriteOp, WriteSet},
};
use move_core_types::vm_status::VMStatus;

/// Output produced by the VM. Before VMOutput is passed to storage backends,
/// it must be converted to TransactionOutput.
#[derive(Debug, Clone)]
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

    /// Returns a new empty transaction output. Useful for handling discards or
    /// system transactions.
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

    pub fn write_set(&self) -> &WriteSet {
        self.change_set.write_set()
    }

    pub fn delta_change_set(&self) -> &DeltaChangeSet {
        self.change_set.delta_change_set()
    }

    pub fn events(&self) -> &[ContractEvent] {
        self.change_set.events()
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

    /// Materializes this transaction output by materializing the change set it
    /// carries. Materialization can fail due to delta applications, in which
    /// case an error is returned.
    /// If the call succeeds (returns `Ok(..)`), the output is guaranteed to have
    /// an empty delta change set.
    pub fn try_materialize(self, state_view: &impl StateView) -> anyhow::Result<Self, VMStatus> {
        // First, check if output of transaction should be discarded or delta
        // change set is empty. In both cases, we do not need to apply any
        // deltas and can return immediately.
        if self.status().is_discarded() || self.delta_change_set().is_empty() {
            return Ok(self);
        }

        // Try to materialize deltas and add them to the write set.
        let (change_set, fee_statement, status) = self.unpack_with_fee_statement();
        let materialized_change_set = change_set.try_materialize(state_view)?;
        Ok(VMOutput::new(
            materialized_change_set,
            fee_statement,
            status,
        ))
    }

    /// Converts VMOutput into TransactionOutput which can be used by storage
    /// backends. During this conversion delta materialization can fail, in
    /// which case an error is returned.
    pub fn into_transaction_output(
        self,
        state_view: &impl StateView,
    ) -> anyhow::Result<TransactionOutput, VMStatus> {
        let materialized_output = self.try_materialize(state_view)?;
        let (change_set, gas_used, status) = materialized_output.unpack();
        let (write_set, delta_change_set, events) = change_set.unpack();

        debug_assert!(
            delta_change_set.is_empty(),
            "DeltaChangeSet must be empty after materialization."
        );

        Ok(TransactionOutput::new(write_set, events, gas_used, status))
    }

    /// Converts VM output into transaction output which storage or state sync
    /// can understand. Extends writes with values from materialized deltas.
    pub fn output_with_delta_writes(
        self,
        delta_writes: Vec<(StateKey, WriteOp)>,
    ) -> TransactionOutput {
        let (change_set, gas_used, status) = self.unpack();
        let (write_set, mut delta_change_set, events) = change_set.unpack();
        let mut write_set_mut = write_set.into_mut();

        // We should have a materialized delta for every delta in the output.
        assert_eq!(delta_writes.len(), delta_change_set.len());

        // Add the delta writes to the write set of the transaction.
        delta_writes.into_iter().for_each(|item| {
            debug_assert!(
                delta_change_set.remove(&item.0).is_some(),
                "Delta writes contain a key which does not exist in DeltaChangeSet."
            );
            write_set_mut.insert(item)
        });

        let write_set = write_set_mut
            .freeze()
            .expect("Freezing of WriteSet should succeed.");
        TransactionOutput::new(write_set, events, gas_used, status)
    }
}
