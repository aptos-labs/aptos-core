// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    abstract_write_op::AbstractResourceWriteOp,
    change_set::{ChangeSetInterface, VMChangeSet},
    module_write_set::{ModuleWrite, ModuleWriteSet},
};
use velor_aggregator::{
    delayed_change::DelayedChange, delta_change_set::DeltaOp, resolver::AggregatorV1Resolver,
};
use velor_types::{
    contract_event::ContractEvent,
    error::{code_invariant_error, PanicError},
    fee_statement::FeeStatement,
    state_store::state_key::StateKey,
    transaction::{TransactionAuxiliaryData, TransactionOutput, TransactionStatus},
    write_set::WriteOp,
};
use move_core_types::{
    value::MoveTypeLayout,
    vm_status::{StatusCode, VMStatus},
};
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
use std::collections::BTreeMap;

/// Output produced by the VM after executing a transaction.
///
/// **WARNING**: This type should only be used inside the VM. For storage backends,
/// use `TransactionOutput`.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct VMOutput {
    change_set: VMChangeSet,
    module_write_set: ModuleWriteSet,
    fee_statement: FeeStatement,
    status: TransactionStatus,
}

impl VMOutput {
    pub fn new(
        change_set: VMChangeSet,
        module_write_set: ModuleWriteSet,
        fee_statement: FeeStatement,
        status: TransactionStatus,
    ) -> Self {
        Self {
            change_set,
            module_write_set,
            fee_statement,
            status,
        }
    }

    pub fn empty_with_status(status: TransactionStatus) -> Self {
        Self {
            change_set: VMChangeSet::empty(),
            module_write_set: ModuleWriteSet::empty(),
            fee_statement: FeeStatement::zero(),
            status,
        }
    }

    pub fn aggregator_v1_delta_set(&self) -> &BTreeMap<StateKey, DeltaOp> {
        self.change_set.aggregator_v1_delta_set()
    }

    pub fn aggregator_v1_write_set(&self) -> &BTreeMap<StateKey, WriteOp> {
        self.change_set.aggregator_v1_write_set()
    }

    pub fn resource_write_set(&self) -> &BTreeMap<StateKey, AbstractResourceWriteOp> {
        self.change_set.resource_write_set()
    }

    pub fn module_write_set(&self) -> &BTreeMap<StateKey, ModuleWrite<WriteOp>> {
        self.module_write_set.writes()
    }

    pub fn delayed_field_change_set(
        &self,
    ) -> &BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>> {
        self.change_set.delayed_field_change_set()
    }

    pub fn events(&self) -> &[(ContractEvent, Option<MoveTypeLayout>)] {
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

    pub fn materialized_size(&self) -> u64 {
        let mut size = 0;
        for (state_key, write_size) in self
            .change_set
            .write_set_size_iter()
            .chain(self.module_write_set.write_set_size_iter())
        {
            size += state_key.size() as u64 + write_size.write_len().unwrap_or(0);
        }

        for event in self.change_set.events_iter() {
            size += event.size() as u64;
        }
        size
    }

    pub fn concrete_write_set_iter(&self) -> impl Iterator<Item = (&StateKey, Option<&WriteOp>)> {
        self.change_set.concrete_write_set_iter().chain(
            self.module_write_set
                .writes()
                .iter()
                .map(|(k, v)| (k, Some(v.write_op()))),
        )
    }

    /// Materializes delta sets.
    /// Guarantees that if deltas are materialized successfully, the output
    /// has an empty delta set.
    /// TODO `[agg_v2](cleanup)` Consolidate materialization paths. See either:
    /// - if we can/should move try_materialize_aggregator_v1_delta_set into
    ///   executor.rs
    /// - move all materialization (including delayed fields) into change_set
    pub fn try_materialize(
        &mut self,
        resolver: &impl AggregatorV1Resolver,
    ) -> anyhow::Result<(), VMStatus> {
        // First, check if output of transaction should be discarded or delta
        // change set is empty. In both cases, we do not need to apply any
        // deltas and can return immediately.
        if self.status().is_discarded()
            || (self.aggregator_v1_delta_set().is_empty()
                && self.delayed_field_change_set().is_empty())
        {
            return Ok(());
        }

        self.change_set
            .try_materialize_aggregator_v1_delta_set(resolver)?;

        Ok(())
    }

    /// Same as `try_materialize` but also constructs `TransactionOutput`.
    pub fn try_materialize_into_transaction_output(
        mut self,
        resolver: &impl AggregatorV1Resolver,
    ) -> anyhow::Result<TransactionOutput, VMStatus> {
        self.try_materialize(resolver)?;
        self.into_transaction_output().map_err(|e| {
            VMStatus::error(
                StatusCode::DELAYED_FIELD_OR_BLOCKSTM_CODE_INVARIANT_ERROR,
                Some(e.to_string()),
            )
        })
    }

    /// Constructs `TransactionOutput`, without doing `try_materialize`.
    pub fn into_transaction_output(self) -> Result<TransactionOutput, PanicError> {
        let Self {
            change_set,
            module_write_set,
            fee_statement,
            status,
        } = self;
        let (write_set, events) = change_set
            .try_combine_into_storage_change_set(module_write_set)?
            .into_inner();
        Ok(TransactionOutput::new(
            write_set,
            events,
            fee_statement.gas_used(),
            status,
            TransactionAuxiliaryData::default(),
        ))
    }

    /// Updates the VMChangeSet based on the input aggregator v1 deltas, patched resource write set,
    /// patched events, and generates TransactionOutput
    pub fn into_transaction_output_with_materialized_write_set(
        mut self,
        materialized_aggregator_v1_deltas: Vec<(StateKey, WriteOp)>,
        patched_resource_write_set: Vec<(StateKey, WriteOp)>,
        patched_events: Vec<ContractEvent>,
    ) -> Result<TransactionOutput, PanicError> {
        // materialize aggregator V1 deltas into writes
        if materialized_aggregator_v1_deltas.len() != self.aggregator_v1_delta_set().len() {
            return Err(code_invariant_error(
                "Different number of materialized deltas and deltas in the output.",
            ));
        }
        if !materialized_aggregator_v1_deltas
            .iter()
            .all(|(k, _)| self.aggregator_v1_delta_set().contains_key(k))
        {
            return Err(code_invariant_error(
                "Materialized aggregator writes contain a key which does not exist in delta set.",
            ));
        }
        self.change_set
            .extend_aggregator_v1_write_set(materialized_aggregator_v1_deltas.into_iter());
        // TODO[agg_v2](cleanup) move all drains to happen when getting what to materialize.
        let _ = self.change_set.drain_aggregator_v1_delta_set();

        // materialize delayed fields into resource writes
        self.change_set
            .extend_resource_write_set(patched_resource_write_set.into_iter())?;
        let _ = self.change_set.drain_delayed_field_change_set();

        // materialize delayed fields into events
        if patched_events.len() != self.events().len() {
            return Err(code_invariant_error(
                "Different number of events and patched events in the output.",
            ));
        }
        self.change_set.set_events(patched_events.into_iter());

        self.into_transaction_output()
    }
}
