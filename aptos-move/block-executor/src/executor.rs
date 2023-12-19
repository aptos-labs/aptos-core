// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    counters,
    counters::{
        PARALLEL_EXECUTION_SECONDS, RAYON_EXECUTION_SECONDS, TASK_EXECUTE_SECONDS,
        TASK_VALIDATE_SECONDS, VM_INIT_SECONDS, WORK_WITH_TASK_SECONDS,
    },
    errors::*,
    explicit_sync_wrapper::ExplicitSyncWrapper,
    limit_processor::BlockGasLimitProcessor,
    scheduler::{DependencyStatus, ExecutionTaskType, Scheduler, SchedulerTask, Wave},
    task::{ExecutionStatus, ExecutorTask, TransactionOutput},
    txn_commit_hook::TransactionCommitHook,
    txn_last_input_output::{KeyKind, TxnLastInputOutput},
    types::ReadWriteSummary,
    view::{LatestView, ParallelState, SequentialState, ViewState},
};
use aptos_aggregator::{
    delayed_change::{ApplyBase, DelayedChange},
    delta_change_set::serialize,
    types::{code_invariant_error, expect_ok, PanicOr},
};
use aptos_drop_helper::DEFAULT_DROPPER;
use aptos_logger::{debug, error, info};
use aptos_mvhashmap::{
    types::{Incarnation, MVDataOutput, MVDelayedFieldsError, TxnIndex, ValueWithLayout},
    unsync_map::UnsyncMap,
    versioned_delayed_fields::CommitError,
    MVHashMap,
};
use aptos_types::{
    aggregator::PanicError,
    block_executor::config::BlockExecutorConfig,
    contract_event::TransactionEvent,
    executable::Executable,
    on_chain_config::BlockGasLimitType,
    state_store::TStateView,
    transaction::{BlockExecutableTransaction as Transaction, BlockOutput},
    write_set::{TransactionWrite, WriteOp},
};
use aptos_vm_logging::{clear_speculative_txn_logs, init_speculative_logs};
use bytes::Bytes;
use claims::assert_none;
use core::panic;
use move_core_types::value::MoveTypeLayout;
use num_cpus;
use rand::{thread_rng, Rng};
use rayon::ThreadPool;
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap, HashSet},
    marker::{PhantomData, Sync},
    sync::{atomic::AtomicU32, Arc},
};

pub struct BlockExecutor<T, E, S, L, X> {
    // Number of active concurrent tasks, corresponding to the maximum number of rayon
    // threads that may be concurrently participating in parallel execution.
    config: BlockExecutorConfig,
    executor_thread_pool: Arc<ThreadPool>,
    transaction_commit_hook: Option<L>,
    phantom: PhantomData<(T, E, S, L, X)>,
}

