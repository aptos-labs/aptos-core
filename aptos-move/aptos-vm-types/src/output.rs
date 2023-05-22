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

#[derive(Debug)]
pub struct VMOutput {
    writes: WriteSet,
    deltas: DeltaChangeSet,
    events: Vec<ContractEvent>,
    gas_used: u64,
    status: TransactionStatus,
}

impl VMOutput {
    pub fn new(
        writes: WriteSet,
        deltas: DeltaChangeSet,
        events: Vec<ContractEvent>,
        gas_used: u64,
        status: TransactionStatus,
    ) -> Self {
        Self {
            writes,
            deltas,
            events,
            gas_used,
            status,
        }
    }

    pub fn empty_with_status(status: TransactionStatus) -> Self {
        Self {
            writes: WriteSet::default(),
            deltas: DeltaChangeSet::empty(),
            events: vec![],
            gas_used: 0,
            status,
        }
    }

    pub fn into(self) -> (WriteSet, DeltaChangeSet, Vec<ContractEvent>) {
        (self.writes, self.deltas, self.events)
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
            self.writes,
            self.deltas,
            self.events,
            self.gas_used,
            self.status,
        )
    }

    pub fn writes(&self) -> &WriteSet {
        &self.writes
    }

    pub fn deltas(&self) -> &DeltaChangeSet {
        &self.deltas
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

    /// Converts VM output into transaction output which storage or state sync
    /// understand. Extends writes with values from materialized deltas.
    pub fn output_with_delta_writes(
        self,
        delta_writes: Vec<(StateKey, WriteOp)>,
    ) -> TransactionOutput {
        // We should have a materialized delta for every delta in the output.
        let (writes, deltas, events, gas_used, status) = self.unpack();
        assert_eq!(deltas.len(), delta_writes.len());

        let mut write_set_mut = writes.into_mut();
        // Add the delta writes to the write set of the transaction.
        delta_writes
            .into_iter()
            .for_each(|item| write_set_mut.insert(item));

        let writes = write_set_mut
            .freeze()
            .expect("Freezing of WriteSet should succeed.");
        TransactionOutput::new(writes, events, gas_used, status)
    }

    /// Tries to materialize deltas and merges them with the set of writes produced
    /// by this VM output.
    pub fn try_materialize(self, state_view: &impl StateView) -> anyhow::Result<Self, VMStatus> {
        // First, check if output of transaction should be discarded or delta
        // change set is empty. In both cases, we do not need to apply any
        // deltas and can return immediately.
        if self.status.is_discarded() || self.deltas.is_empty() {
            return Ok(self);
        }

        let (writes, deltas, events, gas_used, status) = self.unpack();
        let delta_writes = deltas.take_materialized(state_view)?;
        let mut write_set_mut = writes.into_mut();

        // Add the delta writes to the write set of the transaction.
        delta_writes
            .into_iter()
            .for_each(|item| write_set_mut.insert(item));

        let writes = write_set_mut.freeze().map_err(|_| {
            VMStatus::Error(
                StatusCode::DATA_FORMAT_ERROR,
                Some("failed to freeze writeset".to_string()),
            )
        })?;
        Ok(VMOutput::new(
            writes,
            DeltaChangeSet::empty(),
            events,
            gas_used,
            status,
        ))
    }
}
