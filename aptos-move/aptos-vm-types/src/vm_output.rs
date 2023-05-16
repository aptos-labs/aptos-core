// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    change_set::{into_write_set, AptosChangeSet},
    op::Op,
    write_change_set::WriteChangeSet,
};
use aptos_aggregator::delta_change_set::DeltaChangeSet;
use aptos_state_view::StateView;
use aptos_types::{
    contract_event::ContractEvent,
    state_store::state_key::StateKey,
    transaction::{TransactionOutput, TransactionStatus},
};
use move_core_types::vm_status::VMStatus;

#[derive(Debug)]
pub struct VMOutput {
    resource_writes: WriteChangeSet<Vec<u8>>,
    module_writes: WriteChangeSet<Vec<u8>>,
    aggregator_writes: WriteChangeSet<Vec<u8>>,
    deltas: DeltaChangeSet,
    events: Vec<ContractEvent>,
    gas_used: u64,
    status: TransactionStatus,
}

impl VMOutput {
    pub fn new(
        resource_writes: WriteChangeSet<Vec<u8>>,
        module_writes: WriteChangeSet<Vec<u8>>,
        aggregator_writes: WriteChangeSet<Vec<u8>>,
        deltas: DeltaChangeSet,
        events: Vec<ContractEvent>,
        gas_used: u64,
        status: TransactionStatus,
    ) -> Self {
        Self {
            resource_writes,
            module_writes,
            aggregator_writes,
            deltas,
            events,
            gas_used,
            status,
        }
    }

    pub fn from_change_set(
        change_set: AptosChangeSet,
        gas_used: u64,
        status: TransactionStatus,
    ) -> Self {
        let (resource_writes, module_writes, aggregator_writes, deltas, events) =
            change_set.into_inner();
        Self {
            resource_writes,
            module_writes,
            aggregator_writes,
            deltas,
            events,
            gas_used,
            status,
        }
    }

    pub fn empty_with_status(status: TransactionStatus) -> Self {
        Self {
            resource_writes: WriteChangeSet::empty(),
            module_writes: WriteChangeSet::empty(),
            aggregator_writes: WriteChangeSet::empty(),
            deltas: DeltaChangeSet::empty(),
            events: vec![],
            gas_used: 0,
            status,
        }
    }

    pub fn into(
        self,
    ) -> (
        WriteChangeSet<Vec<u8>>,
        WriteChangeSet<Vec<u8>>,
        WriteChangeSet<Vec<u8>>,
        DeltaChangeSet,
        Vec<ContractEvent>,
    ) {
        (
            self.resource_writes,
            self.module_writes,
            self.aggregator_writes,
            self.deltas,
            self.events,
        )
    }

    pub fn unpack(
        self,
    ) -> (
        WriteChangeSet<Vec<u8>>,
        WriteChangeSet<Vec<u8>>,
        WriteChangeSet<Vec<u8>>,
        DeltaChangeSet,
        Vec<ContractEvent>,
        u64,
        TransactionStatus,
    ) {
        (
            self.resource_writes,
            self.module_writes,
            self.aggregator_writes,
            self.deltas,
            self.events,
            self.gas_used,
            self.status,
        )
    }

    pub fn resource_writes(&self) -> &WriteChangeSet<Vec<u8>> {
        &self.resource_writes
    }

    pub fn module_writes(&self) -> &WriteChangeSet<Vec<u8>> {
        &self.module_writes
    }

    pub fn aggregator_writes(&self) -> &WriteChangeSet<Vec<u8>> {
        &self.aggregator_writes
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
    pub fn output_with_materialized_deltas(
        self,
        materialized_deltas: Vec<(StateKey, Op<Vec<u8>>)>,
    ) -> TransactionOutput {
        // We should have a materialized delta for every delta in the output.
        let (
            resource_writes,
            module_writes,
            mut aggregator_writes,
            deltas,
            events,
            gas_used,
            status,
        ) = self.unpack();
        assert_eq!(deltas.len(), materialized_deltas.len());

        // First, extend aggregator writes with materialized deltas.
        aggregator_writes
            .extend_with_writes(materialized_deltas)
            .expect("Extending with materialized deltas should always succeed");

        let write_set = into_write_set(resource_writes, module_writes, aggregator_writes)
            .expect("Conversion to WriteSet should succeed");
        TransactionOutput::new(write_set, events, gas_used, status)
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

        let (
            resource_writes,
            module_writes,
            mut aggregator_writes,
            deltas,
            events,
            gas_used,
            status,
        ) = self.unpack();
        let materialized_deltas = WriteChangeSet::from_deltas(deltas, state_view)?;
        aggregator_writes
            .extend_with_writes(materialized_deltas)
            .expect("Extending with materialized deltas should always succeed");
        Ok(VMOutput::new(
            resource_writes,
            module_writes,
            aggregator_writes,
            DeltaChangeSet::empty(),
            events,
            gas_used,
            status,
        ))
    }
}