impl<T, E, S, L, X> BlockExecutor<T, E, S, L, X>
where
    T: Transaction,
    E: ExecutorTask<Txn = T>,
    S: TStateView<Key = T::Key> + Sync,
    L: TransactionCommitHook<Output = E::Output>,
    X: Executable + 'static,
{
    /// The caller needs to ensure that concurrency_level > 1 (0 is illegal and 1 should
    /// be handled by sequential execution) and that concurrency_level <= num_cpus.
    pub fn new(
        config: BlockExecutorConfig,
        executor_thread_pool: Arc<ThreadPool>,
        transaction_commit_hook: Option<L>,
    ) -> Self {
        assert!(
            config.local.concurrency_level > 0 && config.local.concurrency_level <= num_cpus::get(),
            "Parallel execution concurrency level {} should be between 1 and number of CPUs",
            config.local.concurrency_level
        );
        Self {
            config,
            executor_thread_pool,
            transaction_commit_hook,
            phantom: PhantomData,
        }
    }

    fn execute(
        idx_to_execute: TxnIndex,
        incarnation: Incarnation,
        signature_verified_block: &[T],
        last_input_output: &TxnLastInputOutput<T, E::Output, E::Error>,
        versioned_cache: &MVHashMap<T::Key, T::Tag, T::Value, X, T::Identifier>,
        executor: &E,
        base_view: &S,
        latest_view: ParallelState<T, X>,
    ) -> ::std::result::Result<bool, PanicOr<IntentionalFallbackToSequential>> {
        let _timer = TASK_EXECUTE_SECONDS.start_timer();
        let txn = &signature_verified_block[idx_to_execute as usize];

        // VM execution.
        let sync_view = LatestView::new(base_view, ViewState::Sync(latest_view), idx_to_execute);
        let execute_result = executor.execute_transaction(&sync_view, txn, idx_to_execute);

        let mut prev_modified_keys = last_input_output
            .modified_keys(idx_to_execute)
            .map_or(HashMap::new(), |keys| keys.collect());

        let mut prev_modified_delayed_fields = last_input_output
            .delayed_field_keys(idx_to_execute)
            .map_or(HashSet::new(), |keys| keys.collect());

        let mut read_set = sync_view.take_parallel_reads();

        // For tracking whether the recent execution wrote outside of the previous write/delta set.
        let mut updates_outside = false;
        let mut apply_updates = |output: &E::Output| -> ::std::result::Result<(), PanicError> {
            for (group_key, group_metadata_op, group_ops) in
                output.resource_group_write_set().into_iter()
            {
                if prev_modified_keys.remove(&group_key).is_none() {
                    // Previously no write to the group at all.
                    updates_outside = true;
                }

                versioned_cache.data().write(
                    group_key.clone(),
                    idx_to_execute,
                    incarnation,
                    // Group metadata op needs no layout (individual resources in groups do).
                    (group_metadata_op, None),
                );
                if versioned_cache.group_data().write(
                    group_key,
                    idx_to_execute,
                    incarnation,
                    group_ops.into_iter(),
                ) {
                    // Should return true if writes outside.
                    updates_outside = true;
                }
            }

            // Then, process resource & aggregator_v1 & module writes.
            for (k, v) in output.resource_write_set().into_iter().chain(
                output
                    .aggregator_v1_write_set()
                    .into_iter()
                    .map(|(state_key, write_op)| (state_key, (write_op, None))),
            ) {
                if prev_modified_keys.remove(&k).is_none() {
                    updates_outside = true;
                }
                versioned_cache
                    .data()
                    .write(k, idx_to_execute, incarnation, v);
            }

            for (k, v) in output.module_write_set().into_iter() {
                if prev_modified_keys.remove(&k).is_none() {
                    updates_outside = true;
                }
                versioned_cache.modules().write(k, idx_to_execute, v);
            }

            // Then, apply deltas.
            for (k, d) in output.aggregator_v1_delta_set().into_iter() {
                if prev_modified_keys.remove(&k).is_none() {
                    updates_outside = true;
                }
                versioned_cache.data().add_delta(k, idx_to_execute, d);
            }

            let delayed_field_change_set = output.delayed_field_change_set();

            // TODO[agg_v2](optimize): see if/how we want to incorporate DeltaHistory from read set into versoined_delayed_fields.
            // Without that, currently materialized reads cannot check history and fail early.
            //
            // We can extract histories with something like the code below,
            // and then change change.into_entry_no_additional_history() to include history.
            //
            // for id in read_set.get_delayed_field_keys() {
            //     if !delayed_field_change_set.contains_key(id) {
            //         let read_value = read_set.get_delayed_field_by_kind(id, DelayedFieldReadKind::Bounded).unwrap();
            //     }
            // }

            for (id, change) in delayed_field_change_set.into_iter() {
                prev_modified_delayed_fields.remove(&id);

                let entry = change.into_entry_no_additional_history();

                // TODO[agg_v2](optimize): figure out if it is useful for change to update updates_outside
                if let Err(e) =
                    versioned_cache
                        .delayed_fields()
                        .record_change(id, idx_to_execute, entry)
                {
                    match e {
                        PanicOr::CodeInvariantError(m) => {
                            return Err(code_invariant_error(format!(
                                "Record change failed with CodeInvariantError: {:?}",
                                m
                            )));
                        },
                        PanicOr::Or(_) => {
                            read_set.capture_delayed_field_read_error(&PanicOr::Or(
                                MVDelayedFieldsError::DeltaApplicationFailure,
                            ));
                        },
                    };
                }
            }
            Ok(())
        };

        let result = match execute_result {
            // These statuses are the results of speculative execution, so even for
            // SkipRest (skip the rest of transactions) and Abort (abort execution with
            // user defined error), no immediate action is taken. Instead the statuses
            // are recorded and (final statuses) are analyzed when the block is executed.
            ExecutionStatus::Success(output) => {
                // Apply the writes/deltas to the versioned_data_cache.
                apply_updates(&output)?;
                ExecutionStatus::Success(output)
            },
            ExecutionStatus::SkipRest(output) => {
                // Apply the writes/deltas and record status indicating skip.
                apply_updates(&output)?;
                ExecutionStatus::SkipRest(output)
            },
            ExecutionStatus::Abort(err) => {
                // Record the status indicating abort.
                ExecutionStatus::Abort(Error::UserError(err))
            },
            ExecutionStatus::DirectWriteSetTransactionNotCapableError => {
                // TODO[agg_v2](fix) decide how to handle/propagate.
                panic!("PayloadWriteSet::Direct transaction not alone in a block");
            },
            ExecutionStatus::SpeculativeExecutionAbortError(msg) => {
                read_set.capture_delayed_field_read_error(&PanicOr::Or(
                    MVDelayedFieldsError::DeltaApplicationFailure,
                ));
                ExecutionStatus::SpeculativeExecutionAbortError(msg)
            },
            ExecutionStatus::DelayedFieldsCodeInvariantError(msg) => {
                return Err(code_invariant_error(format!(
                    "Transaction execution failed with DelayedFieldsCodeInvariantError: {:?}",
                    msg
                ))
                .into());
            },
        };

        // Remove entries from previous write/delta set that were not overwritten.
        for (k, kind) in prev_modified_keys {
            use KeyKind::*;
            match kind {
                Resource => versioned_cache.data().remove(&k, idx_to_execute),
                Module => versioned_cache.modules().remove(&k, idx_to_execute),
                Group => {
                    versioned_cache.data().remove(&k, idx_to_execute);
                    versioned_cache.group_data().remove(&k, idx_to_execute);
                },
            };
        }

        for id in prev_modified_delayed_fields {
            versioned_cache.delayed_fields().remove(&id, idx_to_execute);
        }

        if !last_input_output.record(idx_to_execute, read_set, result) {
            return Err(PanicOr::Or(
                IntentionalFallbackToSequential::ModulePathReadWrite,
            ));
        }
        Ok(updates_outside)
    }

    fn validate(
        idx_to_validate: TxnIndex,
        last_input_output: &TxnLastInputOutput<T, E::Output, E::Error>,
        versioned_cache: &MVHashMap<T::Key, T::Tag, T::Value, X, T::Identifier>,
    ) -> ::std::result::Result<bool, PanicError> {
        let _timer = TASK_VALIDATE_SECONDS.start_timer();
        let read_set = last_input_output
            .read_set(idx_to_validate)
            .expect("[BlockSTM]: Prior read-set must be recorded");

        if read_set.is_incorrect_use() {
            return Err(code_invariant_error(
                "Incorrect use detected in CapturedReads",
            ));
        }

        // Note: we validate delayed field reads only at try_commit.
        // TODO[agg_v2](optimize): potentially add some basic validation.
        // TODO[agg_v2](optimize): potentially add more sophisticated validation, but if it fails,
        // we mark it as a soft failure, requires some new statuses in the scheduler
        // (i.e. not re-execute unless some other part of the validation fails or
        // until commit, but mark as estimates).

        // TODO: validate modules when there is no r/w fallback.
        Ok(
            read_set.validate_data_reads(versioned_cache.data(), idx_to_validate)
                && read_set.validate_group_reads(versioned_cache.group_data(), idx_to_validate),
        )
    }

    fn update_transaction_on_abort(
        txn_idx: TxnIndex,
        last_input_output: &TxnLastInputOutput<T, E::Output, E::Error>,
        versioned_cache: &MVHashMap<T::Key, T::Tag, T::Value, X, T::Identifier>,
    ) {
        counters::SPECULATIVE_ABORT_COUNT.inc();

        // Any logs from the aborted execution should be cleared and not reported.
        clear_speculative_txn_logs(txn_idx as usize);

        // Not valid and successfully aborted, mark the latest write/delta sets as estimates.
        if let Some(keys) = last_input_output.modified_keys(txn_idx) {
            for (k, kind) in keys {
                use KeyKind::*;
                match kind {
                    Resource => versioned_cache.data().mark_estimate(&k, txn_idx),
                    Module => versioned_cache.modules().mark_estimate(&k, txn_idx),
                    Group => {
                        versioned_cache.data().mark_estimate(&k, txn_idx);
                        versioned_cache.group_data().mark_estimate(&k, txn_idx);
                    },
                };
            }
        }

        if let Some(keys) = last_input_output.delayed_field_keys(txn_idx) {
            for k in keys {
                versioned_cache.delayed_fields().mark_estimate(&k, txn_idx);
            }
        }
    }

    fn update_on_validation(
        txn_idx: TxnIndex,
        incarnation: Incarnation,
        valid: bool,
        validation_wave: Wave,
        last_input_output: &TxnLastInputOutput<T, E::Output, E::Error>,
        versioned_cache: &MVHashMap<T::Key, T::Tag, T::Value, X, T::Identifier>,
        scheduler: &Scheduler,
    ) -> SchedulerTask {
        let aborted = !valid && scheduler.try_abort(txn_idx, incarnation);

        if aborted {
            Self::update_transaction_on_abort(txn_idx, last_input_output, versioned_cache);
            scheduler.finish_abort(txn_idx, incarnation)
        } else {
            scheduler.finish_validation(txn_idx, validation_wave);

            if valid {
                scheduler.queueing_commits_arm();
            }

            SchedulerTask::NoTask
        }
    }

    fn validate_commit_ready(
        txn_idx: TxnIndex,
        versioned_cache: &MVHashMap<T::Key, T::Tag, T::Value, X, T::Identifier>,
        last_input_output: &TxnLastInputOutput<T, E::Output, E::Error>,
    ) -> ::std::result::Result<bool, PanicError> {
        let read_set = last_input_output
            .read_set(txn_idx)
            .expect("Read set must be recorded");

        let mut execution_still_valid =
            read_set.validate_delayed_field_reads(versioned_cache.delayed_fields(), txn_idx)?;

        if execution_still_valid {
            if let Some(delayed_field_ids) = last_input_output.delayed_field_keys(txn_idx) {
                if let Err(e) = versioned_cache
                    .delayed_fields()
                    .try_commit(txn_idx, delayed_field_ids.collect())
                {
                    match e {
                        CommitError::ReExecutionNeeded(_) => {
                            execution_still_valid = false;
                        },
                        CommitError::CodeInvariantError(msg) => {
                            return Err(code_invariant_error(msg));
                        },
                    }
                }
            }
        }
        Ok(execution_still_valid)
    }

    /// This method may be executed by different threads / workers, but is guaranteed to be executed
    /// non-concurrently by the scheduling in parallel executor. This allows to perform light logic
    /// related to committing a transaction in a simple way and without excessive synchronization
    /// overhead. On the other hand, materialization that happens after commit (& after this method)
    /// is concurrent and deals with logic such as patching delayed fields / resource groups
    /// in outputs, which is heavier (due to serialization / deserialization, copies, etc). Moreover,
    /// since prepare_and_queue_commit_ready_txns takes care of synchronization in the flag-combining
    /// way, the materialization can be almost embarassingly parallelizable.
    fn prepare_and_queue_commit_ready_txns(
        &self,
        block_gas_limit_type: &BlockGasLimitType,
        scheduler: &Scheduler,
        versioned_cache: &MVHashMap<T::Key, T::Tag, T::Value, X, T::Identifier>,
        scheduler_task: &mut SchedulerTask,
        last_input_output: &TxnLastInputOutput<T, E::Output, E::Error>,
        shared_commit_state: &ExplicitSyncWrapper<(
            BlockGasLimitProcessor<T>,
            Option<Error<E::Error>>,
        )>,
        base_view: &S,
        start_shared_counter: u32,
        shared_counter: &AtomicU32,
        executor: &E,
        block: &[T],
    ) -> ::std::result::Result<(), PanicOr<IntentionalFallbackToSequential>> {
        let mut shared_commit_state_guard = shared_commit_state.acquire();
        let (block_limit_processor, shared_maybe_error) =
            shared_commit_state_guard.dereference_mut();

        while let Some((txn_idx, incarnation)) = scheduler.try_commit() {
            if !Self::validate_commit_ready(txn_idx, versioned_cache, last_input_output)? {
                // Transaction needs to be re-executed, one final time.

                Self::update_transaction_on_abort(txn_idx, last_input_output, versioned_cache);
                // We are going to skip reducing validation index here, as we
                // are executing immediately, and will reduce it unconditionally
                // after execution, inside finish_execution_during_commit.
                // Because of that, we can also ignore _updates_outside result.
                let _updates_outside = Self::execute(
                    txn_idx,
                    incarnation + 1,
                    block,
                    last_input_output,
                    versioned_cache,
                    executor,
                    base_view,
                    ParallelState::new(
                        versioned_cache,
                        scheduler,
                        start_shared_counter,
                        shared_counter,
                    ),
                )?;

                scheduler.finish_execution_during_commit(txn_idx);

                let validation_result =
                    Self::validate(txn_idx, last_input_output, versioned_cache)?;
                if !validation_result
                    || !Self::validate_commit_ready(txn_idx, versioned_cache, last_input_output)
                        .unwrap_or(false)
                {
                    return Err(code_invariant_error(format!(
                        "Validation after re-execution failed for {} txn, validate() = {}",
                        txn_idx, validation_result
                    ))
                    .into());
                }
            }

            defer! {
                scheduler.add_to_commit_queue(txn_idx);
            }

            if let Some(fee_statement) = last_input_output.fee_statement(txn_idx) {
                let approx_output_size = block_gas_limit_type.block_output_limit().and_then(|_| {
                    last_input_output
                        .output_approx_size(txn_idx)
                        .map(|approx_output| {
                            approx_output
                                + if block_gas_limit_type.include_user_txn_size_in_block_output() {
                                    block[txn_idx as usize].user_txn_bytes_len()
                                } else {
                                    0
                                } as u64
                        })
                });
                let txn_read_write_summary = block_gas_limit_type
                    .conflict_penalty_window()
                    .map(|_| last_input_output.get_txn_read_write_summary(txn_idx));

                // For committed txns with Success status, calculate the accumulated gas costs.
                block_limit_processor.accumulate_fee_statement(
                    fee_statement,
                    txn_read_write_summary,
                    approx_output_size,
                );

                if txn_idx < scheduler.num_txns() - 1
                    && block_limit_processor.should_end_block_parallel()
                {
                    // Set the execution output status to be SkipRest, to skip the rest of the txns.
                    last_input_output.update_to_skip_rest(txn_idx);
                }
            }

            let process_finalized_group =
                |finalized_group: anyhow::Result<Vec<(T::Tag, ValueWithLayout<T::Value>)>>,
                 metadata_is_deletion: bool|
                 -> Result<_, _> {
                    match finalized_group {
                        Ok(finalized_group) => {
                            // finalize_group already applies the deletions.
                            if finalized_group.is_empty() != metadata_is_deletion {
                                return Err(Error::FallbackToSequential(resource_group_error(
                                    format!(
                                "Group is empty = {} but op is deletion = {} in parallel execution",
                                finalized_group.is_empty(),
                                metadata_is_deletion
                            ),
                                )));
                            }
                            Ok(finalized_group)
                        },
                        Err(e) => Err(Error::FallbackToSequential(resource_group_error(format!(
                            "Error committing resource group {:?}",
                            e
                        )))),
                    }
                };

            let group_metadata_ops = last_input_output.group_metadata_ops(txn_idx);
            let mut finalized_groups = Vec::with_capacity(group_metadata_ops.len());
            let mut maybe_err = None;
            for (group_key, metadata_op) in group_metadata_ops.into_iter() {
                // finalize_group copies Arc of values and the Tags (TODO: optimize as needed).
                let finalized_result = versioned_cache
                    .group_data()
                    .finalize_group(&group_key, txn_idx);
                match process_finalized_group(finalized_result, metadata_op.is_deletion()) {
                    Ok(finalized_group) => {
                        finalized_groups.push((group_key, metadata_op, finalized_group));
                    },
                    Err(err) => {
                        maybe_err = Some(err);
                        break;
                    },
                }
                if maybe_err.is_some() {
                    break;
                }
            }

            if maybe_err.is_none() {
                if let Some(group_reads_needing_delayed_field_exchange) =
                    last_input_output.group_reads_needing_delayed_field_exchange(txn_idx)
                {
                    for (group_key, metadata_op) in
                        group_reads_needing_delayed_field_exchange.into_iter()
                    {
                        let finalized_result = versioned_cache
                            .group_data()
                            .get_last_committed_group(&group_key);
                        match process_finalized_group(finalized_result, metadata_op.is_deletion()) {
                            Ok(finalized_group) => {
                                finalized_groups.push((group_key, metadata_op, finalized_group));
                            },
                            Err(err) => {
                                maybe_err = Some(err);
                                break;
                            },
                        }
                        if maybe_err.is_some() {
                            break;
                        }
                    }
                }
            }

            last_input_output.record_finalized_group(txn_idx, finalized_groups);

            maybe_err = maybe_err.or_else(|| last_input_output.maybe_execution_error(txn_idx));

            // We `halt` the execution in the following 4 cases:
            // 1) We detect module read/write intersection
            // 2) A transaction triggered an Abort
            // 3) All transactions are scheduled for committing
            // 4) We skip_rest after a transaction

            // We cover cases 1 and 2 here
            if let Some(err) = maybe_err {
                *shared_maybe_error = Some(err);
                if scheduler.halt() {
                    info!(
                        "Block execution was aborted due to {:?}",
                        shared_maybe_error.as_ref().unwrap()
                    );
                    block_limit_processor.finish_parallel_update_counters_and_log_info(
                        txn_idx + 1,
                        scheduler.num_txns(),
                    );
                } // else it's already halted
                break;
            }

            // We cover cases 3 and 4 here: Either all txn committed,
            // or a committed txn caused an early halt.
            if txn_idx + 1 == scheduler.num_txns()
                || last_input_output.block_skips_rest_at_idx(txn_idx)
            {
                if txn_idx + 1 == scheduler.num_txns() {
                    assert!(
                        !matches!(scheduler_task, SchedulerTask::ExecutionTask(_, _, _)),
                        "All transactions can be committed, can't have execution task"
                    );
                }

                if scheduler.halt() {
                    block_limit_processor.finish_parallel_update_counters_and_log_info(
                        txn_idx + 1,
                        scheduler.num_txns(),
                    );
                }
                break;
            }
        }
        Ok(())
    }

    // For each delayed field in resource write set, replace the identifiers with values.
    fn map_id_to_values_in_write_set(
        resource_write_set: Option<Vec<(T::Key, (T::Value, Option<Arc<MoveTypeLayout>>))>>,
        latest_view: &LatestView<T, S, X>,
    ) -> BTreeMap<T::Key, T::Value> {
        let mut patched_resource_write_set = BTreeMap::new();
        if let Some(resource_write_set) = resource_write_set {
            for (key, (write_op, layout)) in resource_write_set.into_iter() {
                // layout is Some(_) if it contains a delayed field
                if let Some(layout) = layout {
                    if !write_op.is_deletion() {
                        match write_op.bytes() {
                            // TODO[agg_v2](fix): propagate error
                            None => unreachable!(),
                            Some(write_op_bytes) => {
                                let patched_bytes = match latest_view
                                    .replace_identifiers_with_values(write_op_bytes, &layout)
                                {
                                    Ok((bytes, _)) => bytes,
                                    Err(_) => {
                                        unreachable!("Failed to replace identifiers with values")
                                    },
                                };
                                let mut patched_write_op = write_op;
                                patched_write_op.set_bytes(patched_bytes);
                                patched_resource_write_set.insert(key, patched_write_op);
                            },
                        }
                    }
                }
            }
        }
        patched_resource_write_set
    }

    fn map_id_to_values_in_group_writes(
        finalized_groups: Vec<(T::Key, T::Value, Vec<(T::Tag, ValueWithLayout<T::Value>)>)>,
        latest_view: &LatestView<T, S, X>,
    ) -> Vec<(T::Key, T::Value, Vec<(T::Tag, Arc<T::Value>)>)> {
        let mut patched_finalized_groups = Vec::with_capacity(finalized_groups.len());
        for (group_key, group_metadata_op, resource_vec) in finalized_groups.into_iter() {
            let mut patched_resource_vec = Vec::with_capacity(resource_vec.len());
            for (tag, value_with_layout) in resource_vec.into_iter() {
                let value = match value_with_layout {
                    ValueWithLayout::RawFromStorage(value) => value,
                    ValueWithLayout::Exchanged(value, None) => value,
                    ValueWithLayout::Exchanged(value, Some(layout)) => Arc::new(
                        Self::replace_ids_with_values(&value, layout.as_ref(), latest_view),
                    ),
                };
                patched_resource_vec.push((tag, value));
            }
            patched_finalized_groups.push((group_key, group_metadata_op, patched_resource_vec));
        }
        patched_finalized_groups
    }

    // Parse the input `value` and replace delayed field identifiers with
    // corresponding values
    fn replace_ids_with_values(
        value: &T::Value,
        layout: &MoveTypeLayout,
        latest_view: &LatestView<T, S, X>,
    ) -> T::Value {
        if let Some(mut value) = value.convert_read_to_modification() {
            if let Some(value_bytes) = value.bytes() {
                let (patched_bytes, _) = latest_view
                    .replace_identifiers_with_values(value_bytes, layout)
                    .unwrap();
                value.set_bytes(patched_bytes);
                value
            } else {
                unreachable!("Value to be exchanged doesn't have bytes: {:?}", value)
            }
        } else {
            unreachable!(
                "Value to be exchanged cannot be transformed to modification: {:?}",
                value
            );
        }
    }

    // For each delayed field in the event, replace delayed field identifier with value.
    fn map_id_to_values_events(
        events: Box<dyn Iterator<Item = (T::Event, Option<MoveTypeLayout>)>>,
        latest_view: &LatestView<T, S, X>,
    ) -> Vec<T::Event> {
        let mut patched_events = vec![];
        for (event, layout) in events {
            if let Some(layout) = layout {
                let event_data = event.get_event_data();
                match latest_view
                    .replace_identifiers_with_values(&Bytes::from(event_data.to_vec()), &layout)
                {
                    Ok((bytes, _)) => {
                        let mut patched_event = event;
                        patched_event.set_event_data(bytes.to_vec());
                        patched_events.push(patched_event);
                    },
                    Err(_) => unreachable!("Failed to replace identifiers with values in event"),
                }
            } else {
                patched_events.push(event);
            }
        }
        patched_events
    }

    fn materialize_aggregator_v1_delta_writes(
        txn_idx: TxnIndex,
        last_input_output: &TxnLastInputOutput<T, E::Output, E::Error>,
        versioned_cache: &MVHashMap<T::Key, T::Tag, T::Value, X, T::Identifier>,
        base_view: &S,
    ) -> Vec<(T::Key, WriteOp)> {
        // Materialize all the aggregator v1 deltas.
        let aggregator_v1_delta_keys = last_input_output.aggregator_v1_delta_keys(txn_idx);
        let mut aggregator_v1_delta_writes = Vec::with_capacity(aggregator_v1_delta_keys.len());
        for k in aggregator_v1_delta_keys.into_iter() {
            // Note that delta materialization happens concurrently, but under concurrent
            // commit_hooks (which may be dispatched by the coordinator), threads may end up
            // contending on delta materialization of the same aggregator. However, the
            // materialization is based on previously materialized values and should not
            // introduce long critical sections. Moreover, with more aggregators, and given
            // that the commit_hook will be performed at dispersed times based on the
            // completion of the respective previous tasks of threads, this should not be
            // an immediate bottleneck - confirmed by an experiment with 32 core and a
            // single materialized aggregator. If needed, the contention may be further
            // mitigated by batching consecutive commit_hooks.
            let committed_delta = versioned_cache
                .data()
                .materialize_delta(&k, txn_idx)
                .unwrap_or_else(|op| {
                    // TODO[agg_v1](cleanup): this logic should improve with the new AGGR data structure
                    // TODO[agg_v1](cleanup): and the ugly base_view parameter will also disappear.
                    let storage_value = base_view
                        .get_state_value(&k)
                        .expect("Error reading the base value for committed delta in storage");

                    let w: T::Value = TransactionWrite::from_state_value(storage_value);
                    let value_u128 = w
                        .as_u128()
                        .expect("Aggregator base value deserialization error")
                        .expect("Aggregator base value must exist");

                    versioned_cache
                        .data()
                        .set_base_value(k.clone(), ValueWithLayout::RawFromStorage(Arc::new(w)));
                    op.apply_to(value_u128)
                        .expect("Materializing delta w. base value set must succeed")
                });

            // Must contain committed value as we set the base value above.
            aggregator_v1_delta_writes.push((
                k,
                WriteOp::legacy_modification(serialize(&committed_delta).into()),
            ));
        }
        aggregator_v1_delta_writes
    }

    fn serialize_groups(
        finalized_groups: Vec<(T::Key, T::Value, Vec<(T::Tag, Arc<T::Value>)>)>,
    ) -> ::std::result::Result<Vec<(T::Key, T::Value)>, PanicOr<IntentionalFallbackToSequential>>
    {
        finalized_groups
            .into_iter()
            .map(|(group_key, mut metadata_op, finalized_group)| {
                let btree: BTreeMap<T::Tag, Bytes> = finalized_group
                    .into_iter()
                    // TODO[agg_v2](fix): Should anything be done using the layout here?
                    .map(|(resource_tag, arc_v)| {
                        let bytes = arc_v
                            .extract_raw_bytes()
                            .expect("Deletions should already be applied");
                        (resource_tag, bytes)
                    })
                    .collect();

                bcs::to_bytes(&btree)
                    .map_err(|e| {
                        resource_group_error(format!("Unexpected resource group error {:?}", e))
                    })
                    .map(|group_bytes| {
                        metadata_op.set_bytes(group_bytes.into());
                        (group_key, metadata_op)
                    })
            })
            .collect()
    }

    fn materialize_txn_commit(
        &self,
        txn_idx: TxnIndex,
        versioned_cache: &MVHashMap<T::Key, T::Tag, T::Value, X, T::Identifier>,
        scheduler: &Scheduler,
        start_shared_counter: u32,
        shared_counter: &AtomicU32,
        last_input_output: &TxnLastInputOutput<T, E::Output, E::Error>,
        base_view: &S,
        final_results: &ExplicitSyncWrapper<Vec<E::Output>>,
    ) -> ::std::result::Result<(), PanicOr<IntentionalFallbackToSequential>> {
        let parallel_state = ParallelState::<T, X>::new(
            versioned_cache,
            scheduler,
            start_shared_counter,
            shared_counter,
        );
        let latest_view = LatestView::new(base_view, ViewState::Sync(parallel_state), txn_idx);
        let resource_write_set = last_input_output.resource_write_set(txn_idx);
        let finalized_groups = last_input_output.take_finalized_group(txn_idx);

        let mut patched_resource_write_set =
            Self::map_id_to_values_in_write_set(resource_write_set, &latest_view);

        if let Some(reads_needing_delayed_field_exchange) =
            last_input_output.reads_needing_delayed_field_exchange(txn_idx)
        {
            for (key, layout) in reads_needing_delayed_field_exchange.into_iter() {
                if let Ok(MVDataOutput::Versioned(
                    _,
                    ValueWithLayout::Exchanged(value, _existing_layout),
                )) = versioned_cache.data().fetch_data(&key, txn_idx)
                {
                    // TODO[agg_v2](fix) add randomly_check_layout_matches(Some(_existing_layout), layout);
                    patched_resource_write_set.insert(
                        key,
                        Self::replace_ids_with_values(&value, layout.as_ref(), &latest_view),
                    );
                }
            }
        }

        let patched_finalized_groups =
            Self::map_id_to_values_in_group_writes(finalized_groups, &latest_view);

        let events = last_input_output.events(txn_idx);
        let patched_events = Self::map_id_to_values_events(events, &latest_view);
        let aggregator_v1_delta_writes = Self::materialize_aggregator_v1_delta_writes(
            txn_idx,
            last_input_output,
            versioned_cache,
            base_view,
        );

        let serialized_groups = Self::serialize_groups(patched_finalized_groups)?;

        last_input_output.record_materialized_txn_output(
            txn_idx,
            aggregator_v1_delta_writes,
            patched_resource_write_set
                .into_iter()
                .chain(serialized_groups)
                .collect(),
            patched_events,
        );
        if let Some(txn_commit_listener) = &self.transaction_commit_hook {
            let txn_output = last_input_output.txn_output(txn_idx).unwrap();
            let execution_status = txn_output.output_status();

            match execution_status {
                ExecutionStatus::Success(output) | ExecutionStatus::SkipRest(output) => {
                    txn_commit_listener.on_transaction_committed(txn_idx, output);
                },
                ExecutionStatus::Abort(_) => {
                    txn_commit_listener.on_execution_aborted(txn_idx);
                },
                ExecutionStatus::DirectWriteSetTransactionNotCapableError => {
                    // This should already be handled and fallback to sequential called,
                    // such a transaction should never reach this point.
                    panic!("Cannot be materializing with DirectWriteSetTransactionNotCapableError");
                },
                ExecutionStatus::SpeculativeExecutionAbortError(msg)
                | ExecutionStatus::DelayedFieldsCodeInvariantError(msg) => {
                    panic!("Cannot be materializing with {}", msg);
                },
            }
        }

        let mut final_results = final_results.acquire();
        match last_input_output.take_output(txn_idx) {
            ExecutionStatus::Success(t) | ExecutionStatus::SkipRest(t) => {
                final_results[txn_idx as usize] = t;
            },
            ExecutionStatus::Abort(_) => (),
            ExecutionStatus::DirectWriteSetTransactionNotCapableError => {
                panic!("Cannot be materializing with DirectWriteSetTransactionNotCapableError");
                // This should already be handled and fallback to sequential called,
                // such a transaction should never reach this point.
            },
            ExecutionStatus::SpeculativeExecutionAbortError(msg)
            | ExecutionStatus::DelayedFieldsCodeInvariantError(msg) => {
                panic!("Cannot be materializing with {}", msg);
            },
        };
        Ok(())
    }

    fn worker_loop(
        &self,
        executor_arguments: &E::Argument,
        block: &[T],
        last_input_output: &TxnLastInputOutput<T, E::Output, E::Error>,
        versioned_cache: &MVHashMap<T::Key, T::Tag, T::Value, X, T::Identifier>,
        scheduler: &Scheduler,
        // TODO: should not need to pass base view.
        base_view: &S,
        start_shared_counter: u32,
        shared_counter: &AtomicU32,
        shared_commit_state: &ExplicitSyncWrapper<(
            BlockGasLimitProcessor<T>,
            Option<Error<E::Error>>,
        )>,
        final_results: &ExplicitSyncWrapper<Vec<E::Output>>,
    ) -> ::std::result::Result<(), PanicOr<IntentionalFallbackToSequential>> {
        // Make executor for each task. TODO: fast concurrent executor.
        let init_timer = VM_INIT_SECONDS.start_timer();
        let executor = E::init(*executor_arguments);
        drop(init_timer);

        let _timer = WORK_WITH_TASK_SECONDS.start_timer();
        let mut scheduler_task = SchedulerTask::NoTask;

        let drain_commit_queue =
            || -> ::std::result::Result<(), PanicOr<IntentionalFallbackToSequential>> {
                while let Ok(txn_idx) = scheduler.pop_from_commit_queue() {
                    self.materialize_txn_commit(
                        txn_idx,
                        versioned_cache,
                        scheduler,
                        start_shared_counter,
                        shared_counter,
                        last_input_output,
                        base_view,
                        final_results,
                    )?;
                }
                Ok(())
            };

        loop {
            // Priorotize committing validated transactions
            while scheduler.should_coordinate_commits() {
                self.prepare_and_queue_commit_ready_txns(
                    &self.config.onchain.block_gas_limit_type,
                    scheduler,
                    versioned_cache,
                    &mut scheduler_task,
                    last_input_output,
                    shared_commit_state,
                    base_view,
                    start_shared_counter,
                    shared_counter,
                    &executor,
                    block,
                )?;
                scheduler.queueing_commits_mark_done();
            }

            drain_commit_queue()?;

            scheduler_task = match scheduler_task {
                SchedulerTask::ValidationTask(txn_idx, incarnation, wave) => {
                    let valid = Self::validate(txn_idx, last_input_output, versioned_cache)?;
                    Self::update_on_validation(
                        txn_idx,
                        incarnation,
                        valid,
                        wave,
                        last_input_output,
                        versioned_cache,
                        scheduler,
                    )
                },
                SchedulerTask::ExecutionTask(
                    txn_idx,
                    incarnation,
                    ExecutionTaskType::Execution,
                ) => {
                    let updates_outside = Self::execute(
                        txn_idx,
                        incarnation,
                        block,
                        last_input_output,
                        versioned_cache,
                        &executor,
                        base_view,
                        ParallelState::new(
                            versioned_cache,
                            scheduler,
                            start_shared_counter,
                            shared_counter,
                        ),
                    )?;
                    scheduler.finish_execution(txn_idx, incarnation, updates_outside)
                },
                SchedulerTask::ExecutionTask(_, _, ExecutionTaskType::Wakeup(condvar)) => {
                    let (lock, cvar) = &*condvar;
                    // Mark dependency resolved.
                    let mut lock = lock.lock();
                    *lock = DependencyStatus::Resolved;
                    // Wake up the process waiting for dependency.
                    cvar.notify_one();

                    scheduler.next_task()
                },
                SchedulerTask::NoTask => scheduler.next_task(),
                SchedulerTask::Done => {
                    drain_commit_queue()?;
                    break Ok(());
                },
            }
        }
    }

    pub(crate) fn execute_transactions_parallel(
        &self,
        executor_initial_arguments: E::Argument,
        signature_verified_block: &[T],
        base_view: &S,
    ) -> Result<BlockOutput<E::Output>, E::Error> {
        let _timer = PARALLEL_EXECUTION_SECONDS.start_timer();
        // Using parallel execution with 1 thread currently will not work as it
        // will only have a coordinator role but no workers for rolling commit.
        // Need to special case no roles (commit hook by thread itself) to run
        // w. concurrency_level = 1 for some reason.
        assert!(
            self.config.local.concurrency_level > 1,
            "Must use sequential execution"
        );

        let versioned_cache = MVHashMap::new();
        let start_shared_counter = gen_id_start_value(false);
        let shared_counter = AtomicU32::new(start_shared_counter);

        if signature_verified_block.is_empty() {
            return Ok(BlockOutput::new(vec![]));
        }

        let num_txns = signature_verified_block.len();

        let shared_commit_state = ExplicitSyncWrapper::new((
            BlockGasLimitProcessor::new(self.config.onchain.block_gas_limit_type.clone(), num_txns),
            None,
        ));

        let final_results = ExplicitSyncWrapper::new(Vec::with_capacity(num_txns));

        {
            final_results
                .acquire()
                .resize_with(num_txns, E::Output::skip_output);
        }

        let num_txns = num_txns as u32;

        let last_input_output = TxnLastInputOutput::new(num_txns);
        let scheduler = Scheduler::new(num_txns);

        let timer = RAYON_EXECUTION_SECONDS.start_timer();
        self.executor_thread_pool.scope(|s| {
            for _ in 0..self.config.local.concurrency_level {
                s.spawn(|_| {
                    if let Err(e) = self.worker_loop(
                        &executor_initial_arguments,
                        signature_verified_block,
                        &last_input_output,
                        &versioned_cache,
                        &scheduler,
                        base_view,
                        start_shared_counter,
                        &shared_counter,
                        &shared_commit_state,
                        &final_results,
                    ) {
                        if scheduler.halt() {
                            let mut shared_commit_state_guard = shared_commit_state.acquire();
                            let (_, maybe_error) = shared_commit_state_guard.dereference_mut();
                            *maybe_error = Some(Error::FallbackToSequential(e));
                        }
                    }
                });
            }
        });
        drop(timer);
        // Explicit async drops.
        DEFAULT_DROPPER.schedule_drop((last_input_output, scheduler, versioned_cache));
        let (_block_limit_processor, maybe_error) = shared_commit_state.into_inner();

        // TODO add block end info to output.
        // block_limit_processor.is_block_limit_reached();

        match maybe_error {
            Some(err) => Err(err),
            None => Ok(BlockOutput::new(final_results.into_inner())),
        }
    }

    fn apply_output_sequential(
        unsync_map: &UnsyncMap<T::Key, T::Tag, T::Value, X, T::Identifier>,
        output: &E::Output,
    ) -> ::std::result::Result<(), PanicOr<IntentionalFallbackToSequential>> {
        for (key, (write_op, layout)) in output.resource_write_set().into_iter() {
            unsync_map.write(key, write_op, layout);
        }

        for (group_key, metadata_op, group_ops) in output.resource_group_write_set().into_iter() {
            for (value_tag, (group_op, maybe_layout)) in group_ops.into_iter() {
                unsync_map
                    .insert_group_op(&group_key, value_tag, group_op, maybe_layout)
                    .map_err(|e| {
                        resource_group_error(format!("Unexpected resource group error {:?}", e))
                    })?;
            }
            unsync_map.write(group_key, metadata_op, None);
        }

        for (key, write_op) in output.aggregator_v1_write_set().into_iter() {
            unsync_map.write(key, write_op, None);
        }

        for (key, write_op) in output.module_write_set().into_iter() {
            unsync_map.write_module(key, write_op);
        }

        let mut second_phase = Vec::new();
        let mut updates = HashMap::new();
        for (id, change) in output.delayed_field_change_set().into_iter() {
            match change {
                DelayedChange::Create(value) => {
                    assert_none!(
                        unsync_map.fetch_delayed_field(&id),
                        "Sequential execution must not create duplicate aggregators"
                    );
                    updates.insert(id, value);
                },
                DelayedChange::Apply(apply) => {
                    match apply.get_apply_base_id(&id) {
                        ApplyBase::Previous(base_id) => {
                            updates.insert(
                                id,
                                expect_ok(apply.apply_to_base(
                                    unsync_map.fetch_delayed_field(&base_id).unwrap(),
                                ))
                                .unwrap(),
                            );
                        },
                        ApplyBase::Current(base_id) => {
                            second_phase.push((id, base_id, apply));
                        },
                    };
                },
            }
        }
        for (id, base_id, apply) in second_phase.into_iter() {
            updates.insert(
                id,
                expect_ok(
                    apply.apply_to_base(
                        updates
                            .get(&base_id)
                            .cloned()
                            .unwrap_or_else(|| unsync_map.fetch_delayed_field(&base_id).unwrap()),
                    ),
                )
                .unwrap(),
            );
        }
        for (id, value) in updates.into_iter() {
            unsync_map.write_delayed_field(id, value);
        }

        Ok(())
    }

    // TODO[agg_v2][fix] Propagate code_invariant_error, to use second fallback.
    pub(crate) fn execute_transactions_sequential(
        &self,
        executor_arguments: E::Argument,
        signature_verified_block: &[T],
        base_view: &S,
        dynamic_change_set_optimizations_enabled: bool,
    ) -> Result<BlockOutput<E::Output>, E::Error> {
        let num_txns = signature_verified_block.len();
        let init_timer = VM_INIT_SECONDS.start_timer();
        let executor = E::init(executor_arguments);
        drop(init_timer);

        let start_counter = gen_id_start_value(true);
        let counter = RefCell::new(start_counter);
        let unsync_map = UnsyncMap::new();
        let mut ret = Vec::with_capacity(num_txns);
        let mut block_limit_processor = BlockGasLimitProcessor::<T>::new(
            self.config.onchain.block_gas_limit_type.clone(),
            num_txns,
        );

        for (idx, txn) in signature_verified_block.iter().enumerate() {
            let latest_view = LatestView::<T, S, X>::new(
                base_view,
                ViewState::Unsync(SequentialState::new(
                    &unsync_map,
                    start_counter,
                    &counter,
                    dynamic_change_set_optimizations_enabled,
                )),
                idx as TxnIndex,
            );
            let res = executor.execute_transaction(&latest_view, txn, idx as TxnIndex);

            let must_skip = matches!(res, ExecutionStatus::SkipRest(_));
            match res {
                ExecutionStatus::Success(output) | ExecutionStatus::SkipRest(output) => {
                    // Calculating the accumulated gas costs of the committed txns.
                    let fee_statement = output.fee_statement();

                    let approx_output_size = self
                        .config
                        .onchain
                        .block_gas_limit_type
                        .block_output_limit()
                        .map(|_| {
                            output.output_approx_size()
                                + if self
                                    .config
                                    .onchain
                                    .block_gas_limit_type
                                    .include_user_txn_size_in_block_output()
                                {
                                    txn.user_txn_bytes_len()
                                } else {
                                    0
                                } as u64
                        });

                    let read_write_summary = self
                        .config
                        .onchain
                        .block_gas_limit_type
                        .conflict_penalty_window()
                        .map(|_| {
                            ReadWriteSummary::new(
                                latest_view.take_sequential_reads().get_read_summary(),
                                output.get_write_summary(),
                            )
                        });
                    block_limit_processor.accumulate_fee_statement(
                        fee_statement,
                        read_write_summary,
                        approx_output_size,
                    );

                    output.materialize_agg_v1(&latest_view);
                    assert_eq!(
                        output.aggregator_v1_delta_set().len(),
                        0,
                        "Sequential execution must materialize deltas"
                    );

                    // Apply the writes.
                    // TODO[agg_v2](fix): return code invariant error if dynamic change set optimizations disabled.
                    Self::apply_output_sequential(&unsync_map, &output)?;

                    if dynamic_change_set_optimizations_enabled {
                        let group_metadata_ops = output.resource_group_metadata_ops();
                        let mut finalized_groups = Vec::with_capacity(group_metadata_ops.len());
                        for (group_key, group_metadata_op) in group_metadata_ops.into_iter() {
                            let finalized_group = unsync_map.finalize_group(&group_key);
                            if finalized_group.is_empty() != group_metadata_op.is_deletion() {
                                // TODO[agg_v2](fix): code invariant error if dynamic change set optimizations disabled.
                                // TODO[agg_v2](fix): make sure this cannot be triggered by an user transaction
                                return Err(resource_group_error(format!(
                                    "Group is empty = {} but op is deletion = {} in sequential execution",
                                    finalized_group.is_empty(),
                                    group_metadata_op.is_deletion()
                                )).into());
                            }
                            finalized_groups.push((group_key, group_metadata_op, finalized_group));
                        }

                        for (group_key, group_metadata_op) in
                            output.group_reads_needing_delayed_field_exchange()
                        {
                            let finalized_group = unsync_map.finalize_group(&group_key);
                            if finalized_group.is_empty() != group_metadata_op.is_deletion() {
                                return Err(resource_group_error(format!(
                                    "Group is empty = {} but op is deletion = {} in sequential execution",
                                    finalized_group.is_empty(),
                                    group_metadata_op.is_deletion()
                                )).into());
                            }
                            finalized_groups.push((group_key, group_metadata_op, finalized_group));
                        }

                        // Replace delayed field id with values in resource write set and read set.
                        let resource_change_set = Some(output.resource_write_set());
                        let mut patched_resource_write_set =
                            Self::map_id_to_values_in_write_set(resource_change_set, &latest_view);

                        for (key, layout) in
                            output.reads_needing_delayed_field_exchange().into_iter()
                        {
                            if let Some(ValueWithLayout::Exchanged(value, _)) =
                                unsync_map.fetch_data(&key)
                            {
                                if patched_resource_write_set
                                    .insert(
                                        key,
                                        Self::replace_ids_with_values(
                                            &value,
                                            layout.as_ref(),
                                            &latest_view,
                                        ),
                                    )
                                    .is_some()
                                {
                                    return Err(Error::FallbackToSequential(code_invariant_error(
                                        "reads_needing_delayed_field_exchange already in the write set for key",
                                    ).into()));
                                }
                            }
                        }

                        let patched_finalized_groups =
                            Self::map_id_to_values_in_group_writes(finalized_groups, &latest_view);

                        // Replace delayed field id with values in events
                        let patched_events = Self::map_id_to_values_events(
                            Box::new(output.get_events().into_iter()),
                            &latest_view,
                        );

                        let serialized_groups = Self::serialize_groups(patched_finalized_groups)
                            .map_err(Error::FallbackToSequential)?;

                        // TODO[agg_v2] patch resources in groups and provide explicitly
                        output.incorporate_materialized_txn_output(
                            // No aggregator v1 delta writes are needed for sequential execution.
                            // They are already handled because we passed materialize_deltas=true
                            // to execute_transaction.
                            vec![],
                            patched_resource_write_set
                                .into_iter()
                                .chain(serialized_groups.into_iter())
                                .collect(),
                            patched_events,
                        );
                    } else {
                        output.set_txn_output_for_non_dynamic_change_set();
                    }

                    if latest_view.is_incorrect_use() {
                        panic!("Incorrect use in sequential execution")
                    }

                    if let Some(commit_hook) = &self.transaction_commit_hook {
                        commit_hook.on_transaction_committed(idx as TxnIndex, &output);
                    }
                    ret.push(output);
                },
                ExecutionStatus::Abort(err) => {
                    if let Some(commit_hook) = &self.transaction_commit_hook {
                        commit_hook.on_execution_aborted(idx as TxnIndex);
                    }
                    // Record the status indicating abort.
                    return Err(Error::UserError(err));
                },
                ExecutionStatus::DirectWriteSetTransactionNotCapableError => {
                    panic!("PayloadWriteSet::Direct transaction not alone in a block, in sequential execution")
                },
                ExecutionStatus::SpeculativeExecutionAbortError(msg) => {
                    panic!(
                        "Sequential execution must not have SpeculativeExecutionAbortError: {:?}",
                        msg
                    );
                },
                ExecutionStatus::DelayedFieldsCodeInvariantError(msg) => {
                    error!(
                        "Sequential execution failed with DelayedFieldsCodeInvariantError: {:?}",
                        msg
                    );
                    return Err(Error::FallbackToSequential(PanicOr::CodeInvariantError(
                        msg,
                    )));
                },
            }
            // When the txn is a SkipRest txn, halt sequential execution.
            if must_skip {
                break;
            }

            if idx < num_txns - 1 && block_limit_processor.should_end_block_sequential() {
                break;
            }
        }

        block_limit_processor
            .finish_sequential_update_counters_and_log_info(ret.len() as u32, num_txns as u32);

        ret.resize_with(num_txns, E::Output::skip_output);

        // TODO add block end info to output.
        // block_limit_processor.is_block_limit_reached();

        Ok(BlockOutput::new(ret))
    }

    pub fn execute_block(
        &self,
        executor_arguments: E::Argument,
        signature_verified_block: &[T],
        base_view: &S,
    ) -> Result<BlockOutput<E::Output>, E::Error> {
        let dynamic_change_set_optimizations_enabled = signature_verified_block.len() != 1
            || E::is_transaction_dynamic_change_set_capable(&signature_verified_block[0]);

        let mut ret = if self.config.local.concurrency_level > 1
            && dynamic_change_set_optimizations_enabled
        {
            self.execute_transactions_parallel(
                executor_arguments,
                signature_verified_block,
                base_view,
            )
        } else {
            self.execute_transactions_sequential(
                executor_arguments,
                signature_verified_block,
                base_view,
                dynamic_change_set_optimizations_enabled,
            )
        };

        // Sequential execution fallback
        // Only worth doing if we did parallel before, i.e. if we did a different pass.
        if self.config.local.concurrency_level > 1 {
            if let Err(Error::FallbackToSequential(e)) = &ret {
                match e {
                    PanicOr::Or(IntentionalFallbackToSequential::ModulePathReadWrite) => {
                        debug!("[Execution]: Module read & written, sequential fallback");
                    },
                    PanicOr::Or(IntentionalFallbackToSequential::ResourceGroupError(msg)) => {
                        error!(
                            "[Execution]: ResourceGroupError({:?}), sequential fallback",
                            msg
                        );
                    },
                    PanicOr::CodeInvariantError(msg) => {
                        error!(
                            "[Execution]: CodeInvariantError({:?}), sequential fallback",
                            msg
                        );
                    },
                };

                // All logs from the parallel execution should be cleared and not reported.
                // Clear by re-initializing the speculative logs.
                init_speculative_logs(signature_verified_block.len());

                ret = self.execute_transactions_sequential(
                    executor_arguments,
                    signature_verified_block,
                    base_view,
                    dynamic_change_set_optimizations_enabled,
                );
            }
        }

        // If after trying available fallbacks, we still are askign to do a fallback,
        // something unrecoverable went wrong.
        if let Err(Error::FallbackToSequential(e)) = &ret {
            // TODO[agg_v2][fix] make sure this can never happen - we have sequential raising
            // this error often when something that should never happen goes wrong
            panic!("Sequential execution failed with {:?}", e);
        }

        ret
    }
}

fn resource_group_error(err_msg: String) -> PanicOr<IntentionalFallbackToSequential> {
    error!("resource_group_error: {:?}", err_msg);
    PanicOr::Or(IntentionalFallbackToSequential::ResourceGroupError(err_msg))
}

fn gen_id_start_value(sequential: bool) -> u32 {
    // IDs are ephemeral. Pick a random prefix, and different each time,
    // in case exchange is mistakenly not performed - to more easily catch it.
    // And in a bad case where it happens in prod, to and make sure incorrect
    // block doesn't get committed, but chain halts.
    // (take a different range from parallel execution, to even more easily differentiate)

    let offset = if sequential { 0 } else { 1000 };
    thread_rng().gen_range(1 + offset, 1000 + offset) * 1_000_000
}
