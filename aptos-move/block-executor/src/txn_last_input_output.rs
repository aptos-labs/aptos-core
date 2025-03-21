// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    captured_reads::{CapturedReads, DataRead, ReadKind},
    errors::ParallelBlockExecutionError,
    explicit_sync_wrapper::ExplicitSyncWrapper,
    task::{ExecutionStatus, TransactionOutput},
    types::{InputOutputKey, ReadWriteSummary},
};
use aptos_logger::error;
use aptos_mvhashmap::types::TxnIndex;
use aptos_types::{
    error::{code_invariant_error, PanicError},
    fee_statement::FeeStatement,
    state_store::state_value::StateValueMetadata,
    transaction::BlockExecutableTransaction as Transaction,
    vm::modules::AptosModuleExtension,
    write_set::WriteOp,
};
use aptos_vm_types::module_write_set::ModuleWrite;
use arc_swap::ArcSwapOption;
use crossbeam::utils::CachePadded;
use move_binary_format::CompiledModule;
use move_core_types::{language_storage::ModuleId, value::MoveTypeLayout};
use move_vm_runtime::Module;
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
use std::{
    collections::{BTreeMap, HashSet},
    fmt::Debug,
    iter::{empty, Iterator},
    sync::Arc,
};

type TxnInput<T> = CapturedReads<T, ModuleId, CompiledModule, Module, AptosModuleExtension>;

macro_rules! forward_on_success_or_skip_rest {
    ($self:ident, $txn_idx:ident, $f:ident) => {{
        $self.outputs[$txn_idx as usize]
            .load()
            .as_ref()
            .map_or_else(Vec::new, |txn_output| match txn_output.as_ref() {
                ExecutionStatus::Success(t) | ExecutionStatus::SkipRest(t) => t.$f(),
                ExecutionStatus::Abort(_)
                | ExecutionStatus::SpeculativeExecutionAbortError(_)
                | ExecutionStatus::DelayedFieldsCodeInvariantError(_) => vec![],
            })
    }};
}

pub(crate) enum KeyKind<T> {
    Resource,
    // Contains the set of tags for the given group key.
    Group(HashSet<T>),
}

pub struct TxnLastInputOutput<T: Transaction, O: TransactionOutput<Txn = T>, E: Debug> {
    inputs: Vec<CachePadded<ArcSwapOption<TxnInput<T>>>>, // txn_idx -> input.

