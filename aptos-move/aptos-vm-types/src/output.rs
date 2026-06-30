// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    abstract_write_op::AbstractResourceWriteOp,
    change_set::{ChangeSetInterface, VMChangeSet},
    module_write_set::{ModuleWrite, ModuleWriteSet},
};
use aptos_aggregator::delayed_change::DelayedChange;
use aptos_types::{
    contract_event::ContractEvent,
    error::{code_invariant_error, PanicError},
    fee_statement::FeeStatement,
    state_store::state_key::StateKey,
    transaction::{TransactionAuxiliaryData, TransactionOutput, TransactionStatus},
    write_set::WriteOp,
};
use derivative::Derivative;
use move_core_types::{
    value::MoveTypeLayout,
    vm_status::{StatusCode, VMStatus},
};
use move_vm_runtime::execution_tracing::Trace;
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
use rustc_hash::FxHashSet;
use std::{collections::BTreeMap, mem};

/// Output produced by the VM after executing a transaction.
///
/// **WARNING**: This type should only be used inside the VM. For storage backends,
/// use `TransactionOutput`.
#[derive(Derivative)]
#[derivative(Debug, Clone, Eq, PartialEq)]
pub struct VMOutput {
    change_set: VMChangeSet,
    module_write_set: ModuleWriteSet,
    fee_statement: FeeStatement,
    status: TransactionStatus,
    /// Trace of the user transaction payload execution in Move VM. Trace is always created as
    /// empty, and users have to set it manually after execution.
    #[derivative(PartialEq = "ignore", Debug = "ignore")]
    trace: Trace,
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
            trace: Trace::empty(),
        }
    }

    pub fn empty_with_status(status: TransactionStatus) -> Self {
        Self {
            change_set: VMChangeSet::empty(),
            module_write_set: ModuleWriteSet::empty(),
            fee_statement: FeeStatement::zero(),
            status,
            trace: Trace::empty(),
        }
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

    /// Sets the trace for this output. Should only be called once to replace the default empty
    /// trace with one recorded by the Move VM.
    ///
    /// Panics if current putput stores non-empty trace.
    pub fn set_trace(&mut self, trace: Trace) {
        let old = mem::replace(&mut self.trace, trace);
        assert!(old.is_empty());
    }

    /// Extracts the trace from the output, for subsequent replay.
    pub fn take_trace(&mut self) -> Trace {
        mem::take(&mut self.trace)
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

    /// Constructs the `TransactionOutput`, mapping a conversion error to a VM status. There is
    /// nothing to materialize in the sequential path: with the delayed field optimization off the
    /// natives apply aggregator V1 deltas in place, and with it on the deltas are materialized by
    /// the block executor.
    pub fn try_materialize_into_transaction_output(
        self,
    ) -> anyhow::Result<TransactionOutput, VMStatus> {
        self.into_transaction_output().map_err(|e| {
            VMStatus::error(
                StatusCode::DELAYED_FIELD_OR_BLOCKSTM_CODE_INVARIANT_ERROR,
                Some(e.to_string()),
            )
        })
    }

    /// Constructs `TransactionOutput`.
    pub fn into_transaction_output(self) -> Result<TransactionOutput, PanicError> {
        let Self {
            change_set,
            module_write_set,
            fee_statement,
            status,
            trace,
        } = self;

        // INVARIANT:
        //   When converting to transaction output, trace is either irrelevant or has already been
        //   extracted.
        if !trace.is_empty() {
            return Err(PanicError::CodeInvariantError(
                "Non-empty trace found when converting to transaction output".to_string(),
            ));
        }

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

    /// Replaces the change set events / resources with the patched ones (that carry
    /// materialized delayed fields). Drains delayed fields (everything is materialized)
    /// and returns a [`TransactionOutput`].
    pub fn into_transaction_output_with_materialized_write_set(
        mut self,
        patched_resource_write_set: Vec<(StateKey, WriteOp)>,
        patched_events: Vec<ContractEvent>,
    ) -> Result<TransactionOutput, PanicError> {
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

/// A transaction's read set, used for hot-state promotion. Unordered at the
/// per-transaction level; ordering is imposed later when aggregating per-block.
///
/// Data and module keys are kept as recorded. Both sides are already deduplicated and
/// they can never contain the same key (module and data state keys are disjoint), so
/// merging them into one set would only re-hash every module key.
#[derive(Clone, Debug, Default)]
pub struct UnorderedReadSet {
    data_keys: FxHashSet<StateKey>,
    module_keys: FxHashSet<StateKey>,
}

impl UnorderedReadSet {
    pub fn new(data_keys: FxHashSet<StateKey>, module_keys: FxHashSet<StateKey>) -> Self {
        Self {
            data_keys,
            module_keys,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &StateKey> {
        self.data_keys.iter().chain(self.module_keys.iter())
    }
}
