// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    captured_reads::CapturedReads,
    errors::ParallelBlockExecutionError,
    explicit_sync_wrapper::ExplicitSyncWrapper,
    task::{ExecutionStatus, TransactionOutput},
    types::{InputOutputKey, ReadWriteSummary},
};
use aptos_aggregator::types::code_invariant_error;
use aptos_logger::error;
use aptos_mvhashmap::types::{Incarnation, TxnIndex, ValueWithLayout};
use aptos_types::{
    delayed_fields::PanicError, fee_statement::FeeStatement,
    state_store::state_value::StateValueMetadata,
    transaction::BlockExecutableTransaction as Transaction, write_set::WriteOp,
};
use arc_swap::ArcSwapOption;
use crossbeam::utils::CachePadded;
use dashmap::DashSet;
use move_core_types::value::MoveTypeLayout;
use std::{
    collections::{BTreeMap, HashSet},
    fmt::Debug,
    iter::{empty, Iterator},
    sync::Arc,
};

type TxnInput<T> = CapturedReads<T>;

macro_rules! forward_on_success_or_skip_rest {
    ($self:ident, $txn_idx:ident, $f:ident) => {{
        $self.outputs[$txn_idx as usize]
            .load()
            .as_ref()
            .map_or(vec![], |txn_output| match &txn_output.status {
                ExecutionStatus::Success(t) | ExecutionStatus::SkipRest(t) => t.$f(),
                ExecutionStatus::Abort(_)
                | ExecutionStatus::SpeculativeExecutionAbortError(_)
                | ExecutionStatus::DelayedFieldsCodeInvariantError(_) => vec![],
            })
    }};
}

pub(crate) enum KeyKind {
    Resource,
    Module,
    Group,
}

#[derive(Debug)]
pub(crate) struct RecordedOutput<O, E> {
    pub(crate) status: ExecutionStatus<O, E>,
    pub(crate) incarnation: Incarnation,
}

pub(crate) struct RecordedInput<T: Transaction> {
    pub(crate) input: TxnInput<T>,
    pub(crate) is_backup_validated: bool,
}

impl<O, E> RecordedOutput<O, E> {
    pub(crate) fn new(status: ExecutionStatus<O, E>, incarnation: Incarnation) -> Self {
        Self {
            status,
            incarnation,
        }
    }

    pub(crate) fn take(self) -> (ExecutionStatus<O, E>, Incarnation) {
        (self.status, self.incarnation)
    }
}

impl<T: Transaction> RecordedInput<T> {
    pub(crate) fn new(input: TxnInput<T>, is_backup_validated: bool) -> Self {
        Self {
            input,
            is_backup_validated,
        }
    }
}

pub struct TxnLastInputOutput<T: Transaction, O: TransactionOutput<Txn = T>, E: Debug> {
    // txn_idx -> RecordedInput
    inputs: Vec<CachePadded<ArcSwapOption<RecordedInput<T>>>>,
    // Set once when the group outputs are committed sequentially, to be processed later by
    // concurrent materialization / output preparation.
    finalized_groups: Vec<
        CachePadded<
            ExplicitSyncWrapper<Vec<(T::Key, T::Value, Vec<(T::Tag, ValueWithLayout<T::Value>)>)>>,
        >,
    >,

    // txn_idx -> RecordedOutput
    outputs: Vec<CachePadded<ArcSwapOption<RecordedOutput<O, E>>>>,
    // Cache to avoid expensive clones of data.
    // TODO(clean-up): be consistent with naming resource writes: here it means specifically
    // individual writes, but in some contexts it refers to all writes (e.g. including group writes)
    arced_resource_writes: Vec<
        CachePadded<ExplicitSyncWrapper<Vec<(T::Key, Arc<T::Value>, Option<Arc<MoveTypeLayout>>)>>>,
    >,

    // Record all writes and reads to access paths corresponding to modules (code) in any
    // (speculative) executions. Used to avoid a potential race with module publishing and
    // Move-VM loader cache - see 'record' function comment for more information.
    module_writes: DashSet<T::Key>,
    module_reads: DashSet<T::Key>,
}

