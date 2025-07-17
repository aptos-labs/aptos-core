// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    captured_reads::{CapturedReads, DataRead, ReadKind},
    errors::ParallelBlockExecutionError,
    explicit_sync_wrapper::ExplicitSyncWrapper,
    limit_processor::BlockGasLimitProcessor,
    scheduler_wrapper::SchedulerWrapper,
    task::{ExecutionStatus, TransactionOutput},
    types::{InputOutputKey, ReadWriteSummary},
};
use aptos_logger::error;
use aptos_mvhashmap::types::TxnIndex;
use aptos_types::{
    error::{code_invariant_error, PanicError, PanicOr},
    on_chain_config::BlockGasLimitType,
    state_store::{state_value::StateValueMetadata, TStateView},
    transaction::BlockExecutableTransaction as Transaction,
    vm::modules::AptosModuleExtension,
    write_set::WriteOp,
};
use aptos_vm_types::module_write_set::ModuleWrite;
use arc_swap::ArcSwapOption;
use crossbeam::utils::CachePadded;
use fail::fail_point;
use move_binary_format::CompiledModule;
use move_core_types::{language_storage::ModuleId, value::MoveTypeLayout};
use move_vm_runtime::Module;
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
use std::{
    collections::HashSet,
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
}

impl<T: Transaction, O: TransactionOutput<Txn = T>, E: Debug + Send + Clone>
    TxnLastInputOutput<T, O, E>
{
    /// num_txns passed here is typically larger than the number of txns in the block,
    /// currently by 1 to account for the block epilogue txn.
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
        }
    }

    pub(crate) fn record(
        &self,
        txn_idx: TxnIndex,
        input: TxnInput<T>,
        output: ExecutionStatus<O, E>,
        arced_resource_writes: Vec<(T::Key, Arc<T::Value>, Option<Arc<MoveTypeLayout>>)>,
    ) {
        *self.arced_resource_writes[txn_idx as usize].acquire() = arced_resource_writes;
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

    // Should be called when txn_idx is committed, while holding commit lock.
    //
    // Records fee statement separately for block epilogue txn. This is done because the
    // recorded output will be taken by materialization which can be concurrent with the
    // block epilogue txn.
    //
    // Returns whether the block epilogue txn should be created. This is true when both
    // of the following conditions hold:
    // (1) the last txn in the block was committed (if any txns are left over, they must
    // all be skipped), and
    // (2) the last txn did not emit a new epoch event.
    // To avoid unnecessarily inspecting events, we only check (2) if (1) is true.
    pub(crate) fn commit<S: TStateView<Key = T::Key>>(
        &self,
        txn_idx: TxnIndex,
        num_txns: TxnIndex,
        num_workers: usize,
        user_txn_bytes_len: u64,
        block_gas_limit_type: &BlockGasLimitType,
        block_limit_processor: &mut BlockGasLimitProcessor<T, S>,
        scheduler: &SchedulerWrapper,
    ) -> Result<bool, PanicOr<ParallelBlockExecutionError>> {
        let (
            mut skips_rest,
            mut must_create_epilogue_txn,
            maybe_fee_statement_and_output_size,
        ) = match self.outputs[txn_idx as usize]
            .load()
            .as_ref()
            .ok_or_else(|| {
                code_invariant_error(format!(
                    "Execution output for txn {} not found during commit",
                    txn_idx
                ))
            })?
            .as_ref()
        {
            ExecutionStatus::Success(output) => (
                false,
                (txn_idx == num_txns - 1 && !output.has_new_epoch_event()?),
                Some((output.fee_statement(), output.output_approx_size()?)),
            ),
            ExecutionStatus::SkipRest(output) => (
                true,
                !output.has_new_epoch_event()?,
                Some((output.fee_statement(), output.output_approx_size()?)),
            ),
            // Transaction cannot be committed with below statuses, as:
            // - Speculative error must have failed validation.
            // - Execution w. delayed field code error propagates the error directly,
            // does not finish execution. Similar for FatalVMError / abort.
            ExecutionStatus::Abort(err) => {
                // Fatal VM error.
                error!(
                    "FatalVMError from parallel execution {:?} at txn {}",
                    err, txn_idx
                );
                return Err(PanicOr::Or(ParallelBlockExecutionError::FatalVMError));
            },
            ExecutionStatus::SpeculativeExecutionAbortError(_) => {
                return Err(
                    code_invariant_error("Speculative error status cannot be committed").into(),
                );
            },
            ExecutionStatus::DelayedFieldsCodeInvariantError(_) => {
                return Err(code_invariant_error(
                    "Delayed field invariant error cannot be committed",
                )
                .into());
            },
        };

        if let Some((fee_statement, recorded_output_size)) = maybe_fee_statement_and_output_size {
            let approx_output_size = block_gas_limit_type.block_output_limit().map(|_| {
                recorded_output_size
                    + if block_gas_limit_type.include_user_txn_size_in_block_output() {
                        user_txn_bytes_len
                    } else {
                        0
                    }
            });
            let txn_read_write_summary = block_gas_limit_type
                .conflict_penalty_window()
                .map(|_| self.get_txn_read_write_summary(txn_idx));

            // For committed txns with Success status, calculate the accumulated gas costs.
            block_limit_processor.accumulate_fee_statement(
                fee_statement,
                txn_read_write_summary,
                approx_output_size,
            );

            if txn_idx < num_txns - 1
                && block_limit_processor.should_end_block_parallel()
                && !skips_rest
            {
                // Set the execution output status to be SkipRest, to skip the rest of the txns.
                // check_execution_status_during_commit must be used for checks re:status.
                // Hence, since the status is not SkipRest, it must be Success.
                if let ExecutionStatus::Success(output) = self.take_output(txn_idx)? {
                    must_create_epilogue_txn = !output.has_new_epoch_event()?;
                    self.outputs[txn_idx as usize]
                        .store(Some(Arc::new(ExecutionStatus::SkipRest(output))));
                } else {
                    return Err(code_invariant_error(
                        "Unexpected status to change to SkipRest, must be Success",
                    )
                    .into());
                }
                skips_rest = true;
            }
        }

        // Add before halt, so SchedulerV2 can organically observe and process post commit
        // processing tasks even after it has halted.
        scheduler.add_to_post_commit(txn_idx)?;

        // !!! CAUTION !!! after the txn_idx is added to the post commit queue, it is no longer
        // safe to expect an output be stored for txn_idx: post-commit materialization takes
        // the output (instead of cloning for efficiency) for parallel post-processing.

        // While panic errors can lead to halting parallel execution (and fallback),
        // below we may halt the execution by design (no errors) in cases when:
        // a) all transactions are scheduled for committing, or
        // b) we skip_rest after a transaction
        // Either all txn committed, or a committed txn caused an early halt.
        if (txn_idx + 1 == num_txns || skips_rest) && scheduler.halt() {
            block_limit_processor.finish_parallel_update_counters_and_log_info(
                txn_idx + 1,
                num_txns,
                num_workers,
            );

            // failpoint triggering error at the last committed transaction,
            // to test that next transaction is handled correctly
            fail_point!("commit-all-halt-err", |_| Err(code_invariant_error(
                "fail points: Last committed transaction halted"
            )
            .into()));
        }

        Ok(must_create_epilogue_txn)
    }

    pub(crate) fn txn_output(&self, txn_idx: TxnIndex) -> Option<Arc<ExecutionStatus<O, E>>> {
        self.outputs[txn_idx as usize].load_full()
    }

    /// Returns an error if callback returns an error.
    pub(crate) fn for_each_resource_group_key_and_tags<F>(
        &self,
        txn_idx: TxnIndex,
        callback: F,
    ) -> Result<(), PanicError>
    where
        F: FnMut(&T::Key, HashSet<&T::Tag>) -> Result<(), PanicError>,
    {
        if let Some(txn_output) = self.outputs[txn_idx as usize].load().as_ref() {
            match txn_output.as_ref() {
                ExecutionStatus::Success(t) | ExecutionStatus::SkipRest(t) => {
                    t.for_each_resource_group_key_and_tags(callback)?;
                },
                ExecutionStatus::Abort(_)
                | ExecutionStatus::SpeculativeExecutionAbortError(_)
                | ExecutionStatus::DelayedFieldsCodeInvariantError(_) => {
                    // No resource group keys for failed transactions
                },
            }
        }
        Ok(())
    }

    pub(crate) fn modified_group_key_and_tags_cloned(
        &self,
        txn_idx: TxnIndex,
    ) -> Vec<(T::Key, HashSet<T::Tag>)> {
        forward_on_success_or_skip_rest!(self, txn_idx, resource_group_tags)
    }

    // Extracts a set of resource paths (keys) written or updated during execution from
    // transaction output. The group keys are not included, and the boolean indicates
    // whether the resource is used as an AggregatorV1.
    pub(crate) fn modified_resource_keys(
        &self,
        txn_idx: TxnIndex,
    ) -> Option<impl Iterator<Item = (T::Key, bool)>> {
        self.outputs[txn_idx as usize]
            .load()
            .as_ref()
            .and_then(|txn_output| match txn_output.as_ref() {
                ExecutionStatus::Success(t) | ExecutionStatus::SkipRest(t) => Some(
                    t.resource_write_set()
                        .into_iter()
                        .map(|(k, _, _)| (k, false))
                        .chain(t.aggregator_v1_write_set().into_keys().map(|k| (k, true)))
                        .chain(t.aggregator_v1_delta_set().into_keys().map(|k| (k, true))),
                ),
                ExecutionStatus::Abort(_)
                | ExecutionStatus::SpeculativeExecutionAbortError(_)
                | ExecutionStatus::DelayedFieldsCodeInvariantError(_) => None,
            })
    }

    pub(crate) fn modified_resource_keys_no_aggregator_v1(
        &self,
        txn_idx: TxnIndex,
    ) -> Option<impl Iterator<Item = T::Key>> {
        self.outputs[txn_idx as usize]
            .load()
            .as_ref()
            .and_then(|txn_output| match txn_output.as_ref() {
                ExecutionStatus::Success(t) | ExecutionStatus::SkipRest(t) => {
                    Some(t.resource_write_set().into_iter().map(|(k, _, _)| k))
                },
                ExecutionStatus::Abort(_)
                | ExecutionStatus::SpeculativeExecutionAbortError(_)
                | ExecutionStatus::DelayedFieldsCodeInvariantError(_) => None,
            })
    }

    pub(crate) fn modified_aggregator_v1_keys(
        &self,
        txn_idx: TxnIndex,
    ) -> Option<impl Iterator<Item = T::Key>> {
        self.outputs[txn_idx as usize]
            .load()
            .as_ref()
            .and_then(|txn_output| match txn_output.as_ref() {
                ExecutionStatus::Success(t) | ExecutionStatus::SkipRest(t) => Some(
                    t.aggregator_v1_write_set()
                        .into_keys()
                        .chain(t.aggregator_v1_delta_set().into_keys()),
                ),
                ExecutionStatus::Abort(_)
                | ExecutionStatus::SpeculativeExecutionAbortError(_)
                | ExecutionStatus::DelayedFieldsCodeInvariantError(_) => None,
            })
    }

    pub(crate) fn module_write_set(&self, txn_idx: TxnIndex) -> Vec<ModuleWrite<T::Value>> {
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
            | None => Vec::new(),
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

    pub(crate) fn aggregator_v1_delta_keys(
        &self,
        txn_idx: TxnIndex,
    ) -> Option<impl Iterator<Item = T::Key>> {
        self.outputs[txn_idx as usize]
            .load()
            .as_ref()
            .and_then(|txn_output| match txn_output.as_ref() {
                ExecutionStatus::Success(t) | ExecutionStatus::SkipRest(t) => {
                    Some(t.aggregator_v1_delta_set().into_keys())
                },
                ExecutionStatus::Abort(_)
                | ExecutionStatus::SpeculativeExecutionAbortError(_)
                | ExecutionStatus::DelayedFieldsCodeInvariantError(_) => None,
            })
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
