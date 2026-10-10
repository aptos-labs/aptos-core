// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    captured_reads::TxnInput,
    code_cache_global::{add_module_write_to_module_cache, GlobalModuleCache},
    errors::ParallelBlockExecutionError,
    explicit_sync_wrapper::ExplicitSyncWrapper,
    limit_processor::BlockGasLimitProcessor,
    scheduler_wrapper::SchedulerWrapper,
    task::{ExecutionStatus, TxnOutput},
    types::ReadWriteSummary,
};
use aptos_logger::error;
use aptos_mvhashmap::{types::TxnIndex, MVHashMap};
use aptos_types::{
    error::{code_invariant_error, PanicError, PanicOr},
    on_chain_config::BlockGasLimitType,
    transaction::BlockExecutableTransaction as Transaction,
    vm::modules::AptosModuleExtension,
};
use crossbeam::utils::CachePadded;
use fail::fail_point;
use move_binary_format::CompiledModule;
use move_core_types::language_storage::ModuleId;
use move_vm_runtime::{Module, RuntimeEnvironment};
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
use parking_lot::Mutex;
use std::{
    collections::{BTreeSet, HashSet},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

struct OutputWrapper<O: TxnOutput> {
    output: Option<ExecutionStatus<O>>,
    maybe_read_write_summary: Option<ReadWriteSummary<O::Key, O::Tag>>,
    maybe_approx_output_size: Option<u64>,
}

impl<O: TxnOutput> OutputWrapper<O> {
    fn empty() -> Self {
        Self {
            output: None,
            maybe_read_write_summary: None,
            maybe_approx_output_size: None,
        }
    }

    fn get_output(&self) -> Option<&O> {
        self.output.as_ref().and_then(|status| status.get_output())
    }

    fn from_execution_status<I: TxnInput<Key = O::Key, Tag = O::Tag>>(
        output: ExecutionStatus<O>,
        read_set: &I,
        block_gas_limit_type: &BlockGasLimitType,
        user_txn_bytes_len: u64,
    ) -> Self {
        // Summaries are only meaningful for an executed output.
        let (maybe_approx_output_size, maybe_read_write_summary) = match &output {
            ExecutionStatus::Executed { output, .. } => {
                let maybe_approx_output_size =
                    block_gas_limit_type.block_output_limit().map(|_| {
                        output.output_approx_size()
                            + if block_gas_limit_type.include_user_txn_size_in_block_output() {
                                user_txn_bytes_len
                            } else {
                                0
                            }
                    });
                let maybe_read_write_summary =
                    block_gas_limit_type.conflict_penalty_window().map(|_| {
                        ReadWriteSummary::new(
                            read_set.get_read_summary(),
                            output.get_write_summary(),
                        )
                    });
                (maybe_approx_output_size, maybe_read_write_summary)
            },
            ExecutionStatus::Aborted(_) | ExecutionStatus::SpeculativeFailure => (None, None),
        };

        Self {
            output: Some(output),
            maybe_read_write_summary,
            maybe_approx_output_size,
        }
    }
}

pub struct TxnLastInputOutput<
    T: Transaction,
    I: TxnInput,
    O: TxnOutput<Txn = T, Key = I::Key, Tag = I::Tag>,
> {
    inputs: Vec<CachePadded<Mutex<Option<Arc<I>>>>>, // txn_idx -> input (read set).

    output_wrappers: Vec<CachePadded<Mutex<OutputWrapper<O>>>>,
    // Used to record if the latest incarnation of a txn was a failure due to the
    // speculative nature of parallel execution.
    speculative_failures: Vec<CachePadded<AtomicBool>>,
}

impl<T: Transaction, I: TxnInput, O: TxnOutput<Txn = T, Key = I::Key, Tag = I::Tag>>
    TxnLastInputOutput<T, I, O>
{
    /// num_txns passed here is typically larger than the number of txns in the block,
    /// currently by 1 to account for the block epilogue txn.
    pub fn new(num_txns: TxnIndex) -> Self {
        Self {
            inputs: (0..num_txns)
                .map(|_| CachePadded::new(Mutex::new(None)))
                .collect(),
            output_wrappers: (0..num_txns)
                .map(|_| CachePadded::new(Mutex::new(OutputWrapper::empty())))
                .collect(),
            speculative_failures: (0..num_txns)
                .map(|_| CachePadded::new(AtomicBool::new(false)))
                .collect(),
        }
    }

    pub(crate) fn record(
        &self,
        txn_idx: TxnIndex,
        input: I,
        output: ExecutionStatus<O>,
        block_gas_limit_type: &BlockGasLimitType,
        user_txn_bytes_len: u64,
    ) -> Result<(), PanicError> {
        self.speculative_failures[txn_idx as usize].store(false, Ordering::Relaxed);
        *self.output_wrappers[txn_idx as usize].lock() = OutputWrapper::from_execution_status(
            output,
            &input,
            block_gas_limit_type,
            user_txn_bytes_len,
        );
        *self.inputs[txn_idx as usize].lock() = Some(Arc::new(input));

        Ok(())
    }

    pub(crate) fn record_speculative_failure(&self, txn_idx: TxnIndex) {
        self.speculative_failures[txn_idx as usize].store(true, Ordering::Relaxed);
    }

    pub(crate) fn is_speculative_failure(&self, txn_idx: TxnIndex) -> bool {
        self.speculative_failures[txn_idx as usize].load(Ordering::Relaxed)
    }

    pub(crate) fn read_set(&self, txn_idx: TxnIndex) -> Option<Arc<I>> {
        Some(Arc::clone(self.inputs[txn_idx as usize].lock().as_ref()?))
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
    pub(crate) fn commit(
        &self,
        txn_idx: TxnIndex,
        num_txns: TxnIndex,
        num_workers: usize,
        block_limit_processor: &mut BlockGasLimitProcessor<I::Key, I::Tag>,
        maybe_block_epilogue_txn_idx: &ExplicitSyncWrapper<Option<TxnIndex>>,
        scheduler: &SchedulerWrapper,
    ) -> Result<(), PanicOr<ParallelBlockExecutionError>> {
        let mut output_wrapper = self.output_wrappers[txn_idx as usize].lock();
        let maybe_read_write_summary = output_wrapper.maybe_read_write_summary.take();
        let maybe_approx_output_size = output_wrapper.maybe_approx_output_size;

        // Only an executed transaction can commit: Aborted is a fatal VM error,
        // SpeculativeFailure must have failed validation earlier, and None means
        // nothing was recorded.
        let (output, skips_rest_flag) = match &mut output_wrapper.output {
            Some(ExecutionStatus::Executed { output, skips_rest }) => (&*output, skips_rest),
            Some(ExecutionStatus::Aborted(msg)) => {
                error!(
                    "FatalVMError from parallel execution {:?} at txn {}",
                    msg, txn_idx
                );
                return Err(PanicOr::Or(ParallelBlockExecutionError::FatalVMError));
            },
            Some(ExecutionStatus::SpeculativeFailure) | None => {
                return Err(code_invariant_error(format!(
                    "Transaction {} has no executed output to commit",
                    txn_idx
                ))
                .into());
            },
        };

        // Read the output facts before flipping the disjoint skips_rest flag.
        let has_new_epoch_event = output.has_new_epoch_event();
        let fee_statement = output.fee_statement();

        // For committed txns, calculate the accumulated gas costs.
        block_limit_processor.accumulate_fee_statement(
            fee_statement,
            maybe_read_write_summary,
            maybe_approx_output_size,
        );
        if block_limit_processor.is_hot_state_accumulation_enabled() {
            block_limit_processor
                .accumulate_hot_state_rw(output.storage_keys_written(), output.storage_keys_read());
        }

        let mut must_create_epilogue_txn = if *skips_rest_flag {
            !has_new_epoch_event
        } else {
            txn_idx == num_txns - 1 && !has_new_epoch_event
        };

        if txn_idx < num_txns - 1
            && block_limit_processor.should_end_block_parallel()
            && !*skips_rest_flag
        {
            // The output did not skip the rest; the block gas limit ends the
            // block early here.
            must_create_epilogue_txn |= !has_new_epoch_event;
            *skips_rest_flag = true;
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
        if (txn_idx + 1 == num_txns || *skips_rest_flag) && scheduler.halt() {
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

        if must_create_epilogue_txn {
            *maybe_block_epilogue_txn_idx.acquire().dereference_mut() = Some(txn_idx + 1);
        }

        Ok(())
    }

    pub(crate) fn for_each_resource_key_no_aggregator_v1(
        &self,
        txn_idx: TxnIndex,
        mut callback: impl FnMut(&O::Key) -> Result<(), PanicError>,
    ) -> Result<(), PanicError> {
        let output_wrapper = self.output_wrappers[txn_idx as usize].lock();
        if let Some(output) = output_wrapper.get_output() {
            output.for_each_resource_key(&mut callback)?;
        }

        Ok(())
    }

    /// Returns an error if callback returns an error.
    pub(crate) fn for_each_resource_group_key_and_tags(
        &self,
        txn_idx: TxnIndex,
        mut callback: impl FnMut(&O::Key, HashSet<&O::Tag>) -> Result<(), PanicError>,
    ) -> Result<(), PanicError> {
        let output_wrapper = self.output_wrappers[txn_idx as usize].lock();
        if let Some(output) = output_wrapper.get_output() {
            output.for_each_resource_group_key_and_tags(&mut callback)?;
        }

        Ok(())
    }

    pub(crate) fn modified_group_key_and_tags_cloned(
        &self,
        txn_idx: TxnIndex,
    ) -> Vec<(O::Key, HashSet<O::Tag>)> {
        let output_wrapper = self.output_wrappers[txn_idx as usize].lock();
        match output_wrapper.get_output() {
            Some(output) => output.legacy_v1_resource_group_tags(),
            None => vec![],
        }
    }

    // Extracts a set of resource paths (keys) written or updated during execution from
    // transaction output. The group keys are not included.
    pub(crate) fn modified_resource_keys(
        &self,
        txn_idx: TxnIndex,
    ) -> Option<impl Iterator<Item = O::Key>> {
        let output_wrapper = self.output_wrappers[txn_idx as usize].lock();
        let output = output_wrapper.get_output()?;
        Some(output.resource_write_set().into_keys())
    }

    // The output must be an executed output, o.w. an invariant error is returned.
    pub(crate) fn publish_module_write_set(
        &self,
        txn_idx: TxnIndex,
        global_module_cache: &GlobalModuleCache<
            ModuleId,
            CompiledModule,
            Module,
            AptosModuleExtension,
        >,
        versioned_cache: &MVHashMap<I::Key, I::Tag, I::Value, DelayedFieldID>,
        runtime_environment: &RuntimeEnvironment,
        scheduler: &SchedulerWrapper<'_>,
    ) -> Result<bool, PanicError> {
        let output_wrapper = self.output_wrappers[txn_idx as usize].lock();
        let output = output_wrapper.get_output().ok_or_else(|| {
            code_invariant_error(format!(
                "Module publish requires an executed output, txn {}",
                txn_idx
            ))
        })?;

        let mut published = false;
        let mut module_ids_for_v2 = BTreeSet::new();
        output.for_each_module_write(&mut |module_id, state_value| {
            published = true;
            if scheduler.is_v2() {
                module_ids_for_v2.insert(module_id.clone());
            }
            add_module_write_to_module_cache(
                module_id,
                state_value,
                txn_idx,
                runtime_environment,
                global_module_cache,
                versioned_cache.module_cache(),
            )
        })?;
        if published {
            // Record validation requirements after the modules are published.
            scheduler.record_validation_requirements(txn_idx, module_ids_for_v2)?;
        }
        Ok(published)
    }

    pub(crate) fn delayed_field_keys(
        &self,
        txn_idx: TxnIndex,
    ) -> Option<impl Iterator<Item = DelayedFieldID>> {
        let output_wrapper = self.output_wrappers[txn_idx as usize].lock();
        let output = output_wrapper.get_output()?;
        Some(output.delayed_field_change_set().into_keys())
    }

    /// Takes the recorded output for a committed transaction so it can be
    /// materialized (finalizing resource groups and replacing delayed field ids).
    pub(crate) fn take_committed_output(&self, txn_idx: TxnIndex) -> Result<O, PanicError> {
        match self.output_wrappers[txn_idx as usize].lock().output.take() {
            Some(ExecutionStatus::Executed { output, .. }) => Ok(output),
            Some(ExecutionStatus::Aborted(_))
            | Some(ExecutionStatus::SpeculativeFailure)
            | None => Err(code_invariant_error(
                "[BlockSTM]: Output must be recorded after execution",
            )),
        }
    }
}