impl<T: Transaction, O: TransactionOutput<Txn = T>, E: Debug + Send + Clone>
    TxnLastInputOutput<T, O, E>
{
    pub fn new(num_txns: TxnIndex) -> Self {
        Self {
            inputs: (0..num_txns)
                .map(|_| CachePadded::new(ArcSwapOption::empty()))
                .collect(),
            outputs: (0..num_txns)
                .map(|_| CachePadded::new(ArcSwapOption::empty()))
                .collect(),
            arced_resource_writes: (0..num_txns)
                .map(|_| CachePadded::new(ExplicitSyncWrapper::<Vec<_>>::new(vec![])))
                .collect(),
            finalized_groups: (0..num_txns)
                .map(|_| CachePadded::new(ExplicitSyncWrapper::<Vec<_>>::new(vec![])))
                .collect(),
            module_writes: DashSet::new(),
            module_reads: DashSet::new(),
        }
    }

    fn append_and_check<'a>(
        paths: impl Iterator<Item = &'a T::Key>,
        set_to_append: &DashSet<T::Key>,
        set_to_check: &DashSet<T::Key>,
    ) -> bool {
        for path in paths {
            // Standard flags, first show, then look.
            set_to_append.insert(path.clone());

            if set_to_check.contains(path) {
                return true;
            }
        }
        false
    }

    /// Returns false on an error - if a module path that was read was previously written to, and vice versa.
    /// Since parallel executor is instantiated per block, any module that is in the Move-VM loader
    /// cache must previously be read and would be recorded in the 'module_reads' set. Any module
    /// that is written (published or re-published) goes through transaction output write-set and
    /// gets recorded in the 'module_writes' set. If these sets have an intersection, it is currently
    /// possible that Move-VM loader cache loads a module and incorrectly uses it for another
    /// transaction (e.g. a smaller transaction, or if the speculative execution of the publishing
    /// transaction later aborts). The intersection is guaranteed to be found because we first
    /// record the paths then check the other set (flags principle), and in this case we return an
    /// error that ensures a fallback to a correct sequential execution.
    /// When the sets do not have an intersection, it is impossible for the race to occur as any
    /// module in the loader cache may not be published by a transaction in the ongoing block.
    pub(crate) fn record(
        &self,
        txn_idx: TxnIndex,
        incarnation: Incarnation,
        is_backup_validated: bool,
        input: CapturedReads<T>,
        output: ExecutionStatus<O, E>,
        arced_resource_writes: Vec<(T::Key, Arc<T::Value>, Option<Arc<MoveTypeLayout>>)>,
    ) -> bool {
        let written_modules = match &output {
            ExecutionStatus::Success(output) | ExecutionStatus::SkipRest(output) => {
                output.module_write_set()
            },
            ExecutionStatus::Abort(_)
            | ExecutionStatus::SpeculativeExecutionAbortError(_)
            | ExecutionStatus::DelayedFieldsCodeInvariantError(_) => BTreeMap::new(),
        };

        if self
            .check_and_append_module_rw_conflict(input.module_reads.iter(), written_modules.keys())
        {
            return false;
        }

        *self.arced_resource_writes[txn_idx as usize].acquire() = arced_resource_writes;
        self.inputs[txn_idx as usize].store(Some(Arc::new(RecordedInput::new(
            input,
            is_backup_validated,
        ))));
        self.outputs[txn_idx as usize]
            .store(Some(Arc::new(RecordedOutput::new(output, incarnation))));

        true
    }

    pub(crate) fn check_and_append_module_rw_conflict<'a>(
        &self,
        module_reads_keys: impl Iterator<Item = &'a T::Key>,
        module_writes_keys: impl Iterator<Item = &'a T::Key>,
    ) -> bool {
        // Check if adding new read & write modules leads to intersections.
        Self::append_and_check(module_reads_keys, &self.module_reads, &self.module_writes)
            || Self::append_and_check(module_writes_keys, &self.module_writes, &self.module_reads)
    }

    pub(crate) fn recorded_input(&self, txn_idx: TxnIndex) -> Option<Arc<RecordedInput<T>>> {
        self.inputs[txn_idx as usize].load_full()
    }

    /// Returns the total gas, execution gas, io gas and storage gas of the transaction.
    pub(crate) fn fee_statement(&self, txn_idx: TxnIndex) -> Option<FeeStatement> {
        match &self.outputs[txn_idx as usize]
            .load_full()
	    .unwrap_or_else(|| panic!("[BlockSTM]: Execution output for txn {txn_idx} must be recorded after execution"))
            .status
        {
            ExecutionStatus::Success(output) | ExecutionStatus::SkipRest(output) => {
                Some(output.fee_statement())
            },
            _ => None,
        }
    }

    pub(crate) fn output_approx_size(&self, txn_idx: TxnIndex) -> Option<u64> {
        match &self.outputs[txn_idx as usize]
            .load_full()
            .unwrap_or_else(|| panic!("[BlockSTM]: Execution output for txn {txn_idx} must be recorded after execution"))
            .status        {
            ExecutionStatus::Success(output) | ExecutionStatus::SkipRest(output) => {
                Some(output.output_approx_size())
            },
            _ => None,
        }
    }

    /// Does a transaction at txn_idx have SkipRest or Abort status.
    pub(crate) fn block_skips_rest_at_idx(&self, txn_idx: TxnIndex) -> bool {
        matches!(
            &self.outputs[txn_idx as usize]
                .load_full()
                .unwrap_or_else(|| panic!("[BlockSTM]: Execution output for txn {txn_idx} must be recorded after execution"))
                .status,
            ExecutionStatus::SkipRest(_)
        )
    }

    pub(crate) fn check_fatal_vm_error(
        &self,
        txn_idx: TxnIndex,
    ) -> Result<(), ParallelBlockExecutionError> {
        if let Some(recorded_output) = self.outputs[txn_idx as usize].load_full() {
            if let ExecutionStatus::Abort(err) = &recorded_output.status {
                error!(
                    "FatalVMError from parallel execution {:?} at txn {}",
                    err, txn_idx
                );
                return Err(ParallelBlockExecutionError::FatalVMError);
            }
        }
        Ok(())
    }

    pub(crate) fn check_execution_status_during_commit(
        &self,
        txn_idx: TxnIndex,
    ) -> Result<(), PanicError> {
        if let Some(recorded_output) = self.outputs[txn_idx as usize].load_full() {
            match &recorded_output.status {
                ExecutionStatus::Success(_) | ExecutionStatus::SkipRest(_) => Ok(()),
                // Transaction cannot be committed with below statuses, as:
                // - Speculative error must have failed validation.
                // - Execution w. delayed field code error propagates the error directly,
                // does not finish execution. Similar for FatalVMError / abort.
                ExecutionStatus::Abort(_) => {
                    Err(code_invariant_error("Abort status cannot be committed"))
                },
                ExecutionStatus::SpeculativeExecutionAbortError(_) => Err(code_invariant_error(
                    "Speculative error status cannot be committed",
                )),
                ExecutionStatus::DelayedFieldsCodeInvariantError(_) => Err(code_invariant_error(
                    "Delayed field invariant error cannot be committed",
                )),
            }
        } else {
            Err(code_invariant_error(
                "Recorded output not found during commit",
            ))
        }
    }

    pub(crate) fn update_to_skip_rest(&self, txn_idx: TxnIndex) {
        if self.block_skips_rest_at_idx(txn_idx) {
            // Already skipping.
            return;
        }

        // check_execution_status_during_commit must be used for checks re:status.
        // Hence, since the status is not SkipRest, it must be Success.
        if let (ExecutionStatus::Success(output), incarnation) = self.take_output(txn_idx).take() {
            self.outputs[txn_idx as usize].store(Some(Arc::new(RecordedOutput::new(
                ExecutionStatus::SkipRest(output),
                incarnation,
            ))));
        } else {
            unreachable!("Unexpected status, must be Success");
        }
    }

    pub(crate) fn txn_output(&self, txn_idx: TxnIndex) -> Option<Arc<RecordedOutput<O, E>>> {
        self.outputs[txn_idx as usize].load_full()
    }

    // Extracts a set of paths (keys) written or updated during execution from transaction
    // output, .1 for each item is false for non-module paths and true for module paths.
    pub(crate) fn modified_keys(
        &self,
        txn_idx: TxnIndex,
    ) -> Option<impl Iterator<Item = (T::Key, KeyKind)>> {
        self.outputs[txn_idx as usize]
            .load_full()
            .and_then(|txn_output| match &txn_output.status {
                ExecutionStatus::Success(t) | ExecutionStatus::SkipRest(t) => Some(
                    t.resource_write_set()
                        .into_iter()
                        .map(|(k, _, _)| k)
                        .chain(t.aggregator_v1_write_set().into_keys())
                        .chain(
                            t.aggregator_v1_delta_set()
                                .into_iter()
                                .map(|(k, _)| k)
                                .collect::<Vec<_>>(),
                        )
                        .map(|k| (k, KeyKind::Resource))
                        .chain(
                            t.module_write_set()
                                .into_keys()
                                .map(|k| (k, KeyKind::Module)),
                        )
                        .chain(
                            t.resource_group_metadata_ops()
                                .into_iter()
                                .map(|(k, _)| (k, KeyKind::Group)),
                        ),
                ),
                ExecutionStatus::Abort(_)
                | ExecutionStatus::SpeculativeExecutionAbortError(_)
                | ExecutionStatus::DelayedFieldsCodeInvariantError(_) => None,
            })
    }

    pub(crate) fn delayed_field_keys(
        &self,
        txn_idx: TxnIndex,
    ) -> Option<impl Iterator<Item = T::Identifier>> {
        self.outputs[txn_idx as usize]
            .load()
            .as_ref()
            .and_then(|txn_output| match &txn_output.status {
                ExecutionStatus::Success(t) | ExecutionStatus::SkipRest(t) => {
                    Some(t.delayed_field_change_set().into_keys())
                },
                ExecutionStatus::Abort(_)
                | ExecutionStatus::SpeculativeExecutionAbortError(_)
                | ExecutionStatus::DelayedFieldsCodeInvariantError(_) => None,
            })
    }

    pub(crate) fn reads_needing_delayed_field_exchange(
        &self,
        txn_idx: TxnIndex,
    ) -> Vec<(T::Key, StateValueMetadata, Arc<MoveTypeLayout>)> {
        forward_on_success_or_skip_rest!(self, txn_idx, reads_needing_delayed_field_exchange)
    }

    pub(crate) fn group_reads_needing_delayed_field_exchange(
        &self,
        txn_idx: TxnIndex,
    ) -> Vec<(T::Key, StateValueMetadata)> {
        forward_on_success_or_skip_rest!(self, txn_idx, group_reads_needing_delayed_field_exchange)
    }

    pub(crate) fn aggregator_v1_delta_keys(&self, txn_idx: TxnIndex) -> Vec<T::Key> {
        forward_on_success_or_skip_rest!(self, txn_idx, aggregator_v1_delta_set)
            .into_iter()
            .map(|(k, _)| k)
            .collect()
    }

    pub(crate) fn resource_group_metadata_ops(&self, txn_idx: TxnIndex) -> Vec<(T::Key, T::Value)> {
        forward_on_success_or_skip_rest!(self, txn_idx, resource_group_metadata_ops)
    }

    pub(crate) fn events(
        &self,
        txn_idx: TxnIndex,
    ) -> Box<dyn Iterator<Item = (T::Event, Option<MoveTypeLayout>)>> {
        self.outputs[txn_idx as usize].load().as_ref().map_or(
            Box::new(empty::<(T::Event, Option<MoveTypeLayout>)>()),
            |txn_output| match &txn_output.status {
                ExecutionStatus::Success(t) | ExecutionStatus::SkipRest(t) => {
                    let events = t.get_events();
                    Box::new(events.into_iter())
                },
                ExecutionStatus::Abort(_)
                | ExecutionStatus::SpeculativeExecutionAbortError(_)
                | ExecutionStatus::DelayedFieldsCodeInvariantError(_) => {
                    Box::new(empty::<(T::Event, Option<MoveTypeLayout>)>())
                },
            },
        )
    }

    pub(crate) fn record_finalized_group(
        &self,
        txn_idx: TxnIndex,
        finalized_groups: Vec<(T::Key, T::Value, Vec<(T::Tag, ValueWithLayout<T::Value>)>)>,
    ) {
        *self.finalized_groups[txn_idx as usize].acquire() = finalized_groups;
    }

    pub(crate) fn take_finalized_group(
        &self,
        txn_idx: TxnIndex,
    ) -> Vec<(T::Key, T::Value, Vec<(T::Tag, ValueWithLayout<T::Value>)>)> {
        std::mem::take(&mut self.finalized_groups[txn_idx as usize].acquire())
    }

    pub(crate) fn take_resource_write_set(
        &self,
        txn_idx: TxnIndex,
    ) -> Vec<(T::Key, Arc<T::Value>, Option<Arc<MoveTypeLayout>>)> {
        std::mem::take(&mut self.arced_resource_writes[txn_idx as usize].acquire())
    }

    // Called when a transaction is committed to record WriteOps for materialized aggregator values
    // corresponding to the (deltas) in the recorded final output of the transaction, as well as
    // finalized group updates.
    pub(crate) fn record_materialized_txn_output(
        &self,
        txn_idx: TxnIndex,
        delta_writes: Vec<(T::Key, WriteOp)>,
        patched_resource_write_set: Vec<(T::Key, T::Value)>,
        patched_events: Vec<T::Event>,
    ) -> Result<(), PanicError> {
        match &self.outputs[txn_idx as usize]
            .load_full()
            .expect("Output must exist")
            .status
        {
            ExecutionStatus::Success(t) | ExecutionStatus::SkipRest(t) => {
                t.incorporate_materialized_txn_output(
                    delta_writes,
                    patched_resource_write_set,
                    patched_events,
                )?;
            },
            ExecutionStatus::Abort(_)
            | ExecutionStatus::SpeculativeExecutionAbortError(_)
            | ExecutionStatus::DelayedFieldsCodeInvariantError(_) => {},
        };
        Ok(())
    }

    pub(crate) fn get_txn_read_write_summary(&self, txn_idx: TxnIndex) -> ReadWriteSummary<T> {
        let read_set = &self
            .recorded_input(txn_idx)
            .expect("Read set must be recorded")
            .input;

        let reads = read_set.get_read_summary();
        let writes = self.get_write_summary(txn_idx);
        ReadWriteSummary::new(reads, writes)
    }

    pub(crate) fn get_write_summary(
        &self,
        txn_idx: TxnIndex,
    ) -> HashSet<InputOutputKey<T::Key, T::Tag, T::Identifier>> {
        match &self.outputs[txn_idx as usize]
            .load_full()
            .expect("Output must exist")
            .status
        {
            ExecutionStatus::Success(t) | ExecutionStatus::SkipRest(t) => t.get_write_summary(),
            ExecutionStatus::Abort(_)
            | ExecutionStatus::SpeculativeExecutionAbortError(_)
            | ExecutionStatus::DelayedFieldsCodeInvariantError(_) => HashSet::new(),
        }
    }

    // Must be executed after parallel execution is done, grabs outputs. Will panic if
    // other outstanding references to the recorded outputs exist.
    pub(crate) fn take_output(&self, txn_idx: TxnIndex) -> RecordedOutput<O, E> {
        let owning_ptr = self.outputs[txn_idx as usize]
            .swap(None)
            .expect("[BlockSTM]: Output must be recorded after execution");

        Arc::try_unwrap(owning_ptr)
            .expect("[BlockSTM]: Output should be uniquely owned after execution")
    }
}
