// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::change_set::VMChangeSet;
use aptos_aggregator::resolver::AggregatorV1Resolver;
use aptos_types::{
    contract_event::ContractEvent, //contract_event::ContractEvent,
    fee_statement::FeeStatement,
    state_store::state_key::StateKey,
    transaction::{TransactionOutput, TransactionStatus},
    write_set::WriteOp, aggregator::PanicError,
};
use move_core_types::vm_status::{VMStatus, StatusCode};

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

    pub fn change_set_mut(&mut self) -> &mut VMChangeSet {
        &mut self.change_set
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
    /// TODO[agg_v2](fix) organize materialization paths better.
    pub fn try_materialize(
        self,
        resolver: &impl AggregatorV1Resolver,
    ) -> anyhow::Result<Self, VMStatus> {
        // First, check if output of transaction should be discarded or delta
        // change set is empty. In both cases, we do not need to apply any
        // deltas and can return immediately.
        if self.status().is_discarded()
            || (self.change_set().aggregator_v1_delta_set().is_empty()
                && self.change_set().delayed_field_change_set().is_empty())
        {
            return Ok(self);
        }

        let (change_set, fee_statement, status) = self.unpack_with_fee_statement();
        let materialized_change_set =
            change_set.try_materialize_aggregator_v1_delta_set(resolver)?;
        // TODO[agg_v2](fix) shouldn't be needed when reorganized
        //     .try_materialize_aggregator_v2_changes(state_view)?;
        Ok(VMOutput::new(
            materialized_change_set,
            fee_statement,
            status,
        ))
    }

    /// Same as `try_materialize` but also constructs `TransactionOutput`.
    pub fn try_into_transaction_output(
        self,
        resolver: &impl AggregatorV1Resolver,
    ) -> anyhow::Result<TransactionOutput, VMStatus> {
        let materialized_output = self.try_materialize(resolver)?;
        Self::convert_to_transaction_output(materialized_output).map_err(|e| VMStatus::error(
            StatusCode::DELAYED_FIELDS_CODE_INVARIANT_ERROR,
            Some(e.to_string()),
        ))
    }

    /// Constructs `TransactionOutput`, without doing `try_materialize`
    pub fn into_transaction_output(self) -> anyhow::Result<TransactionOutput, VMStatus> {
        let (change_set, fee_statement, status) = self.unpack_with_fee_statement();
        let materialized_output = VMOutput::new(change_set, fee_statement, status);
        Self::convert_to_transaction_output(materialized_output).map_err(|e| VMStatus::error(
            StatusCode::DELAYED_FIELDS_CODE_INVARIANT_ERROR,
            Some(e.to_string()),
        ))
    }

    fn convert_to_transaction_output(
        materialized_output: VMOutput,
    ) -> Result<TransactionOutput, PanicError> {
        assert!(
            materialized_output
                .change_set()
                .aggregator_v1_delta_set()
                .is_empty(),
            "Aggregator deltas must be empty after materialization."
        );
        assert!(
            materialized_output
                .change_set()
                .delayed_field_change_set()
                .is_empty(),
            "Delayed fields must be empty after materialization."
        );
        let (vm_change_set, gas_used, status) = materialized_output.unpack();
        let (write_set, events) = vm_change_set.try_into_storage_change_set()?.into_inner();
        Ok(TransactionOutput::new(write_set, events, gas_used, status))
    }

    /// Updates the VMChangeSet based on the input aggregator v1 deltas, patched resource write set,
    /// patched events, and generates TransactionOutput
    pub fn into_transaction_output_with_materialized_write_set(
        mut self,
        materialized_aggregator_v1_deltas: Vec<(StateKey, WriteOp)>,
        patched_resource_write_set: Vec<(StateKey, WriteOp)>,
        patched_events: Vec<ContractEvent>,
    ) -> Result<TransactionOutput, PanicError> {
        assert_eq!(
            materialized_aggregator_v1_deltas.len(),
            self.change_set().aggregator_v1_delta_set().len(),
            "Different number of materialized deltas and deltas in the output."
        );
        debug_assert!(
            materialized_aggregator_v1_deltas
                .iter()
                .all(|(k, _)| self.change_set().aggregator_v1_delta_set().contains_key(k)),
            "Materialized aggregator writes contain a key which does not exist in delta set."
        );
        self.change_set
            .extend_aggregator_v1_write_set(materialized_aggregator_v1_deltas.into_iter());
        self.change_set.extend_resource_write_set(
            patched_resource_write_set.into_iter(),
        )?;

        assert_eq!(
            patched_events.len(),
            self.change_set().events().len(),
            "Different number of events and patched events in the output."
        );
        self.change_set.set_events(patched_events.into_iter());
        // TODO[agg_v2](cleanup) move drain to happen when getting what to materialize.
        let _ = self.change_set.drain_delayed_field_change_set();
        let _ = self.change_set.drain_aggregator_v1_delta_set();

        let (vm_change_set, gas_used, status) = self.unpack();
        let (write_set, events) = vm_change_set
            .into_storage_change_set_forced()
            .into_inner();
        Ok(TransactionOutput::new(write_set, events, gas_used, status))
    }
}
