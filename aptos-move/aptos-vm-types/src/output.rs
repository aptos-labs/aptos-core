// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::change_set::VMChangeSet;
use aptos_state_view::StateView;
use aptos_types::{
    fee_statement::FeeStatement,
    state_store::state_key::StateKey,
    transaction::{TransactionOutput, TransactionStatus},
    write_set::{WriteOp, WriteSetMut},
};
use move_core_types::vm_status::VMStatus;

/// Output produced by the VM. Before VMOutput is passed to storage backends,
/// it must be converted to TransactionOutput.
#[derive(Debug, Clone)]
pub struct VMOutput {
    // All changes to the state, including data, code, events.
    change_set: VMChangeSet,
    // Encapsulates all gas charges, e.g. execution, I/), storage, etc.
    fee_statement: FeeStatement,
    // Status of the executed transaction.
    status: TransactionStatus,
}

impl VMOutput {
    /// Creates a new instance of VM-specific output.
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

    /// Returns an empty transaction output. Useful for handling discards or
    /// system transactions which do not contain any state changes.
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

    /// Materializes this transaction output by materializing the change set it
    /// carries. Materialization can fail due to delta applications, in which
    /// case an error is returned.
    /// If the call succeeds (returns `Ok(..)`), the output is guaranteed to have
    /// an empty delta change set.
    pub fn try_materialize(self, state_view: &impl StateView) -> anyhow::Result<Self, VMStatus> {
        // First, check if output of transaction should be discarded or delta
        // change set is empty. In both cases, we do not need to apply any
        // deltas and can return immediately.
        if self.status().is_discarded() || self.change_set().delta_change_set().is_empty() {
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
        debug_assert!(
            materialized_output
                .change_set()
                .delta_change_set()
                .is_empty(),
            "DeltaChangeSet must be empty after materialization."
        );

        let (change_set, gas_used, status) = materialized_output.unpack();
        let (write_set, events) = change_set.try_into_storage_change_set()?.into_inner();
        Ok(TransactionOutput::new(write_set, events, gas_used, status))
    }

    /// Converts VM output into transaction output which storage or state sync
    /// can understand. Extends writes with values from materialized deltas.
    pub fn output_with_delta_writes(
        self,
        delta_writes: Vec<(StateKey, WriteOp)>,
    ) -> TransactionOutput {
        let (change_set, gas_used, status) = self.unpack();

        // We should have a materialized delta for every delta in the output.
        assert_eq!(delta_writes.len(), change_set.delta_change_set().len());
        debug_assert!(
            delta_writes
                .iter()
                .all(|(k, _)| change_set.delta_change_set().contains(k)),
            "Delta writes contain a key which does not exist in DeltaChangeSet."
        );

        // Add the delta writes to the write set of the transaction.
        let mut write_set_mut = WriteSetMut::new(delta_writes);
        let (write_set, events) = change_set
            .try_into_storage_change_set()
            .expect("Conversion to storage ChangeSet should succeed")
            .into_inner();
        write_set_mut.extend(write_set);

        // Construct the final transaction output.
        let write_set = write_set_mut
            .freeze()
            .expect("Freezing the write set should not fail");
        TransactionOutput::new(write_set, events, gas_used, status)
    }
}
