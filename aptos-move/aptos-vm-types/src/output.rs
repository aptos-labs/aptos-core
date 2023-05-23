// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_aggregator::delta_change_set::DeltaChangeSet;
use aptos_state_view::StateView;
use aptos_types::{
    contract_event::ContractEvent,
    state_store::state_key::StateKey,
    transaction::{TransactionOutput, TransactionStatus},
    write_set::{WriteOp, WriteSet},
};
use move_core_types::vm_status::{StatusCode, VMStatus};

/// Output produced by the VM. Before VMOutput is passed to storage backends,
/// it must be converted to TransactionOutput.
#[derive(Debug)]
pub struct VMOutput {
    write_set: WriteSet,
    delta_change_set: DeltaChangeSet,
    events: Vec<ContractEvent>,
    gas_used: u64,
    status: TransactionStatus,
}

impl VMOutput {
    pub fn new(
        write_set: WriteSet,
        delta_change_set: DeltaChangeSet,
        events: Vec<ContractEvent>,
        gas_used: u64,
        status: TransactionStatus,
    ) -> Self {
        Self {
            write_set,
            delta_change_set,
            events,
            gas_used,
            status,
        }
    }

    pub fn empty_with_status(status: TransactionStatus) -> Self {
        Self {
            write_set: WriteSet::default(),
            delta_change_set: DeltaChangeSet::empty(),
            events: vec![],
            gas_used: 0,
            status,
        }
    }

    pub fn into(self) -> (WriteSet, DeltaChangeSet, Vec<ContractEvent>) {
        (self.write_set, self.delta_change_set, self.events)
    }

    pub fn unpack(
        self,
    ) -> (
        WriteSet,
        DeltaChangeSet,
        Vec<ContractEvent>,
        u64,
        TransactionStatus,
    ) {
        (
            self.write_set,
            self.delta_change_set,
            self.events,
            self.gas_used,
            self.status,
        )
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

    pub fn gas_used(&self) -> u64 {
        self.gas_used
    }

    pub fn status(&self) -> &TransactionStatus {
        &self.status
    }

    /// Converts VMOutput into TransactionOutput which can be used by storage
    /// backends. During this conversion delta materialization can fail, in
    /// which case an error is returned.
    pub fn into_transaction_output(
        self,
        state_view: &impl StateView,
    ) -> anyhow::Result<TransactionOutput, VMStatus> {
        let (write_set, delta_change_set, events, gas_used, status) = self.unpack();

        // First, check if output of transaction should be discarded or delta
        // change set is empty. In both cases, we do not need to apply any
        // deltas and can return immediately.
        if status.is_discarded() || delta_change_set.is_empty() {
            return Ok(TransactionOutput::new(write_set, events, gas_used, status));
        }

        // Try to materialize deltas and add them to the write set.
        let mut write_set_mut = write_set.into_mut();
        let delta_writes = delta_change_set.take_materialized(state_view)?;
        delta_writes
            .into_iter()
            .for_each(|item| write_set_mut.insert(item));

        let write_set = write_set_mut.freeze().map_err(|_| {
            VMStatus::Error(
                StatusCode::DATA_FORMAT_ERROR,
                Some(
                    "Failed to freeze write set when converting VMOutput to TransactionOutput"
                        .to_string(),
                ),
            )
        })?;

        Ok(TransactionOutput::new(write_set, events, gas_used, status))
    }

    /// Converts VM output into transaction output which storage or state sync
    /// understand. Extends writes with values from materialized deltas.
    pub fn output_with_delta_writes(
        self,
        delta_writes: Vec<(StateKey, WriteOp)>,
    ) -> TransactionOutput {
        // We should have a materialized delta for every delta in the output.
        let (write_set, delta_change_set, events, gas_used, status) = self.unpack();
        assert_eq!(delta_writes.len(), delta_change_set.len());

        let mut write_set_mut = write_set.into_mut();
        // Add the delta writes to the write set of the transaction.
        delta_writes
            .into_iter()
            .for_each(|item| write_set_mut.insert(item));

        let write_set = write_set_mut
            .freeze()
            .expect("Freezing of WriteSet should succeed.");
        TransactionOutput::new(write_set, events, gas_used, status)
    }
}