    // TODO: Consider breaking down the outputs when storing (avoid traversals, cache below).
    outputs: Vec<CachePadded<ArcSwapOption<ExecutionStatus<O, E>>>>, // txn_idx -> output.
    // Cache to avoid expensive clones of data.
    // TODO(clean-up): be consistent with naming resource writes: here it means specifically
    // individual writes, but in some contexts it refers to all writes (e.g. including group writes)
    arced_resource_writes: Vec<
        CachePadded<ExplicitSyncWrapper<Vec<(T::Key, Arc<T::Value>, Option<Arc<MoveTypeLayout>>)>>>,
    >,
    resource_group_keys_and_tags:
        Vec<CachePadded<ExplicitSyncWrapper<Vec<(T::Key, HashSet<T::Tag>)>>>>,
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
            resource_group_keys_and_tags: (0..num_txns)
                .map(|_| CachePadded::new(ExplicitSyncWrapper::<Vec<_>>::new(vec![])))
                .collect(),
        }
    }

    pub(crate) fn record(
        &self,
        txn_idx: TxnIndex,
        input: TxnInput<T>,
        output: ExecutionStatus<O, E>,
        arced_resource_writes: Vec<(T::Key, Arc<T::Value>, Option<Arc<MoveTypeLayout>>)>,
        group_keys_and_tags: Vec<(T::Key, HashSet<T::Tag>)>,
    ) {
        *self.arced_resource_writes[txn_idx as usize].acquire() = arced_resource_writes;
        *self.resource_group_keys_and_tags[txn_idx as usize].acquire() = group_keys_and_tags;
        self.inputs[txn_idx as usize].store(Some(Arc::new(input)));
        self.outputs[txn_idx as usize].store(Some(Arc::new(output)));
    }

    pub fn fetch_exchanged_data(
        &self,
        key: &T::Key,
        txn_idx: TxnIndex,
    ) -> Result<(Arc<T::Value>, Arc<MoveTypeLayout>), PanicError> {
        self.inputs[txn_idx as usize].load().as_ref().map_or_else(
            || {
                Err(code_invariant_error(
                    "Read must be recorded before fetching exchanged data".to_string(),
                ))
            },
            |input| {
                let data_read = input.get_by_kind(key, None, ReadKind::Value);
                if let Some(DataRead::Versioned(_, value, Some(layout))) = data_read {
                    Ok((value, layout))
                } else {
                    Err(code_invariant_error(format!(
                        "Read value needing exchange {:?} not in Exchanged format",
                        data_read
                    )))
                }
            },
        )
    }

    pub(crate) fn read_set(&self, txn_idx: TxnIndex) -> Option<Arc<TxnInput<T>>> {
        self.inputs[txn_idx as usize].load_full()
    }

    /// Returns the total gas, execution gas, io gas and storage gas of the transaction.
    pub(crate) fn fee_statement(&self, txn_idx: TxnIndex) -> Option<FeeStatement> {
        match self.outputs[txn_idx as usize]
            .load_full()
            .unwrap_or_else(|| panic!("[BlockSTM]: Execution output for txn {txn_idx} must be recorded after execution"))
            .as_ref()
        {
       	    ExecutionStatus::Success(output) | ExecutionStatus::SkipRest(output) => Some(output.fee_statement()),
            _ => None,
        }
    }

    pub(crate) fn output_approx_size(&self, txn_idx: TxnIndex) -> Option<u64> {
        match self.outputs[txn_idx as usize]
            .load_full()
            .unwrap_or_else(|| panic!("[BlockSTM]: Execution output for txn {txn_idx} must be recorded after execution"))
            .as_ref()
        {
            ExecutionStatus::Success(output) | ExecutionStatus::SkipRest(output) => {
		Some(output.output_approx_size())
            },
            _ => None,
        }
    }

    /// Does a transaction at txn_idx have SkipRest or Abort status.
    pub(crate) fn block_skips_rest_at_idx(&self, txn_idx: TxnIndex) -> bool {
        matches!(
            self.outputs[txn_idx as usize]
                .load_full()
                .unwrap_or_else(|| panic!("[BlockSTM]: Execution output for txn {txn_idx} must be recorded after execution"))
                .as_ref(),
            ExecutionStatus::SkipRest(_)
        )
    }

    pub(crate) fn check_fatal_vm_error(
        &self,
        txn_idx: TxnIndex,
    ) -> Result<(), ParallelBlockExecutionError> {
        if let Some(status) = self.outputs[txn_idx as usize].load_full() {
            if let ExecutionStatus::Abort(err) = status.as_ref() {
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
        if let Some(status) = self.outputs[txn_idx as usize].load_full() {
            match status.as_ref() {
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

    pub(crate) fn update_to_skip_rest(&self, txn_idx: TxnIndex) -> Result<(), PanicError> {
        if self.block_skips_rest_at_idx(txn_idx) {
            // Already skipping.
            return Ok(());
        }

        // check_execution_status_during_commit must be used for checks re:status.
        // Hence, since the status is not SkipRest, it must be Success.
        if let ExecutionStatus::Success(output) = self.take_output(txn_idx)? {
            self.outputs[txn_idx as usize].store(Some(Arc::new(ExecutionStatus::SkipRest(output))));
            Ok(())
        } else {
            Err(code_invariant_error(
                "Unexpected status to change to SkipRest, must be Success",
            ))
        }
    }

    pub(crate) fn txn_output(&self, txn_idx: TxnIndex) -> Option<Arc<ExecutionStatus<O, E>>> {
        self.outputs[txn_idx as usize].load_full()
    }

    // Extracts a set of resource paths (keys) written or updated during execution from transaction
    // output, with corresponding KeyKind. If take_group_tags is true, the final HashSet
    // of tags is moved for the group key - should be called once for each incarnation / record
    // due to 'take'. if false, stored modified group resource tags in the group are cloned out.
    pub(crate) fn modified_keys(
        &self,
        txn_idx: TxnIndex,
        take_group_tags: bool,
    ) -> Option<impl Iterator<Item = (T::Key, KeyKind<T::Tag>)>> {
        let group_keys_and_tags: Vec<(T::Key, HashSet<T::Tag>)> = if take_group_tags {
            std::mem::take(&mut self.resource_group_keys_and_tags[txn_idx as usize].acquire())
        } else {
            self.resource_group_keys_and_tags[txn_idx as usize]
                .acquire()
                .clone()
        };

        self.outputs[txn_idx as usize]
            .load_full()
            .and_then(|txn_output| match txn_output.as_ref() {
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
                            group_keys_and_tags
                                .into_iter()
                                .map(|(k, tags)| (k, KeyKind::Group(tags))),
                        ),
                ),
                ExecutionStatus::Abort(_)
                | ExecutionStatus::SpeculativeExecutionAbortError(_)
                | ExecutionStatus::DelayedFieldsCodeInvariantError(_) => None,
            })
    }

    pub(crate) fn module_write_set(
        &self,
        txn_idx: TxnIndex,
    ) -> BTreeMap<T::Key, ModuleWrite<T::Value>> {
        use ExecutionStatus as E;

        match self.outputs[txn_idx as usize]
            .load()
            .as_ref()
            .map(|status| status.as_ref())
        {
            Some(E::Success(t) | E::SkipRest(t)) => t.module_write_set(),
            Some(
                E::Abort(_)
                | E::DelayedFieldsCodeInvariantError(_)
                | E::SpeculativeExecutionAbortError(_),
            )
            | None => BTreeMap::new(),
        }
    }

    pub(crate) fn delayed_field_keys(
        &self,
        txn_idx: TxnIndex,
    ) -> Option<impl Iterator<Item = DelayedFieldID>> {
        self.outputs[txn_idx as usize]
            .load()
            .as_ref()
            .and_then(|txn_output| match txn_output.as_ref() {
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
        match self.outputs[txn_idx as usize].load().as_ref() {
            None => Box::new(empty::<(T::Event, Option<MoveTypeLayout>)>()),
            Some(txn_output) => match txn_output.as_ref() {
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
        }
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
        match self.outputs[txn_idx as usize]
            .load_full()
            .expect("Output must exist")
            .as_ref()
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
        let read_set = self.read_set(txn_idx).expect("Read set must be recorded");

        let reads = read_set.get_read_summary();
        let writes = self.get_write_summary(txn_idx);
        ReadWriteSummary::new(reads, writes)
    }

    pub(crate) fn get_write_summary(
        &self,
        txn_idx: TxnIndex,
    ) -> HashSet<InputOutputKey<T::Key, T::Tag>> {
        match self.outputs[txn_idx as usize]
            .load_full()
            .expect("Output must exist")
            .as_ref()
        {
            ExecutionStatus::Success(t) | ExecutionStatus::SkipRest(t) => t.get_write_summary(),
            ExecutionStatus::Abort(_)
            | ExecutionStatus::SpeculativeExecutionAbortError(_)
            | ExecutionStatus::DelayedFieldsCodeInvariantError(_) => HashSet::new(),
        }
    }

    // Must be executed after parallel execution is done, grabs outputs. Will panic if
    // other outstanding references to the recorded outputs exist.
    pub(crate) fn take_output(
        &self,
        txn_idx: TxnIndex,
    ) -> Result<ExecutionStatus<O, E>, PanicError> {
        let owning_ptr = self.outputs[txn_idx as usize].swap(None).ok_or_else(|| {
            code_invariant_error("[BlockSTM]: Output must be recorded after execution")
        })?;

        Arc::try_unwrap(owning_ptr).map_err(|_| {
            code_invariant_error("[BlockSTM]: Output must be uniquely owned after execution")
        })
    }
}
