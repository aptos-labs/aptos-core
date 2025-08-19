// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    captured_reads::CapturedReads,
    code_cache_global::{add_module_write_to_module_cache, GlobalModuleCache},
    code_cache_global_manager::AptosModuleCacheManagerGuard,
    counters::{
        self, BLOCK_EXECUTOR_INNER_EXECUTE_BLOCK, PARALLEL_EXECUTION_SECONDS,
        PARALLEL_FINALIZE_SECONDS, RAYON_EXECUTION_SECONDS, TASK_EXECUTE_SECONDS,
        TASK_VALIDATE_SECONDS, VM_INIT_SECONDS, WORK_WITH_TASK_SECONDS,
    },
    errors::*,
    executor_utilities::*,
    explicit_sync_wrapper::ExplicitSyncWrapper,
    limit_processor::BlockGasLimitProcessor,
    scheduler::{DependencyStatus, ExecutionTaskType, Scheduler, SchedulerTask, Wave},
    scheduler_v2::{AbortManager, SchedulerV2, TaskKind},
    scheduler_wrapper::SchedulerWrapper,
    task::{
        AfterMaterializationOutput, BeforeMaterializationOutput, ExecutionStatus, ExecutorTask,
        TransactionOutput,
    },
    txn_commit_hook::TransactionCommitHook,
    txn_last_input_output::TxnLastInputOutput,
    txn_provider::TxnProvider,
    types::ReadWriteSummary,
    view::{LatestView, ParallelState, SequentialState, ViewState},
};
use aptos_aggregator::{
    delayed_change::{ApplyBase, DelayedChange},
    delta_change_set::serialize,
};
use aptos_crypto::HashValue;
use aptos_drop_helper::DEFAULT_DROPPER;
use aptos_logger::{error, info};
use aptos_mvhashmap::{
    types::{Incarnation, MVDelayedFieldsError, TxnIndex, ValueWithLayout},
    unsync_map::UnsyncMap,
    versioned_delayed_fields::CommitError,
    MVHashMap,
};
use aptos_types::{
    block_executor::{
        config::BlockExecutorConfig, transaction_slice_metadata::TransactionSliceMetadata,
    },
    error::{code_invariant_error, expect_ok, PanicError, PanicOr},
    on_chain_config::Features,
    state_store::{state_value::StateValue, TStateView},
    transaction::{
        block_epilogue::TBlockEndInfoExt, AuxiliaryInfoTrait, BlockExecutableTransaction,
        BlockOutput, FeeDistribution,
    },
    vm::modules::AptosModuleExtension,
    write_set::{TransactionWrite, WriteOp},
};
use aptos_vm_environment::environment::AptosEnvironment;
use aptos_vm_logging::{alert, init_speculative_logs, prelude::*};
use aptos_vm_types::{change_set::randomly_check_layout_matches, resolver::ResourceGroupSize};
use bytes::Bytes;
use claims::assert_none;
use core::panic;
use fail::fail_point;
use move_binary_format::CompiledModule;
use move_core_types::{language_storage::ModuleId, value::MoveTypeLayout, vm_status::StatusCode};
use move_vm_runtime::{Module, RuntimeEnvironment, WithRuntimeEnvironment};
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
use num_cpus;
use rayon::ThreadPool;
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    marker::{PhantomData, Sync},
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc,
    },
};

struct SharedSyncParams<'a, 'b, T, E, S>
where
    T: BlockExecutableTransaction,
    E: ExecutorTask<Txn = T>,
    S: TStateView<Key = T::Key> + Sync,
{
    // TODO: should not need to pass base view.
    base_view: &'a S,
    versioned_cache: &'a MVHashMap<T::Key, T::Tag, T::Value, DelayedFieldID>,
    global_module_cache:
        &'a GlobalModuleCache<ModuleId, CompiledModule, Module, AptosModuleExtension>,
    last_input_output: &'a TxnLastInputOutput<T, E::Output, E::Error>,
    start_shared_counter: u32,
    delayed_field_id_counter: &'a AtomicU32,
    block_limit_processor: &'a ExplicitSyncWrapper<BlockGasLimitProcessor<'b, T, S>>,
    final_results: &'a ExplicitSyncWrapper<Vec<E::Output>>,
    maybe_block_epilogue_txn_idx: &'a ExplicitSyncWrapper<Option<TxnIndex>>,
}

pub struct BlockExecutor<T, E, S, L, TP, A> {
    // Number of active concurrent tasks, corresponding to the maximum number of rayon
    // threads that may be concurrently participating in parallel execution.
    config: BlockExecutorConfig,
    executor_thread_pool: Arc<rayon::ThreadPool>,
    transaction_commit_hook: Option<L>,
    phantom: PhantomData<fn() -> (T, E, S, L, TP, A)>,
}

impl<T, E, S, L, TP, A> BlockExecutor<T, E, S, L, TP, A>
where
    T: BlockExecutableTransaction,
    E: ExecutorTask<Txn = T, AuxiliaryInfo = A>,
    S: TStateView<Key = T::Key> + Sync,
    L: TransactionCommitHook<Output = E::Output>,
    TP: TxnProvider<T, A> + Sync,
    A: AuxiliaryInfoTrait,
{
    /// The caller needs to ensure that concurrency_level > 1 (0 is illegal and 1 should
    /// be handled by sequential execution) and that concurrency_level <= num_cpus.
    pub fn new(
        config: BlockExecutorConfig,
        executor_thread_pool: Arc<ThreadPool>,
        transaction_commit_hook: Option<L>,
    ) -> Self {
        let num_cpus = num_cpus::get();
        assert!(
            config.local.concurrency_level > 0 && config.local.concurrency_level <= num_cpus,
            "Parallel execution concurrency level {} should be between 1 and number of CPUs ({})",
            config.local.concurrency_level,
            num_cpus,
        );
        Self {
            config,
            executor_thread_pool,
            transaction_commit_hook,
            phantom: PhantomData,
        }
    }

    // The bool in the result indicates whether execution result is a speculative abort.
    fn process_execution_result<'a>(
        execution_result: &'a ExecutionStatus<E::Output, E::Error>,
        read_set: &mut CapturedReads<T, ModuleId, CompiledModule, Module, AptosModuleExtension>,
        txn_idx: TxnIndex,
    ) -> Result<(Option<&'a E::Output>, bool), PanicError> {
        match execution_result {
            ExecutionStatus::Success(output) | ExecutionStatus::SkipRest(output) => {
                Ok((Some(output), false))
            },
            ExecutionStatus::SpeculativeExecutionAbortError(_msg) => {
                // TODO(BlockSTMv2): cleaner to rename or distinguish V2 early abort
                // from DeltaApplicationFailure. This is also why we return the bool
                // separately for now instead of relying on the read set.
                read_set.capture_delayed_field_read_error(&PanicOr::Or(
                    MVDelayedFieldsError::DeltaApplicationFailure,
                ));
                Ok((None, true))
            },
            ExecutionStatus::Abort(_err) => Ok((None, false)),
            ExecutionStatus::DelayedFieldsCodeInvariantError(msg) => {
                Err(code_invariant_error(format!(
                    "[Execution] At txn {}, failed with DelayedFieldsCodeInvariantError: {:?}",
                    txn_idx, msg
                )))
            },
        }
    }

    // V1 processing is embedded in the execute method, while execute_v2 method calls
    // this method to process speculative resource group outputs.
    fn process_resource_group_output_v2(
        maybe_output: Option<&E::Output>,
        idx_to_execute: TxnIndex,
        incarnation: Incarnation,
        last_input_output: &TxnLastInputOutput<T, E::Output, E::Error>,
        versioned_cache: &MVHashMap<T::Key, T::Tag, T::Value, DelayedFieldID>,
        abort_manager: &mut AbortManager,
    ) -> Result<(), PanicError> {
        // The order of applying new group writes versus clearing previous writes is reversed
        // in BlockSTMv2 as opposed to V1, which avoids the necessity to clone group keys and
        // previous tags.
        // TODO(BlockSTMv2): consider similar flow for resources.

        let mut resource_group_write_set = maybe_output.map_or(Ok(HashMap::new()), |output| {
            output
                .before_materialization()
                .map(|inner| inner.resource_group_write_set())
        })?;

        last_input_output.for_each_resource_group_key_and_tags(
            idx_to_execute,
            |group_key_ref, prev_tags| {
                match resource_group_write_set.remove_entry(group_key_ref) {
                    Some((group_key, (group_metadata_op, group_size, group_ops))) => {
                        // Current incarnation overwrites the previous write to a group.
                        // TODO(BlockSTMv2): After MVHashMap refactoring, expose a single API
                        // for groups handling everything (inner resources, metadata & size).
                        abort_manager.invalidate_dependencies(
                            // Invalidate the readers of group metadata.
                            versioned_cache.data().write_v2::<true>(
                                group_key.clone(),
                                idx_to_execute,
                                incarnation,
                                Arc::new(group_metadata_op),
                                None,
                            )?,
                        )?;
                        abort_manager.invalidate_dependencies(
                            versioned_cache.group_data().write_v2(
                                group_key,
                                idx_to_execute,
                                incarnation,
                                group_ops.into_iter(),
                                group_size,
                                prev_tags,
                            )?,
                        )?;
                    },
                    None => {
                        // Clean up the write from previous incarnation.
                        abort_manager.invalidate_dependencies(
                            // Invalidate the readers of group metadata.
                            versioned_cache
                                .data()
                                .remove_v2::<_, true>(group_key_ref, idx_to_execute)?,
                        )?;
                        abort_manager.invalidate_dependencies(
                            versioned_cache.group_data().remove_v2(
                                group_key_ref,
                                idx_to_execute,
                                prev_tags,
                            )?,
                        )?;
                    },
                }
                Ok(())
            },
        )?;

        // Handle any remaining entries in resource_group_write_set (new group writes)
        for (group_key, (group_metadata_op, group_size, group_ops)) in resource_group_write_set {
            // New group write that wasn't in previous incarnation
            abort_manager.invalidate_dependencies(
                // Invalidate the readers of group metadata.
                versioned_cache.data().write_v2::<true>(
                    group_key.clone(),
                    idx_to_execute,
                    incarnation,
                    Arc::new(group_metadata_op),
                    None,
                )?,
            )?;
            abort_manager.invalidate_dependencies(versioned_cache.group_data().write_v2(
                group_key,
                idx_to_execute,
                incarnation,
                group_ops.into_iter(),
                group_size,
                HashSet::new(), // No previous tags since this is a new group write
            )?)?;
        }

        Ok(())
    }

    fn process_delayed_field_output(
        maybe_output: Option<&E::Output>,
        idx_to_execute: TxnIndex,
        read_set: &mut CapturedReads<T, ModuleId, CompiledModule, Module, AptosModuleExtension>,
        last_input_output: &TxnLastInputOutput<T, E::Output, E::Error>,
        versioned_cache: &MVHashMap<T::Key, T::Tag, T::Value, DelayedFieldID>,
        is_v2: bool,
    ) -> Result<(), PanicError> {
        let mut prev_modified_delayed_fields = last_input_output
            .delayed_field_keys(idx_to_execute)
            .map_or_else(HashSet::new, |keys| keys.collect());

        // TODO[agg_v2](optimize): see if/how we want to incorporate DeltaHistory from read set into
        // versioned_delayed_fields. Without it, currently, materialized reads cannot check history
        // and fail early.
        //
        // We can extract histories with something like the code below, and then include history in
        // change.into_entry_no_additional_history().
        //
        // for id in read_set.get_delayed_field_keys() {
        //     if !delayed_field_change_set.contains_key(id) {
        //         let read_value = read_set.get_delayed_field_by_kind(id, DelayedFieldReadKind::Bounded).unwrap();
        //     }
        // }

        if let Some(output) = maybe_output {
            let output_before_guard = output.before_materialization()?;
            for (id, change) in output_before_guard.delayed_field_change_set().into_iter() {
                prev_modified_delayed_fields.remove(&id);

                let entry = change.into_entry_no_additional_history();

                // TODO[agg_v2](optimize): figure out if it is useful for change to update needs_suffix_validation
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
        }

        for id in prev_modified_delayed_fields {
            versioned_cache
                .delayed_fields()
                .remove(&id, idx_to_execute, is_v2)?;
        }

        Ok(())
    }

    fn execute_v2(
        worker_id: u32,
        idx_to_execute: TxnIndex,
        incarnation: Incarnation,
        txn: &T,
        auxiliary_info: &A,
        last_input_output: &TxnLastInputOutput<T, E::Output, E::Error>,
        versioned_cache: &MVHashMap<T::Key, T::Tag, T::Value, DelayedFieldID>,
        executor: &E,
        base_view: &S,
        global_module_cache: &GlobalModuleCache<
            ModuleId,
            CompiledModule,
            Module,
            AptosModuleExtension,
        >,
        runtime_environment: &RuntimeEnvironment,
        parallel_state: ParallelState<T>,
        scheduler: &SchedulerV2,
    ) -> Result<(), PanicError> {
        let _timer = TASK_EXECUTE_SECONDS.start_timer();

        let mut abort_manager = AbortManager::new(idx_to_execute, incarnation, scheduler);
        let sync_view = LatestView::new(
            base_view,
            global_module_cache,
            runtime_environment,
            ViewState::Sync(parallel_state),
            idx_to_execute,
        );
        let execution_result =
            executor.execute_transaction(&sync_view, txn, auxiliary_info, idx_to_execute);

        let mut read_set = sync_view.take_parallel_reads();
        if read_set.is_incorrect_use() {
            return Err(code_invariant_error(format!(
                "Incorrect use detected in CapturedReads after executing txn = {idx_to_execute} incarnation = {incarnation}"
            )));
        }

        let (maybe_output, is_speculative_failure) =
            Self::process_execution_result(&execution_result, &mut read_set, idx_to_execute)?;

        if is_speculative_failure {
            // Recording in order to check the invariant that the final, committed incarnation
            // of each transaction is not a speculative failure.
            last_input_output.record_speculative_failure(idx_to_execute);
            // Ignoring module validation requirements since speculative failure
            // anyway requires re-execution.
            let _ = scheduler.finish_execution(abort_manager)?;
            return Ok(());
        }

        Self::process_delayed_field_output(
            maybe_output,
            idx_to_execute,
            &mut read_set,
            last_input_output,
            versioned_cache,
            true,
        )?;
        Self::process_resource_group_output_v2(
            maybe_output,
            idx_to_execute,
            incarnation,
            last_input_output,
            versioned_cache,
            &mut abort_manager,
        )?;

        let mut prev_modified_resource_keys = last_input_output
            .modified_resource_keys_no_aggregator_v1(idx_to_execute)
            .map_or_else(HashSet::new, |keys| keys.collect());
        let mut prev_modified_aggregator_v1_keys = last_input_output
            .modified_aggregator_v1_keys(idx_to_execute)
            .map_or_else(HashSet::new, |keys| keys.collect());

        // TODO: BlockSTMv2: use estimates for delayed field reads? (see V1 update on abort).
        if let Some(output) = maybe_output {
            let output_before_guard = output.before_materialization()?;

            for (key, value, maybe_layout) in output_before_guard.resource_write_set().into_iter() {
                prev_modified_resource_keys.remove(&key);
                abort_manager.invalidate_dependencies(
                    versioned_cache.data().write_v2::<false>(
                        key,
                        idx_to_execute,
                        incarnation,
                        value,
                        maybe_layout,
                    )?,
                )?;
            }

            // Apply aggregator v1 writes and deltas, using versioned data's V1 (write/add_delta) APIs.
            // AggregatorV1 is not push-validated, but follows the same logic as delayed fields, i.e.
            // commit-time validation in BlockSTMv2.
            for (key, value) in output_before_guard.aggregator_v1_write_set().into_iter() {
                prev_modified_aggregator_v1_keys.remove(&key);

                versioned_cache.data().write(
                    key,
                    idx_to_execute,
                    incarnation,
                    Arc::new(value),
                    None,
                );
            }
            for (key, delta) in output_before_guard.aggregator_v1_delta_set().into_iter() {
                prev_modified_aggregator_v1_keys.remove(&key);
                versioned_cache.data().add_delta(key, idx_to_execute, delta);
            }
        }

        // Remove entries from previous write/delta set that were not overwritten.
        for key in prev_modified_resource_keys {
            abort_manager.invalidate_dependencies(
                versioned_cache
                    .data()
                    .remove_v2::<_, false>(&key, idx_to_execute)?,
            )?;
        }

        for key in prev_modified_aggregator_v1_keys {
            versioned_cache.data().remove(&key, idx_to_execute);
        }

        last_input_output.record(idx_to_execute, read_set, execution_result);

        // It is important to call finish_execution after recording the input/output.
        // CAUTION: once any update has been applied to the shared data structures, there should
        // be no short circuits until the record succeeds and scheduler is notified that the
        // execution is finished. This allows cleaning up the shared data structures before
        // applying the updates from next incarnation (which can also be the block epilogue txn).
        if let Some(module_validation_requirements) = scheduler.finish_execution(abort_manager)? {
            Self::module_validation_v2(
                idx_to_execute,
                incarnation,
                scheduler,
                &module_validation_requirements,
                last_input_output,
                global_module_cache,
                versioned_cache,
            )?;
            scheduler.finish_cold_validation_requirement(
                worker_id,
                idx_to_execute,
                incarnation,
                true,
            )?;
        }
        Ok(())
    }

    fn execute(
        idx_to_execute: TxnIndex,
        incarnation: Incarnation,
        txn: &T,
        auxiliary_info: &A,
        // Passed for BlockSTMv1 during speculative execution, and used to record when the
        // transaction starts processing the outputs, as well as when the execution is finished.
        maybe_scheduler: Option<&Scheduler>,
        last_input_output: &TxnLastInputOutput<T, E::Output, E::Error>,
        versioned_cache: &MVHashMap<T::Key, T::Tag, T::Value, DelayedFieldID>,
        executor: &E,
        base_view: &S,
        global_module_cache: &GlobalModuleCache<
            ModuleId,
            CompiledModule,
            Module,
            AptosModuleExtension,
        >,
        runtime_environment: &RuntimeEnvironment,
        parallel_state: ParallelState<T>,
    ) -> Result<SchedulerTask, PanicError> {
        let _timer = TASK_EXECUTE_SECONDS.start_timer();

        // VM execution.
        let sync_view = LatestView::new(
            base_view,
            global_module_cache,
            runtime_environment,
            ViewState::Sync(parallel_state),
            idx_to_execute,
        );
        let execution_result =
            executor.execute_transaction(&sync_view, txn, auxiliary_info, idx_to_execute);

        let mut read_set = sync_view.take_parallel_reads();
        if read_set.is_incorrect_use() {
            return Err(code_invariant_error(format!(
                "Incorrect use detected in CapturedReads after executing txn = {} incarnation = {}",
                idx_to_execute, incarnation
            )));
        }
        let (processed_output, _) =
            Self::process_execution_result(&execution_result, &mut read_set, idx_to_execute)?;

        let mut prev_modified_resource_keys = last_input_output
            .modified_resource_keys(idx_to_execute)
            .map_or_else(HashSet::new, |keys| keys.map(|(k, _)| k).collect());
        let mut prev_modified_group_keys: HashMap<T::Key, HashSet<T::Tag>> = last_input_output
            .modified_group_key_and_tags_cloned(idx_to_execute)
            .into_iter()
            .collect();

        // CAUTION: start shared output critical section.
        // If control flow reaches below and changes are applied to the shared data structures,
        // it should be guaranteed that the process will complete fully, completed by
        // recording of the input/outputs and lastly, by finish_execution. Hence, in the below
        // "critical section", e.g. returning with Ok status after observing the scheduler has halted
        // would be incorrect and lead to a PanicError if the block prologue txn were to be
        // executed later at the same index (after block cutting).
        // TODO(BlockSTMv2): Replace with a compile-time check if possible, or custom clippy lint.
        Self::process_delayed_field_output(
            processed_output,
            idx_to_execute,
            &mut read_set,
            last_input_output,
            versioned_cache,
            false,
        )?;

        // For tracking whether it's required to (re-)validate the suffix of transactions in the block.
        // May happen, for instance, when the recent execution wrote outside of the previous write/delta
        // set (vanilla Block-STM rule), or if resource group size or metadata changed from an estimate
        // (since those resource group validations rely on estimates).
        let mut needs_suffix_validation = false;
        let mut apply_updates = |output: &E::Output| -> Result<
            Vec<(T::Key, Arc<T::Value>, Option<Arc<MoveTypeLayout>>)>, // Cached resource writes
            PanicError,
        > {
            let output_before_guard = output.before_materialization()?;
            for (group_key, (group_metadata_op, group_size, group_ops)) in
                output_before_guard.resource_group_write_set().into_iter()
            {
                let prev_tags = prev_modified_group_keys
                    .remove(&group_key)
                    .unwrap_or_else(|| {
                        // Previously no write to the group at all.
                        needs_suffix_validation = true;
                        HashSet::new()
                    });

                if versioned_cache.data().write_metadata(
                    group_key.clone(),
                    idx_to_execute,
                    incarnation,
                    group_metadata_op,
                ) {
                    needs_suffix_validation = true;
                }

                if versioned_cache.group_data().write(
                    group_key,
                    idx_to_execute,
                    incarnation,
                    group_ops.into_iter(),
                    group_size,
                    prev_tags,
                )? {
                    needs_suffix_validation = true;
                }
            }

            let resource_write_set = output_before_guard.resource_write_set();

            // Then, process resource & aggregator_v1 writes.
            for (k, v, maybe_layout) in resource_write_set.clone().into_iter().chain(
                output_before_guard
                    .aggregator_v1_write_set()
                    .into_iter()
                    .map(|(state_key, write_op)| (state_key, Arc::new(write_op), None)),
            ) {
                if !prev_modified_resource_keys.remove(&k) {
                    needs_suffix_validation = true;
                }
                versioned_cache
                    .data()
                    .write(k, idx_to_execute, incarnation, v, maybe_layout);
            }

            // Then, apply deltas.
            for (k, d) in output_before_guard.aggregator_v1_delta_set().into_iter() {
                if !prev_modified_resource_keys.remove(&k) {
                    needs_suffix_validation = true;
                }
                versioned_cache.data().add_delta(k, idx_to_execute, d);
            }

            Ok(resource_write_set)
        };

        if let Some(output) = processed_output {
            apply_updates(output)?;
        }

        // Remove entries from previous write/delta set that were not overwritten.
        for k in prev_modified_resource_keys {
            versioned_cache.data().remove(&k, idx_to_execute);
        }
        for (k, tags) in prev_modified_group_keys {
            // A change in state observable during speculative execution
            // (which includes group metadata and size) changes, suffix
            // re-validation is needed. For resources where speculative
            // execution waits on estimates, having a write that was there
            // but not anymore does not qualify, as it can only cause
            // additional waiting but not an incorrect speculation result.
            // However, a group size or metadata might be read, and then
            // speculative group update might be removed below. Without
            // triggering suffix re-validation, a later transaction might
            // end up with the incorrect read result (corresponding to the
            // removed group information from an incorrect speculative state).
            needs_suffix_validation = true;

            versioned_cache.data().remove(&k, idx_to_execute);
            versioned_cache
                .group_data()
                .remove(&k, idx_to_execute, tags);
        }

        last_input_output.record(idx_to_execute, read_set, execution_result);
        if let Some(scheduler) = maybe_scheduler {
            scheduler.finish_execution(idx_to_execute, incarnation, needs_suffix_validation)
        } else {
            // Final re-execution of the txn does not require scheduler,
            // or need to return a task.
            Ok(SchedulerTask::Retry)
        }
    }

    fn module_validation_v2(
        idx_to_validate: TxnIndex,
        incarnation_to_validate: Incarnation,
        scheduler: &SchedulerV2,
        updated_module_keys: &BTreeSet<ModuleId>,
        last_input_output: &TxnLastInputOutput<T, E::Output, E::Error>,
        global_module_cache: &GlobalModuleCache<
            ModuleId,
            CompiledModule,
            Module,
            AptosModuleExtension,
        >,
        versioned_cache: &MVHashMap<T::Key, T::Tag, T::Value, DelayedFieldID>,
    ) -> Result<bool, PanicError> {
        // The previous read-set must be recorded because:
        // 1. The transaction has finished at least one execution in order for it
        // to be eligible for module validation (status must have been executed).
        // 2. The only possible time to take the read-set from txn_last_input_output
        // is in prepare_and_queue_commit_ready_txn (applying module publishing output).
        // However, required module validation necessarily occurs before the commit.
        let (read_set, is_speculative_failure) =
            last_input_output.read_set(idx_to_validate).ok_or_else(|| {
                code_invariant_error(format!(
                    "Prior read-set of txn {} incarnation {} not recorded for module verification",
                    idx_to_validate, incarnation_to_validate
                ))
            })?;
        // Perform invariant checks or return early based on read set's incarnation.
        let blockstm_v2_incarnation = read_set.blockstm_v2_incarnation().ok_or_else(|| {
            code_invariant_error(
                "BlockSTMv2 must be enabled in CapturedReads when validating module reads",
            )
        })?;
        if blockstm_v2_incarnation > incarnation_to_validate || is_speculative_failure {
            // No need to validate as a newer incarnation has already been executed
            // and recorded its output, or the incarnation has resulted in a speculative
            // failure, which means there will be a further re-execution.
            return Ok(true);
        }
        if blockstm_v2_incarnation < incarnation_to_validate {
            return Err(code_invariant_error(format!(
                "For txn_idx {}, read set incarnation {} < incarnation to validate {}",
                idx_to_validate, blockstm_v2_incarnation, incarnation_to_validate
            )));
        }

        if !read_set.validate_module_reads(
            global_module_cache,
            versioned_cache.module_cache(),
            Some(updated_module_keys),
        ) {
            scheduler.direct_abort(idx_to_validate, incarnation_to_validate, false)?;
            return Ok(false);
        }

        Ok(true)
    }

    fn validate(
        idx_to_validate: TxnIndex,
        last_input_output: &TxnLastInputOutput<T, E::Output, E::Error>,
        global_module_cache: &GlobalModuleCache<
            ModuleId,
            CompiledModule,
            Module,
            AptosModuleExtension,
        >,
        versioned_cache: &MVHashMap<T::Key, T::Tag, T::Value, DelayedFieldID>,
        skip_module_reads_validation: bool,
    ) -> bool {
        let _timer = TASK_VALIDATE_SECONDS.start_timer();
        let (read_set, is_speculative_failure) = last_input_output
            .read_set(idx_to_validate)
            .expect("[BlockSTM]: Prior read-set must be recorded");

        if is_speculative_failure {
            return false;
        }

        assert!(
            !read_set.is_incorrect_use(),
            "Incorrect use must be handled after execution"
        );

        // Note: we validate delayed field reads only at try_commit.
        // TODO[agg_v2](optimize): potentially add some basic validation.
        // TODO[agg_v2](optimize): potentially add more sophisticated validation, but if it fails,
        // we mark it as a soft failure, requires some new statuses in the scheduler
        // (i.e. not re-execute unless some other part of the validation fails or
        // until commit, but mark as estimates).

        read_set.validate_data_reads(versioned_cache.data(), idx_to_validate)
            && read_set.validate_group_reads(versioned_cache.group_data(), idx_to_validate)
            && (skip_module_reads_validation
                || read_set.validate_module_reads(
                    global_module_cache,
                    versioned_cache.module_cache(),
                    None,
                ))
    }

    fn update_on_validation(
        txn_idx: TxnIndex,
        incarnation: Incarnation,
        valid: bool,
        validation_wave: Wave,
        last_input_output: &TxnLastInputOutput<T, E::Output, E::Error>,
        versioned_cache: &MVHashMap<T::Key, T::Tag, T::Value, DelayedFieldID>,
        scheduler: &Scheduler,
    ) -> Result<SchedulerTask, PanicError> {
        let aborted = !valid && scheduler.try_abort(txn_idx, incarnation);

        if aborted {
            update_transaction_on_abort::<T, E>(txn_idx, last_input_output, versioned_cache);
            scheduler.finish_abort(txn_idx, incarnation)
        } else {
            scheduler.finish_validation(txn_idx, validation_wave);

            if valid {
                scheduler.queueing_commits_arm();
            }

            Ok(SchedulerTask::Retry)
        }
    }

    /// Validates delayed fields read-set. If validation is successful, commits delayed field
    /// changes to multi-version data structure and returns true. If validation or commit fails,
    /// returns false (indicating that transaction needs to be re-executed).
    fn validate_and_commit_delayed_fields(
        txn_idx: TxnIndex,
        versioned_cache: &MVHashMap<T::Key, T::Tag, T::Value, DelayedFieldID>,
        last_input_output: &TxnLastInputOutput<T, E::Output, E::Error>,
        is_v2: bool,
    ) -> Result<bool, PanicError> {
        let (read_set, is_speculative_failure) = last_input_output
            .read_set(txn_idx)
            .ok_or_else(|| code_invariant_error("Read set must be recorded"))?;

        if is_speculative_failure {
            return Ok(false);
        }

        if !read_set.validate_delayed_field_reads(versioned_cache.delayed_fields(), txn_idx)?
            || (is_v2
                && !read_set.validate_aggregator_v1_reads(
                    versioned_cache.data(),
                    last_input_output
                        .modified_aggregator_v1_keys(txn_idx)
                        .ok_or_else(|| {
                            code_invariant_error("Modified aggregator v1 keys must be recorded")
                        })?,
                    txn_idx,
                )?)
        {
            return Ok(false);
        }

        let delayed_field_ids = last_input_output
            .delayed_field_keys(txn_idx)
            .ok_or_else(|| code_invariant_error("Delayed field keys must be recorded"))?;
        if let Err(e) = versioned_cache
            .delayed_fields()
            .try_commit(txn_idx, delayed_field_ids)
        {
            return match e {
                CommitError::ReExecutionNeeded(_) => Ok(false),
                CommitError::CodeInvariantError(msg) => Err(code_invariant_error(msg)),
            };
        }

        Ok(true)
    }

    // A transaction may have to be re-executed here outside of the regular worker loop
    // flow. For now, the two possible callers are prepare_and_queue_commit_ready_txn
    // and finalize_parallel_execution (for block epilogue txn). In both cases, all
    // txns below the index are committed, and the contents of the multi-versioned data
    // structure must reflect the corresponding final (committed) outputs.
    fn execute_txn_after_commit(
        txn: &T,
        auxiliary_info: &A,
        txn_idx: TxnIndex,
        incarnation: Incarnation,
        scheduler: SchedulerWrapper,
        versioned_cache: &MVHashMap<T::Key, T::Tag, T::Value, DelayedFieldID>,
        last_input_output: &TxnLastInputOutput<T, E::Output, E::Error>,
        start_shared_counter: u32,
        shared_counter: &AtomicU32,
        executor: &E,
        base_view: &S,
        global_module_cache: &GlobalModuleCache<
            ModuleId,
            CompiledModule,
            Module,
            AptosModuleExtension,
        >,
        runtime_environment: &RuntimeEnvironment,
    ) -> Result<(), PanicError> {
        let parallel_state = ParallelState::new(
            versioned_cache,
            scheduler,
            start_shared_counter,
            shared_counter,
            incarnation,
        );

        match scheduler.as_v2() {
            None => {
                // We are ignoring _needs_suffix_validation, as the caller will reduce the
                // validation index unconditionally after execute_txn_after_commit call.
                Self::execute(
                    txn_idx,
                    incarnation,
                    txn,
                    auxiliary_info,
                    None,
                    last_input_output,
                    versioned_cache,
                    executor,
                    base_view,
                    global_module_cache,
                    runtime_environment,
                    parallel_state,
                )?;
            },
            Some((scheduler, worker_id)) => {
                Self::execute_v2(
                    worker_id,
                    txn_idx,
                    incarnation,
                    txn,
                    auxiliary_info,
                    last_input_output,
                    versioned_cache,
                    executor,
                    base_view,
                    global_module_cache,
                    runtime_environment,
                    parallel_state,
                    scheduler,
                )?;
            },
        }

        if !Self::validate_and_commit_delayed_fields(
            txn_idx,
            versioned_cache,
            last_input_output,
            scheduler.is_v2(),
        )? {
            return Err(code_invariant_error(format!(
                "Delayed field validation after re-execution failed for txn {}",
                txn_idx
            )));
        }

        Ok(())
    }

    /// This method may be executed by different threads / workers, but is guaranteed to be executed
    /// non-concurrently by the scheduling in parallel executor. This allows to perform light logic
    /// related to committing a transaction in a simple way and without excessive synchronization
    /// overhead. On the other hand, materialization that happens after commit (& after this method)
    /// is concurrent and deals with logic such as patching delayed fields / resource groups
    /// in outputs, which is heavier (due to serialization / deserialization, copies, etc). Moreover,
    /// since prepare_and_queue_commit_ready_txns takes care of synchronization in the flat-combining
    /// way, the materialization can be almost embarrassingly parallelizable.
    ///
    /// Returns true if the block is fully committed, and the block epilogue txn should be created.
    fn prepare_and_queue_commit_ready_txn(
        &self,
        txn_idx: TxnIndex,
        incarnation: Incarnation,
        num_txns: TxnIndex,
        executor: &E,
        block: &TP,
        num_workers: usize,
        runtime_environment: &RuntimeEnvironment,
        scheduler: SchedulerWrapper,
        shared_sync_params: &SharedSyncParams<T, E, S>,
    ) -> Result<bool, PanicOr<ParallelBlockExecutionError>> {
        let versioned_cache = shared_sync_params.versioned_cache;
        let last_input_output = shared_sync_params.last_input_output;
        let global_module_cache = shared_sync_params.global_module_cache;

        let block_limit_processor = &mut shared_sync_params.block_limit_processor.acquire();
        let mut side_effect_at_commit = false;

        if !Self::validate_and_commit_delayed_fields(
            txn_idx,
            versioned_cache,
            last_input_output,
            scheduler.is_v2(),
        )? {
            // Transaction needs to be re-executed, one final time.
            side_effect_at_commit = true;

            scheduler.abort_pre_final_reexecution::<T, E>(
                txn_idx,
                incarnation,
                last_input_output,
                versioned_cache,
            )?;

            Self::execute_txn_after_commit(
                block.get_txn(txn_idx),
                &block.get_auxiliary_info(txn_idx),
                txn_idx,
                incarnation + 1,
                scheduler,
                versioned_cache,
                last_input_output,
                shared_sync_params.start_shared_counter,
                shared_sync_params.delayed_field_id_counter,
                executor,
                shared_sync_params.base_view,
                global_module_cache,
                runtime_environment,
            )?;
        }

        // Publish modules before we decrease validation index (in V1) so that validations observe
        // the new module writes as well.
        if last_input_output.publish_module_write_set(
            txn_idx,
            global_module_cache,
            versioned_cache,
            runtime_environment,
            &scheduler,
        )? {
            side_effect_at_commit = true;
        }

        if side_effect_at_commit {
            scheduler.wake_dependencies_and_decrease_validation_idx(txn_idx)?;
        }

        last_input_output.commit(
            txn_idx,
            num_txns,
            num_workers,
            block.get_txn(txn_idx).user_txn_bytes_len() as u64,
            &self.config.onchain.block_gas_limit_type,
            block_limit_processor,
            &scheduler,
        )
    }

    fn materialize_aggregator_v1_delta_writes(
        txn_idx: TxnIndex,
        last_input_output: &TxnLastInputOutput<T, E::Output, E::Error>,
        versioned_cache: &MVHashMap<T::Key, T::Tag, T::Value, DelayedFieldID>,
        base_view: &S,
    ) -> Vec<(T::Key, WriteOp)> {
        // Materialize all the aggregator v1 deltas.
        let mut aggregator_v1_delta_writes = Vec::with_capacity(4);
        if let Some(aggregator_v1_delta_keys_iter) =
            last_input_output.aggregator_v1_delta_keys(txn_idx)
        {
            for k in aggregator_v1_delta_keys_iter {
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

                        versioned_cache.data().set_base_value(
                            k.clone(),
                            ValueWithLayout::RawFromStorage(Arc::new(w)),
                        );
                        op.apply_to(value_u128)
                            .expect("Materializing delta w. base value set must succeed")
                    });

                // Must contain committed value as we set the base value above.
                aggregator_v1_delta_writes.push((
                    k,
                    WriteOp::legacy_modification(serialize(&committed_delta).into()),
                ));
            }
        }
        aggregator_v1_delta_writes
    }

    // If output_idx is set, then the finalized output is recorded at that index,
    // which might be different from txn_idx. This is used for block epilogue txn,
    // because the block may be cut, necessitating the block epilogue txn to be
    // virtually executed at a different index (right after the block cut point).
    // In this case, the data is stored at txn_idx, but finalized output will
    // still appear at the end of the block.
    fn materialize_txn_commit(
        &self,
        txn_idx: TxnIndex,
        scheduler: SchedulerWrapper,
        runtime_environment: &RuntimeEnvironment,
        shared_sync_params: &SharedSyncParams<T, E, S>,
    ) -> Result<(), PanicError> {
        let last_input_output = shared_sync_params.last_input_output;

        // Do a final validation for safety as a part of (parallel) post-processing.
        // Delayed fields are already validated in the sequential commit hook.
        if !Self::validate(
            txn_idx,
            last_input_output,
            shared_sync_params.global_module_cache,
            shared_sync_params.versioned_cache,
            // Module cache is not versioned (published at commit), so validation after
            // commit might observe later publishes (higher txn index) and be incorrect.
            // Hence, we skip the paranoid module validation after commit.
            // TODO(BlockSTMv2): Do the additional checking in sequential commit hook,
            // when modules have been published. Update the comment here as skipping
            // in V2 is needed for a different, code cache implementation related reason.
            true,
        ) {
            return Err(code_invariant_error(format!(
                "Final Validation in post-processing failed for txn {}",
                txn_idx
            )));
        }

        let parallel_state = ParallelState::<T>::new(
            shared_sync_params.versioned_cache,
            scheduler,
            shared_sync_params.start_shared_counter,
            shared_sync_params.delayed_field_id_counter,
            0,
            // Incarnation does not matter here (no re-execution & interrupts)
            // TODO(BlockSTMv2): we could still provide the latest incarnation.
        );
        let latest_view = LatestView::new(
            shared_sync_params.base_view,
            shared_sync_params.global_module_cache,
            runtime_environment,
            ViewState::Sync(parallel_state),
            txn_idx,
        );

        let finalized_groups = groups_to_finalize!(last_input_output, txn_idx)
            .map(|((group_key, metadata_op), is_read_needing_exchange)| {
                let (finalized_group, group_size) = shared_sync_params
                    .versioned_cache
                    .group_data()
                    .finalize_group(&group_key, txn_idx)?;

                map_finalized_group::<T>(
                    group_key,
                    finalized_group,
                    group_size,
                    metadata_op,
                    is_read_needing_exchange,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let materialized_finalized_groups =
            map_id_to_values_in_group_writes(finalized_groups, &latest_view)?;

        let serialized_groups =
            serialize_groups::<T>(materialized_finalized_groups).map_err(|e| {
                code_invariant_error(format!("Panic error in serializing groups {e:?}"))
            })?;

        let resource_write_set = last_input_output.resource_write_set(txn_idx)?;
        let resource_writes_to_materialize = resource_writes_to_materialize!(
            resource_write_set,
            last_input_output,
            last_input_output,
            txn_idx
        )?;
        let materialized_resource_write_set =
            map_id_to_values_in_write_set(resource_writes_to_materialize, &latest_view)?;

        let events = last_input_output.events(txn_idx);
        let materialized_events = map_id_to_values_events(events, &latest_view)?;
        let aggregator_v1_delta_writes = Self::materialize_aggregator_v1_delta_writes(
            txn_idx,
            last_input_output,
            shared_sync_params.versioned_cache,
            shared_sync_params.base_view,
        );

        // This call finalizes the output and may not be concurrent with any other
        // accesses to the output (e.g. querying the write-set, events, etc), as
        // these read accesses are not synchronized and assumed to have terminated.
        last_input_output.record_materialized_txn_output(
            txn_idx,
            aggregator_v1_delta_writes,
            materialized_resource_write_set
                .into_iter()
                .chain(serialized_groups)
                .collect(),
            materialized_events,
        )
    }

    fn record_finalized_output(
        &self,
        txn_idx: TxnIndex,
        output_idx: TxnIndex,
        shared_sync_params: &SharedSyncParams<T, E, S>,
    ) -> Result<(), PanicError> {
        if output_idx < txn_idx {
            return Err(code_invariant_error(format!(
                "Index to record finalized output {} is less than txn index {}",
                output_idx, txn_idx
            )));
        }

        let last_input_output = shared_sync_params.last_input_output;
        if let Some(txn_commit_listener) = &self.transaction_commit_hook {
            match last_input_output.txn_output(txn_idx).unwrap().as_ref() {
                ExecutionStatus::Success(output) | ExecutionStatus::SkipRest(output) => {
                    txn_commit_listener.on_transaction_committed(output_idx, output);
                },
                ExecutionStatus::Abort(_) => {
                    txn_commit_listener.on_execution_aborted(output_idx);
                },
                ExecutionStatus::SpeculativeExecutionAbortError(msg)
                | ExecutionStatus::DelayedFieldsCodeInvariantError(msg) => {
                    panic!("Cannot be materializing with {}", msg);
                },
            }
        }

        let mut final_results = shared_sync_params.final_results.acquire();

        match last_input_output.take_output(txn_idx)? {
            ExecutionStatus::Success(t) => {
                final_results[output_idx as usize] = t;
            },
            ExecutionStatus::SkipRest(t) => {
                final_results[output_idx as usize] = t;
            },
            ExecutionStatus::Abort(_) => (),
            ExecutionStatus::SpeculativeExecutionAbortError(msg)
            | ExecutionStatus::DelayedFieldsCodeInvariantError(msg) => {
                panic!("Cannot be materializing with {}", msg);
            },
        };
        Ok(())
    }

    fn worker_loop(
        &self,
        executor: &E,
        environment: &AptosEnvironment,
        block: &TP,
        scheduler: &Scheduler,
        skip_module_reads_validation: &AtomicBool,
        shared_sync_params: &SharedSyncParams<T, E, S>,
        num_workers: usize,
    ) -> Result<(), PanicOr<ParallelBlockExecutionError>> {
        let num_txns = block.num_txns();

        // Shared environment used by each executor.
        let runtime_environment = environment.runtime_environment();

        let versioned_cache = shared_sync_params.versioned_cache;
        let last_input_output = shared_sync_params.last_input_output;
        let base_view = shared_sync_params.base_view;
        let global_module_cache = shared_sync_params.global_module_cache;
        let maybe_block_epilogue_txn_idx = shared_sync_params.maybe_block_epilogue_txn_idx;
        let scheduler_wrapper = SchedulerWrapper::V1(scheduler, skip_module_reads_validation);

        let _timer = WORK_WITH_TASK_SECONDS.start_timer();
        let mut scheduler_task = SchedulerTask::Retry;

        let drain_commit_queue = || -> Result<(), PanicError> {
            while let Ok(txn_idx) = scheduler.pop_from_commit_queue() {
                self.materialize_txn_commit(
                    txn_idx,
                    scheduler_wrapper,
                    runtime_environment,
                    shared_sync_params,
                )?;
                self.record_finalized_output(txn_idx, txn_idx, shared_sync_params)?;
            }
            Ok(())
        };

        loop {
            if let SchedulerTask::ValidationTask(txn_idx, incarnation, _) = &scheduler_task {
                if *incarnation as usize > num_workers.pow(2) + num_txns + 30 {
                    // Something is wrong if we observe high incarnations (e.g. a bug
                    // might manifest as an execution-invalidation cycle). Break out
                    // to fallback to sequential execution.
                    error!("Observed incarnation {} of txn {txn_idx}", *incarnation);
                    return Err(PanicOr::Or(ParallelBlockExecutionError::IncarnationTooHigh));
                }
            }

            while scheduler.should_coordinate_commits() {
                while let Some((txn_idx, incarnation)) = scheduler.try_commit() {
                    if txn_idx + 1 == num_txns as u32
                        && matches!(
                            scheduler_task,
                            SchedulerTask::ExecutionTask(_, _, ExecutionTaskType::Execution)
                        )
                    {
                        return Err(PanicOr::from(code_invariant_error(
                            "All transactions can be committed, can't have execution task",
                        )));
                    }

                    if self.prepare_and_queue_commit_ready_txn(
                        txn_idx,
                        incarnation,
                        num_txns as u32,
                        executor,
                        block,
                        num_workers,
                        runtime_environment,
                        scheduler_wrapper,
                        shared_sync_params,
                    )? {
                        // We set the variable here and process after commit lock is released.
                        *maybe_block_epilogue_txn_idx.acquire().dereference_mut() =
                            Some(txn_idx + 1);
                    }
                }
                scheduler.queueing_commits_mark_done();
            }

            drain_commit_queue()?;

            scheduler_task = match scheduler_task {
                SchedulerTask::ValidationTask(txn_idx, incarnation, wave) => {
                    let valid = Self::validate(
                        txn_idx,
                        last_input_output,
                        global_module_cache,
                        versioned_cache,
                        skip_module_reads_validation.load(Ordering::Relaxed),
                    );
                    Self::update_on_validation(
                        txn_idx,
                        incarnation,
                        valid,
                        wave,
                        last_input_output,
                        versioned_cache,
                        scheduler,
                    )?
                },
                SchedulerTask::ExecutionTask(
                    txn_idx,
                    incarnation,
                    ExecutionTaskType::Execution,
                ) => Self::execute(
                    txn_idx,
                    incarnation,
                    block.get_txn(txn_idx),
                    &block.get_auxiliary_info(txn_idx),
                    Some(scheduler),
                    last_input_output,
                    versioned_cache,
                    executor,
                    base_view,
                    global_module_cache,
                    runtime_environment,
                    ParallelState::new(
                        versioned_cache,
                        scheduler_wrapper,
                        shared_sync_params.start_shared_counter,
                        shared_sync_params.delayed_field_id_counter,
                        incarnation,
                    ),
                )?,
                SchedulerTask::ExecutionTask(_, _, ExecutionTaskType::Wakeup(condvar)) => {
                    {
                        let (lock, cvar) = &*condvar;

                        // Mark dependency resolved.
                        let mut lock = lock.lock();
                        *lock = DependencyStatus::Resolved;
                        // Wake up the process waiting for dependency.
                        cvar.notify_one();
                    }

                    scheduler.next_task()
                },
                SchedulerTask::Retry => scheduler.next_task(),
                SchedulerTask::Done => {
                    drain_commit_queue()?;
                    break Ok(());
                },
            }
        }
    }

    fn worker_loop_v2(
        &self,
        executor: &E,
        block: &TP,
        environment: &AptosEnvironment,
        worker_id: u32,
        num_workers: u32,
        scheduler: &SchedulerV2,
        shared_sync_params: &SharedSyncParams<'_, '_, T, E, S>,
    ) -> Result<(), PanicOr<ParallelBlockExecutionError>> {
        let num_txns = block.num_txns() as u32;

        let _work_with_task_timer = WORK_WITH_TASK_SECONDS.start_timer();

        // Shared environment used by each executor.
        let runtime_environment = environment.runtime_environment();

        let scheduler_wrapper = SchedulerWrapper::V2(scheduler, worker_id);
        let base_view = shared_sync_params.base_view;
        let versioned_cache = shared_sync_params.versioned_cache;
        let last_input_output = shared_sync_params.last_input_output;
        let global_module_cache = shared_sync_params.global_module_cache;

        loop {
            while scheduler.commit_hooks_try_lock() {
                // Perform sequential commit hooks.
                while let Some((txn_idx, incarnation)) = scheduler.start_commit()? {
                    if self.prepare_and_queue_commit_ready_txn(
                        txn_idx,
                        incarnation,
                        num_txns,
                        executor,
                        block,
                        num_workers as usize,
                        runtime_environment,
                        scheduler_wrapper,
                        shared_sync_params,
                    )? {
                        // We set the variable here and process after commit lock is released.
                        *shared_sync_params
                            .maybe_block_epilogue_txn_idx
                            .acquire()
                            .dereference_mut() = Some(txn_idx + 1);
                    }
                }

                scheduler.commit_hooks_unlock();
            }

            match scheduler.next_task(worker_id)? {
                TaskKind::Execute(txn_idx, incarnation) => {
                    if incarnation > num_workers.pow(2) + num_txns + 30 {
                        // Something is wrong if we observe high incarnations (e.g. a bug
                        // might manifest as an execution-invalidation cycle). Break out
                        // to fallback to sequential execution.
                        error!("Observed incarnation {} of txn {txn_idx}", incarnation);
                        return Err(PanicOr::Or(ParallelBlockExecutionError::IncarnationTooHigh));
                    }

                    Self::execute_v2(
                        worker_id,
                        txn_idx,
                        incarnation,
                        block.get_txn(txn_idx),
                        &block.get_auxiliary_info(txn_idx),
                        last_input_output,
                        versioned_cache,
                        executor,
                        base_view,
                        shared_sync_params.global_module_cache,
                        runtime_environment,
                        ParallelState::new(
                            versioned_cache,
                            scheduler_wrapper,
                            shared_sync_params.start_shared_counter,
                            shared_sync_params.delayed_field_id_counter,
                            incarnation,
                        ),
                        scheduler,
                    )?;
                },
                TaskKind::PostCommitProcessing(txn_idx) => {
                    self.materialize_txn_commit(
                        txn_idx,
                        scheduler_wrapper,
                        runtime_environment,
                        shared_sync_params,
                    )?;
                    self.record_finalized_output(txn_idx, txn_idx, shared_sync_params)?;
                },
                TaskKind::NextTask => {
                    // TODO: Anything intelligent to do here?.
                },
                TaskKind::ModuleValidation(txn_idx, incarnation, modules_to_validate) => {
                    Self::module_validation_v2(
                        txn_idx,
                        incarnation,
                        scheduler,
                        modules_to_validate,
                        last_input_output,
                        global_module_cache,
                        versioned_cache,
                    )?;
                    scheduler.finish_cold_validation_requirement(
                        worker_id,
                        txn_idx,
                        incarnation,
                        false, // Was not deferred (obtained as a task).
                    )?;
                },
                TaskKind::Done => {
                    break;
                },
            }
        }

        Ok(())
    }

    /// Common finalization logic for both BlockSTM and BlockSTMv2 parallel execution.
    /// Handles commit task validation, error checking, state updates, and cleanup.
    /// maybe_executor must be initialized if there was no error during parallel execution.
    fn finalize_parallel_execution(
        &self,
        maybe_executor: Option<E>,
        signature_verified_block: &TP,
        has_remaining_commit_tasks: bool,
        transaction_slice_metadata: &TransactionSliceMetadata,
        scheduler: SchedulerWrapper,
        environment: &AptosEnvironment,
        shared_sync_params: &SharedSyncParams<T, E, S>,
    ) -> Result<Option<T>, PanicError> {
        let _timer = PARALLEL_FINALIZE_SECONDS.start_timer();
        let mut maybe_block_epilogue_txn = None;

        let versioned_cache = shared_sync_params.versioned_cache;
        let num_txns = signature_verified_block.num_txns();
        let final_results = shared_sync_params.final_results;
        let last_input_output = shared_sync_params.last_input_output;
        let start_shared_counter = 0;
        let shared_counter = shared_sync_params.delayed_field_id_counter;
        let base_view = shared_sync_params.base_view;
        let block_limit_processor = shared_sync_params.block_limit_processor;

        if has_remaining_commit_tasks {
            return Err(code_invariant_error(
                "BlockSTMv2: Commit tasks not drained after parallel execution",
            ));
        }

        if final_results.dereference().len() != num_txns + 1 {
            // If this error fires, then the final results length mismatch is
            // due to a bug in the executor.
            return Err(code_invariant_error(format!(
                "Final results length mismatch: {} != {} + 1",
                final_results.dereference().len(),
                num_txns
            )));
        }

        // TODO: test block epilogue append logic once its generation is made a trait
        // method on T (and can be easily mocked).
        if let Some(epilogue_txn_idx) = *shared_sync_params
            .maybe_block_epilogue_txn_idx
            .dereference()
        {
            if epilogue_txn_idx == 0
                || epilogue_txn_idx as usize > num_txns
                || !final_results.dereference()[epilogue_txn_idx as usize - 1]
                    .check_materialization()?
                || final_results.dereference()[epilogue_txn_idx as usize - 1]
                    .after_materialization()?
                    .has_new_epoch_event()
            {
                // If this error fires, and epilogue_txn_idx is not 0 or > num_txns,
                // then is_retry_check_after_commit would have created a panic error,
                // internally logging the reason.
                return Err(code_invariant_error(format!(
                            "Output preceding epilogue txn {} must neither be retry nor have new epoch event",
                            epilogue_txn_idx
                        )));
            }
            if final_results.dereference()[epilogue_txn_idx as usize].check_materialization()? {
                return Err(code_invariant_error(format!(
                    "Output at epilogue txn index {} must be placeholder (is_retry set)",
                    epilogue_txn_idx
                )));
            }

            if let Some(epilogue_txn) = self.generate_block_epilogue_if_needed(
                signature_verified_block,
                transaction_slice_metadata,
                final_results.dereference().iter(),
                epilogue_txn_idx,
                block_limit_processor,
                environment,
            )? {
                let block_epilogue_aux_info = if num_txns > 0 {
                    // Sample a few transactions to check the auxiliary info pattern
                    let sample_aux_infos: Vec<_> = (0..std::cmp::min(num_txns, 3))
                        .map(|i| signature_verified_block.get_auxiliary_info(i as TxnIndex))
                        .collect();

                    let all_auxiliary_infos_are_none = sample_aux_infos
                        .iter()
                        .all(|info| info.transaction_index().is_none());

                    if all_auxiliary_infos_are_none {
                        // If existing auxiliary infos are None, use None for consistency (version 0 behavior)
                        A::new_empty()
                    } else {
                        // Otherwise, use the standard function (version 1 behavior)
                        A::auxiliary_info_at_txn_index(num_txns as u32)
                    }
                } else {
                    // Fallback if no transactions in block
                    A::new_empty()
                };

                let executor = maybe_executor.as_ref().ok_or_else(|| {
                    code_invariant_error("Block epilogue txn requires executor to be initialized")
                })?;

                let module_cache = shared_sync_params.global_module_cache;
                let runtime_environment = environment.runtime_environment();

                let incarnation = scheduler.prepare_for_block_epilogue::<T, E>(
                    epilogue_txn_idx,
                    last_input_output,
                    versioned_cache,
                )?;

                Self::execute_txn_after_commit(
                    &epilogue_txn,
                    &block_epilogue_aux_info,
                    epilogue_txn_idx,
                    incarnation,
                    scheduler,
                    versioned_cache,
                    last_input_output,
                    start_shared_counter,
                    shared_counter,
                    executor,
                    base_view,
                    module_cache,
                    runtime_environment,
                )?;
                self.materialize_txn_commit(
                    epilogue_txn_idx,
                    scheduler,
                    runtime_environment,
                    shared_sync_params,
                )?;
                self.record_finalized_output(
                    epilogue_txn_idx,
                    num_txns as TxnIndex,
                    shared_sync_params,
                )?;

                maybe_block_epilogue_txn = Some(epilogue_txn);
            }
        }
        if maybe_block_epilogue_txn.is_none() {
            // Remove the placeholder output if the block epilogue txn was not executed.
            final_results.acquire().dereference_mut().pop();
        }

        Ok(maybe_block_epilogue_txn)
    }

    pub(crate) fn execute_transactions_parallel_v2(
        &self,
        signature_verified_block: &TP,
        base_view: &S,
        transaction_slice_metadata: &TransactionSliceMetadata,
        module_cache_manager_guard: &mut AptosModuleCacheManagerGuard,
    ) -> Result<BlockOutput<T, E::Output>, ()> {
        let _timer = PARALLEL_EXECUTION_SECONDS.start_timer();
        // BlockSTMv2 should have less restrictions on the number of workers but we
        // still sanity check that it is not instantiated w. concurrency level 1.
        // (since it makes sense to use sequential execution in this case).
        assert!(
            self.config.local.concurrency_level > 1,
            "Must use sequential execution"
        );

        let num_txns = signature_verified_block.num_txns();
        if num_txns == 0 {
            return Ok(BlockOutput::new(vec![], None));
        }

        let num_workers = self.config.local.concurrency_level.min(num_txns / 2).max(2) as u32;
        // +1 for potential BlockEpilogue txn.
        let final_results = ExplicitSyncWrapper::new(
            (0..num_txns + 1)
                .map(|_| E::Output::skip_output())
                .collect::<Vec<_>>(),
        );

        let block_limit_processor = ExplicitSyncWrapper::new(BlockGasLimitProcessor::new(
            base_view,
            self.config.onchain.block_gas_limit_type.clone(),
            self.config.onchain.block_gas_limit_override(),
            num_txns,
        ));
        let block_epilogue_txn_idx = ExplicitSyncWrapper::new(None);
        let num_txns = num_txns as u32;

        let start_delayed_field_id_counter = gen_id_start_value(false);
        let delayed_field_id_counter = AtomicU32::new(start_delayed_field_id_counter);

        let shared_maybe_error = AtomicBool::new(false);

        // +1 for potential BlockEpilogue txn.
        let last_input_output = TxnLastInputOutput::new(num_txns + 1);
        let mut versioned_cache = MVHashMap::new();
        let scheduler = SchedulerV2::new(num_txns, num_workers);

        let shared_sync_params: SharedSyncParams<'_, '_, T, E, S> = SharedSyncParams {
            base_view,
            versioned_cache: &versioned_cache,
            global_module_cache: module_cache_manager_guard.module_cache(),
            last_input_output: &last_input_output,
            delayed_field_id_counter: &delayed_field_id_counter,
            start_shared_counter: start_delayed_field_id_counter,
            block_limit_processor: &block_limit_processor,
            final_results: &final_results,
            maybe_block_epilogue_txn_idx: &block_epilogue_txn_idx,
        };

        let timer = RAYON_EXECUTION_SECONDS.start_timer();
        let worker_ids: Vec<u32> = (0..num_workers).collect();
        let maybe_executor = ExplicitSyncWrapper::new(None);
        self.executor_thread_pool.scope(|s| {
            for worker_id in &worker_ids {
                s.spawn(|_| {
                    let environment = module_cache_manager_guard.environment();
                    let executor = {
                        let _init_timer = VM_INIT_SECONDS.start_timer();
                        E::init(&environment.clone(), shared_sync_params.base_view)
                    };

                    if let Err(err) = self.worker_loop_v2(
                        &executor,
                        signature_verified_block,
                        environment,
                        *worker_id,
                        num_workers,
                        &scheduler,
                        &shared_sync_params,
                    ) {
                        // If there are multiple errors, they all get logged: FatalVMError is
                        // logged at construction, below we log CodeInvariantErrors.
                        if let PanicOr::CodeInvariantError(err_msg) = err {
                            alert!(
                                "[BlockSTMv2] worker loop: CodeInvariantError({:?})",
                                err_msg
                            );
                        }
                        shared_maybe_error.store(true, Ordering::SeqCst);

                        // Make sure to halt the scheduler if it hasn't already been halted.
                        scheduler.halt();
                    }

                    if *worker_id == 0 {
                        maybe_executor.acquire().replace(executor);
                    }
                });
            }
        });
        drop(timer);

        let (has_error, maybe_block_epilogue_txn) = if shared_maybe_error.load(Ordering::SeqCst) {
            (true, None)
        } else {
            match self.finalize_parallel_execution(
                maybe_executor.into_inner(),
                signature_verified_block,
                !scheduler.post_commit_processing_queue_is_empty(),
                transaction_slice_metadata,
                SchedulerWrapper::V2(&scheduler, 0),
                module_cache_manager_guard.environment(),
                &shared_sync_params,
            ) {
                Ok(maybe_block_epilogue_txn) => {
                    // Update state counters & insert verified modules into cache (safe after error check).
                    counters::update_state_counters(versioned_cache.stats(), true);
                    (
                        module_cache_manager_guard
                            .module_cache_mut()
                            .insert_verified(versioned_cache.take_modules_iter())
                            .is_err(),
                        maybe_block_epilogue_txn,
                    )
                },
                Err(_) => (true, None),
            }
        };

        // Explicit async drops even when there is an error.
        DEFAULT_DROPPER.schedule_drop((last_input_output, scheduler, versioned_cache));

        if has_error {
            return Err(());
        }

        // Return final result
        Ok(BlockOutput::new(
            final_results.into_inner(),
            maybe_block_epilogue_txn,
        ))
    }

    pub(crate) fn execute_transactions_parallel(
        &self,
        signature_verified_block: &TP,
        base_view: &S,
        transaction_slice_metadata: &TransactionSliceMetadata,
        module_cache_manager_guard: &mut AptosModuleCacheManagerGuard,
    ) -> Result<BlockOutput<T, E::Output>, ()> {
        let _timer = PARALLEL_EXECUTION_SECONDS.start_timer();
        // Using parallel execution with 1 thread currently will not work as it
        // will only have a coordinator role but no workers for rolling commit.
        // Need to special case no roles (commit hook by thread itself) to run
        // w. concurrency_level = 1 for some reason.
        assert!(
            self.config.local.concurrency_level > 1,
            "Must use sequential execution"
        );

        let mut versioned_cache = MVHashMap::new();
        let start_shared_counter = gen_id_start_value(false);
        let shared_counter = AtomicU32::new(start_shared_counter);

        let num_txns = signature_verified_block.num_txns();
        if num_txns == 0 {
            return Ok(BlockOutput::new(vec![], None));
        }

        let num_workers = self.config.local.concurrency_level.min(num_txns / 2).max(2);
        let block_limit_processor = ExplicitSyncWrapper::new(BlockGasLimitProcessor::new(
            base_view,
            self.config.onchain.block_gas_limit_type.clone(),
            self.config.onchain.block_gas_limit_override(),
            num_txns + 1,
        ));
        let shared_maybe_error = AtomicBool::new(false);

        let final_results = ExplicitSyncWrapper::new(
            // +1 for potential BlockEpilogue txn.
            (0..(num_txns + 1))
                .map(|_| E::Output::skip_output())
                .collect::<Vec<_>>(),
        );

        let block_epilogue_txn_idx = ExplicitSyncWrapper::new(None);

        let num_txns = num_txns as u32;

        let skip_module_reads_validation = AtomicBool::new(true);
        // +1 for potential BlockEpilogue txn.
        let last_input_output = TxnLastInputOutput::new(num_txns + 1);
        let scheduler = Scheduler::new(num_txns);

        let timer = RAYON_EXECUTION_SECONDS.start_timer();
        let worker_ids: Vec<u32> = (0..num_workers as u32).collect();
        let maybe_executor = ExplicitSyncWrapper::new(None);

        let shared_sync_params: SharedSyncParams<'_, '_, T, E, S> = SharedSyncParams {
            base_view,
            versioned_cache: &versioned_cache,
            global_module_cache: module_cache_manager_guard.module_cache(),
            last_input_output: &last_input_output,
            delayed_field_id_counter: &shared_counter,
            start_shared_counter,
            block_limit_processor: &block_limit_processor,
            final_results: &final_results,
            maybe_block_epilogue_txn_idx: &block_epilogue_txn_idx,
        };

        self.executor_thread_pool.scope(|s| {
            for worker_id in &worker_ids {
                s.spawn(|_| {
                    let environment = module_cache_manager_guard.environment();
                    let executor = {
                        let _init_timer = VM_INIT_SECONDS.start_timer();
                        E::init(&environment.clone(), base_view)
                    };

                    if let Err(err) = self.worker_loop(
                        &executor,
                        environment,
                        signature_verified_block,
                        &scheduler,
                        &skip_module_reads_validation,
                        &shared_sync_params,
                        num_workers,
                    ) {
                        // If there are multiple errors, they all get logged:
                        // ModulePathReadWriteError and FatalVMError variant is logged at construction,
                        // and below we log CodeInvariantErrors.
                        if let PanicOr::CodeInvariantError(err_msg) = err {
                            alert!("[BlockSTM] worker loop: CodeInvariantError({:?})", err_msg);
                        }
                        shared_maybe_error.store(true, Ordering::SeqCst);

                        // Make sure to halt the scheduler if it hasn't already been halted.
                        scheduler.halt();
                    }

                    if *worker_id == 0 {
                        maybe_executor.acquire().replace(executor);
                    }
                });
            }
        });
        drop(timer);

        let (has_error, maybe_block_epilogue_txn) = if shared_maybe_error.load(Ordering::SeqCst) {
            (true, None)
        } else {
            match self.finalize_parallel_execution(
                maybe_executor.into_inner(),
                signature_verified_block,
                scheduler.pop_from_commit_queue().is_ok(),
                transaction_slice_metadata,
                SchedulerWrapper::V1(&scheduler, &skip_module_reads_validation),
                module_cache_manager_guard.environment(),
                &shared_sync_params,
            ) {
                Ok(maybe_block_epilogue_txn) => {
                    // Update state counters & insert verified modules into cache (safe after error check).
                    counters::update_state_counters(versioned_cache.stats(), true);
                    (
                        module_cache_manager_guard
                            .module_cache_mut()
                            .insert_verified(versioned_cache.take_modules_iter())
                            .is_err(),
                        maybe_block_epilogue_txn,
                    )
                },
                Err(_) => (true, None),
            }
        };

        // Explicit async drops even when there is an error.
        DEFAULT_DROPPER.schedule_drop((last_input_output, scheduler, versioned_cache));

        if has_error {
            return Err(());
        }

        // Return final result
        Ok(BlockOutput::new(
            final_results.into_inner(),
            maybe_block_epilogue_txn,
        ))
    }

    fn gen_block_epilogue<'a>(
        &self,
        block_id: HashValue,
        signature_verified_block: &TP,
        outputs: impl Iterator<Item = &'a E::Output>,
        epilogue_txn_idx: TxnIndex,
        block_end_info: TBlockEndInfoExt<T::Key>,
        features: &Features,
    ) -> Result<T, PanicError> {
        // TODO(grao): Remove this check once AIP-88 is fully enabled.
        if !self
            .config
            .onchain
            .block_gas_limit_type
            .add_block_limit_outcome_onchain()
        {
            return Ok(T::state_checkpoint(block_id));
        }
        if !features.is_calculate_transaction_fee_for_distribution_enabled() {
            return Ok(T::block_epilogue_v0(
                block_id,
                block_end_info.to_persistent(),
            ));
        }

        let mut amount = BTreeMap::new();

        // TODO(HotState): there are three possible paths where the block epilogue
        // output is passed to the DB:
        //   1. a block from consensus is executed: the VM outputs the block end info
        //      and the block epilogue transaction and output are generated here.
        //   2. a chunk re-executed: The VM will see the block epilogue transaction and
        //      should output the transaction output by looking at the block end info
        //      embedded in the epilogue transaction (and maybe the state view).
        //   3. a chunk replayed by transaction output: we get the transaction output
        //      directly.

        for (i, output) in outputs.enumerate().take(epilogue_txn_idx as usize) {
            // TODO(grao): Also include other transactions that is "Keep" if we are confident
            // that we successfully charge enough gas amount as it appears in the FeeStatement
            // for every corner cases.
            if !output.is_materialized_and_success() {
                continue;
            }
            let output_after_guard = output.after_materialization()?;
            let fee_statement = output_after_guard.fee_statement();

            let txn = signature_verified_block.get_txn(i as TxnIndex);
            if let Some(user_txn) = txn.try_as_signed_user_txn() {
                let auxiliary_info = signature_verified_block.get_auxiliary_info(i as TxnIndex);
                if let Some(proposer_index) = auxiliary_info.proposer_index() {
                    let gas_price = user_txn.gas_unit_price();
                    let total_gas_unit = fee_statement.gas_used();
                    // Total gas unit here includes the storage fee (deposit), which is not
                    // available for distribution. Only the execution gas and IO gas are available
                    // to distribute. Note here we deliberately NOT use the execution gas and IO
                    // gas value from the fee statement, because they might round up during the
                    // calculation and the sum of them could be larger than the actual value we
                    // burn. Instead we use the total amount (which is the total we've burnt)
                    // minus the storage deposit (round up), to avoid over distribution.
                    // We burn a fix amount of gas per gas unit.
                    let gas_price_to_burn = self.config.onchain.gas_price_to_burn();
                    if gas_price > gas_price_to_burn {
                        let gas_unit_available_to_distribute = total_gas_unit
                            .saturating_sub(fee_statement.storage_fee_used().div_ceil(gas_price));
                        if gas_unit_available_to_distribute > 0 {
                            let fee_to_distribute =
                                gas_unit_available_to_distribute * (gas_price - gas_price_to_burn);
                            *amount.entry(proposer_index).or_insert(0) += fee_to_distribute;
                        }
                    }
                }
            }
        }
        Ok(T::block_epilogue_v1(
            block_id,
            block_end_info,
            FeeDistribution::new(amount),
        ))
    }

    fn apply_output_sequential(
        txn_idx: TxnIndex,
        runtime_environment: &RuntimeEnvironment,
        global_module_cache: &GlobalModuleCache<
            ModuleId,
            CompiledModule,
            Module,
            AptosModuleExtension,
        >,
        unsync_map: &UnsyncMap<T::Key, T::Tag, T::Value, DelayedFieldID>,
        output_before_guard: &<E::Output as TransactionOutput>::BeforeMaterializationGuard<'_>,
        resource_write_set: Vec<(T::Key, Arc<T::Value>, Option<Arc<MoveTypeLayout>>)>,
    ) -> Result<(), SequentialBlockExecutionError<E::Error>> {
        for (key, write_op, layout) in resource_write_set.into_iter() {
            unsync_map.write(key, write_op, layout);
        }

        for (group_key, (metadata_op, group_size, group_ops)) in
            output_before_guard.resource_group_write_set().into_iter()
        {
            unsync_map.insert_group_ops(&group_key, group_ops, group_size)?;
            unsync_map.write(group_key, Arc::new(metadata_op), None);
        }

        for (key, write_op) in output_before_guard.aggregator_v1_write_set().into_iter() {
            unsync_map.write(key, Arc::new(write_op), None);
        }

        for write in output_before_guard.module_write_set().values() {
            add_module_write_to_module_cache::<T>(
                write,
                txn_idx,
                runtime_environment,
                global_module_cache,
                unsync_map.module_cache(),
            )?;
        }

        let mut second_phase = Vec::new();
        let mut updates = HashMap::new();
        for (id, change) in output_before_guard.delayed_field_change_set().into_iter() {
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

    pub(crate) fn execute_transactions_sequential(
        &self,
        signature_verified_block: &TP,
        base_view: &S,
        transaction_slice_metadata: &TransactionSliceMetadata,
        module_cache_manager_guard: &mut AptosModuleCacheManagerGuard,
        resource_group_bcs_fallback: bool,
    ) -> Result<BlockOutput<T, E::Output>, SequentialBlockExecutionError<E::Error>> {
        let num_txns = signature_verified_block.num_txns();

        if num_txns == 0 {
            return Ok(BlockOutput::new(vec![], None));
        }

        let init_timer = VM_INIT_SECONDS.start_timer();
        let environment = module_cache_manager_guard.environment();
        let executor = E::init(environment, base_view);
        drop(init_timer);

        let runtime_environment = environment.runtime_environment();
        let start_counter = gen_id_start_value(true);
        let counter = RefCell::new(start_counter);
        let unsync_map = UnsyncMap::new();

        let mut ret = Vec::with_capacity(num_txns + 1);

        let mut block_limit_processor = BlockGasLimitProcessor::<T, S>::new(
            base_view,
            self.config.onchain.block_gas_limit_type.clone(),
            self.config.onchain.block_gas_limit_override(),
            num_txns + 1,
        );

        let mut block_epilogue_txn = None;
        let mut idx = 0;
        while idx <= num_txns {
            let txn = if idx != num_txns {
                signature_verified_block.get_txn(idx as TxnIndex)
            } else if block_epilogue_txn.is_some() {
                block_epilogue_txn.as_ref().unwrap()
            } else {
                break;
            };
            let auxiliary_info = signature_verified_block.get_auxiliary_info(idx as TxnIndex);
            let latest_view = LatestView::<T, S>::new(
                base_view,
                module_cache_manager_guard.module_cache(),
                runtime_environment,
                ViewState::Unsync(SequentialState::new(&unsync_map, start_counter, &counter)),
                idx as TxnIndex,
            );
            let res =
                executor.execute_transaction(&latest_view, txn, &auxiliary_info, idx as TxnIndex);
            let must_skip = matches!(res, ExecutionStatus::SkipRest(_));
            match res {
                ExecutionStatus::Abort(err) => {
                    if let Some(commit_hook) = &self.transaction_commit_hook {
                        commit_hook.on_execution_aborted(idx as TxnIndex);
                    }
                    error!(
                        "Sequential execution FatalVMError by transaction {}",
                        idx as TxnIndex
                    );
                    // Record the status indicating the unrecoverable VM failure.
                    return Err(SequentialBlockExecutionError::ErrorToReturn(
                        BlockExecutionError::FatalVMError(err),
                    ));
                },
                ExecutionStatus::DelayedFieldsCodeInvariantError(msg) => {
                    if let Some(commit_hook) = &self.transaction_commit_hook {
                        commit_hook.on_execution_aborted(idx as TxnIndex);
                    }
                    alert!("Sequential execution DelayedFieldsCodeInvariantError error by transaction {}: {}", idx as TxnIndex, msg);
                    return Err(SequentialBlockExecutionError::ErrorToReturn(
                        BlockExecutionError::FatalBlockExecutorError(code_invariant_error(msg)),
                    ));
                },
                ExecutionStatus::SpeculativeExecutionAbortError(msg) => {
                    if let Some(commit_hook) = &self.transaction_commit_hook {
                        commit_hook.on_execution_aborted(idx as TxnIndex);
                    }
                    alert!("Sequential execution SpeculativeExecutionAbortError error by transaction {}: {}", idx as TxnIndex, msg);
                    return Err(SequentialBlockExecutionError::ErrorToReturn(
                        BlockExecutionError::FatalBlockExecutorError(code_invariant_error(msg)),
                    ));
                },
                ExecutionStatus::Success(output) | ExecutionStatus::SkipRest(output) => {
                    let output_before_guard = output.before_materialization()?;
                    // Calculating the accumulated gas costs of the committed txns.

                    let approx_output_size = self
                        .config
                        .onchain
                        .block_gas_limit_type
                        .block_output_limit()
                        .map(|_| {
                            output_before_guard.output_approx_size()
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

                    let sequential_reads = latest_view.take_sequential_reads();
                    let read_write_summary = self
                        .config
                        .onchain
                        .block_gas_limit_type
                        .conflict_penalty_window()
                        .map(|_| {
                            ReadWriteSummary::new(
                                sequential_reads.get_read_summary(),
                                output_before_guard.get_write_summary(),
                            )
                        });

                    block_limit_processor.accumulate_fee_statement(
                        output_before_guard.fee_statement(),
                        read_write_summary,
                        approx_output_size,
                    );

                    // Drop to acquire a write lock, then re-assign the output_before_guard.
                    drop(output_before_guard);
                    output.legacy_sequential_materialize_agg_v1(&latest_view);
                    let output_before_guard = output.before_materialization()?;

                    assert_eq!(
                        output_before_guard.aggregator_v1_delta_set().len(),
                        0,
                        "Sequential execution must materialize deltas"
                    );

                    if resource_group_bcs_fallback {
                        // Dynamic change set optimizations are enabled, and resource group serialization
                        // previously failed in bcs serialization for preparing final transaction outputs.
                        // TODO: remove this fallback when txn errors can be created from block executor.

                        let finalize = |group_key| -> (BTreeMap<_, _>, ResourceGroupSize) {
                            let (group, size) = unsync_map.finalize_group(&group_key);

                            (
                                group
                                    .map(|(resource_tag, value_with_layout)| {
                                        let value = match value_with_layout {
                                            ValueWithLayout::RawFromStorage(value)
                                            | ValueWithLayout::Exchanged(value, _) => value,
                                        };
                                        (
                                            resource_tag,
                                            value
                                                .extract_raw_bytes()
                                                .expect("Deletions should already be applied"),
                                        )
                                    })
                                    .collect(),
                                size,
                            )
                        };

                        // The IDs are not exchanged but it doesn't change the types (Bytes) or size.
                        let serialization_error = output_before_guard
                            .group_reads_needing_delayed_field_exchange()
                            .iter()
                            .any(|(group_key, _)| {
                                fail_point!("fail-point-resource-group-serialization", |_| {
                                    true
                                });

                                let (finalized_group, group_size) = finalize(group_key.clone());
                                match bcs::to_bytes(&finalized_group) {
                                    Ok(group) => {
                                        (!finalized_group.is_empty() || group_size.get() != 0)
                                            && group.len() as u64 != group_size.get()
                                    },
                                    Err(_) => true,
                                }
                            })
                            || output_before_guard
                                .resource_group_write_set()
                                .into_iter()
                                .any(|(group_key, (_, output_group_size, group_ops))| {
                                    fail_point!("fail-point-resource-group-serialization", |_| {
                                        true
                                    });

                                    let (mut finalized_group, group_size) = finalize(group_key);
                                    if output_group_size.get() != group_size.get() {
                                        return false;
                                    }
                                    for (value_tag, (group_op, _)) in group_ops {
                                        if group_op.is_deletion() {
                                            finalized_group.remove(&value_tag);
                                        } else {
                                            finalized_group.insert(
                                                value_tag,
                                                group_op
                                                    .extract_raw_bytes()
                                                    .expect("Not a deletion"),
                                            );
                                        }
                                    }
                                    match bcs::to_bytes(&finalized_group) {
                                        Ok(group) => {
                                            (!finalized_group.is_empty() || group_size.get() != 0)
                                                && group.len() as u64 != group_size.get()
                                        },
                                        Err(_) => true,
                                    }
                                });

                        if serialization_error {
                            // The corresponding error / alert must already be triggered, the goal in sequential
                            // fallback is to just skip any transactions that would cause such serialization errors.
                            alert!("Discarding transaction because serialization failed in bcs fallback");
                            ret.push(E::Output::discard_output(
                                StatusCode::DELAYED_FIELD_OR_BLOCKSTM_CODE_INVARIANT_ERROR,
                            ));
                            idx += 1;
                            continue;
                        }
                    };

                    // Apply the writes.
                    let resource_write_set = output_before_guard.resource_write_set();
                    Self::apply_output_sequential(
                        idx as TxnIndex,
                        runtime_environment,
                        module_cache_manager_guard.module_cache(),
                        &unsync_map,
                        &output_before_guard,
                        resource_write_set.clone(),
                    )?;

                    // If dynamic change set materialization part (indented for clarity/variable scope):
                    {
                        let finalized_groups = groups_to_finalize!(output_before_guard,)
                            .map(|((group_key, metadata_op), is_read_needing_exchange)| {
                                let (group_ops_iter, group_size) =
                                    unsync_map.finalize_group(&group_key);
                                map_finalized_group::<T>(
                                    group_key,
                                    group_ops_iter.collect(),
                                    group_size,
                                    metadata_op,
                                    is_read_needing_exchange,
                                )
                            })
                            .collect::<Result<Vec<_>, _>>()?;
                        let materialized_finalized_groups =
                            map_id_to_values_in_group_writes(finalized_groups, &latest_view)?;
                        let serialized_groups =
                            serialize_groups::<T>(materialized_finalized_groups).map_err(|_| {
                                SequentialBlockExecutionError::ResourceGroupSerializationError
                            })?;

                        let resource_writes_to_materialize = resource_writes_to_materialize!(
                            resource_write_set,
                            output_before_guard,
                            unsync_map,
                        )?;
                        // Replace delayed field id with values in resource write set and read set.
                        let materialized_resource_write_set = map_id_to_values_in_write_set(
                            resource_writes_to_materialize,
                            &latest_view,
                        )?;

                        // Replace delayed field id with values in events
                        let materialized_events = map_id_to_values_events(
                            Box::new(output_before_guard.get_events().into_iter()),
                            &latest_view,
                        )?;
                        // Output before guard holds a read lock, drop before incorporating materialized
                        // output which needs a write lock.
                        drop(output_before_guard);

                        output.incorporate_materialized_txn_output(
                            // No aggregator v1 delta writes are needed for sequential execution.
                            // They are already handled because we passed materialize_deltas=true
                            // to execute_transaction.
                            vec![],
                            materialized_resource_write_set
                                .into_iter()
                                .chain(serialized_groups.into_iter())
                                .collect(),
                            materialized_events,
                        )?;
                    }
                    // If dynamic change set is disabled, this can be used to assert nothing needs patching instead:
                    //   output.set_txn_output_for_non_dynamic_change_set();

                    if sequential_reads.incorrect_use {
                        return Err(
                            code_invariant_error("Incorrect use in sequential execution").into(),
                        );
                    }

                    if let Some(commit_hook) = &self.transaction_commit_hook {
                        commit_hook.on_transaction_committed(idx as TxnIndex, &output);
                    }
                    ret.push(output);
                },
            };

            if idx == num_txns {
                break;
            }

            idx += 1;

            if must_skip || block_limit_processor.should_end_block_sequential() || idx == num_txns {
                let mut has_reconfig = false;
                if let Some(last_output) = ret.last() {
                    if last_output.after_materialization()?.has_new_epoch_event() {
                        has_reconfig = true;
                    }
                }
                ret.resize_with(num_txns, E::Output::skip_output);
                if let Some(block_id) =
                    transaction_slice_metadata.append_state_checkpoint_to_block()
                {
                    if !has_reconfig {
                        block_epilogue_txn = Some(self.gen_block_epilogue(
                            block_id,
                            signature_verified_block,
                            ret.iter(),
                            idx as TxnIndex,
                            block_limit_processor.get_block_end_info(),
                            module_cache_manager_guard.environment().features(),
                        )?);
                    } else {
                        info!("Reach epoch ending, do not append BlockEpilogue txn, block_id: {block_id:?}.");
                    }
                }
                idx = num_txns;
            }
        }

        block_limit_processor.finish_sequential_update_counters_and_log_info(
            ret.len() as u32,
            num_txns as u32 + block_epilogue_txn.as_ref().map_or(0, |_| 1),
        );

        counters::update_state_counters(unsync_map.stats(), false);
        module_cache_manager_guard
            .module_cache_mut()
            .insert_verified(unsync_map.into_modules_iter())?;

        Ok(BlockOutput::new(ret, block_epilogue_txn))
    }

    pub fn execute_block(
        &self,
        signature_verified_block: &TP,
        base_view: &S,
        transaction_slice_metadata: &TransactionSliceMetadata,
        module_cache_manager_guard: &mut AptosModuleCacheManagerGuard,
    ) -> BlockExecutionResult<BlockOutput<T, E::Output>, E::Error> {
        let _timer = BLOCK_EXECUTOR_INNER_EXECUTE_BLOCK.start_timer();

        if self.config.local.concurrency_level > 1 {
            let parallel_result = if self.config.local.blockstm_v2 {
                self.execute_transactions_parallel_v2(
                    signature_verified_block,
                    base_view,
                    transaction_slice_metadata,
                    module_cache_manager_guard,
                )
            } else {
                self.execute_transactions_parallel(
                    signature_verified_block,
                    base_view,
                    transaction_slice_metadata,
                    module_cache_manager_guard,
                )
            };

            // If parallel gave us result, return it
            if let Ok(output) = parallel_result {
                return Ok(output);
            }

            if !self.config.local.allow_fallback {
                panic!("Parallel execution failed and fallback is not allowed");
            }

            // All logs from the parallel execution should be cleared and not reported.
            // Clear by re-initializing the speculative logs.
            init_speculative_logs(signature_verified_block.num_txns() + 1);

            // Flush all caches to re-run from the "clean" state.
            module_cache_manager_guard
                .environment()
                .runtime_environment()
                .flush_struct_name_and_tag_caches();
            module_cache_manager_guard.module_cache_mut().flush();

            info!("parallel execution requiring fallback");
        }

        // If we didn't run parallel, or it didn't finish successfully - run sequential
        let sequential_result = self.execute_transactions_sequential(
            signature_verified_block,
            base_view,
            transaction_slice_metadata,
            module_cache_manager_guard,
            false,
        );

        // If sequential gave us result, return it
        let sequential_error = match sequential_result {
            Ok(output) => {
                return Ok(output);
            },
            Err(SequentialBlockExecutionError::ResourceGroupSerializationError) => {
                if !self.config.local.allow_fallback {
                    panic!("Parallel execution failed and fallback is not allowed");
                }

                // TODO[agg_v2](cleanup): check if sequential execution logs anything in the speculative logs,
                // and whether clearing them below is needed at all.
                // All logs from the first pass of sequential execution should be cleared and not reported.
                // Clear by re-initializing the speculative logs.
                init_speculative_logs(signature_verified_block.num_txns());

                let sequential_result = self.execute_transactions_sequential(
                    signature_verified_block,
                    base_view,
                    transaction_slice_metadata,
                    module_cache_manager_guard,
                    true,
                );

                // If sequential gave us result, return it
                match sequential_result {
                    Ok(output) => {
                        return Ok(output);
                    },
                    Err(SequentialBlockExecutionError::ResourceGroupSerializationError) => {
                        BlockExecutionError::FatalBlockExecutorError(code_invariant_error(
                            "resource group serialization during bcs fallback should not happen",
                        ))
                    },
                    Err(SequentialBlockExecutionError::ErrorToReturn(err)) => err,
                }
            },
            Err(SequentialBlockExecutionError::ErrorToReturn(err)) => err,
        };

        if self.config.local.discard_failed_blocks {
            // We cannot execute block, discard everything (including block metadata and validator transactions)
            // (TODO: maybe we should add fallback here to first try BlockMetadataTransaction alone)
            let error_code = match sequential_error {
                BlockExecutionError::FatalBlockExecutorError(_) => {
                    StatusCode::DELAYED_FIELD_OR_BLOCKSTM_CODE_INVARIANT_ERROR
                },
                BlockExecutionError::FatalVMError(_) => {
                    StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR
                },
            };
            let ret = (0..signature_verified_block.num_txns())
                .map(|_| E::Output::discard_output(error_code))
                .collect();
            return Ok(BlockOutput::new(ret, None));
        }

        Err(sequential_error)
    }

    /// Helper method that generates and prepares the block epilogue transaction.
    /// Returns Some(Transaction) if a block epilogue should be created, None otherwise.
    /// If Some(Transaction) is returned, it is guaranteed that any concurrent speculative
    /// changes are either all applied to shared state or will never be applied.
    fn generate_block_epilogue_if_needed<'a>(
        &self,
        block: &TP,
        transaction_slice_metadata: &TransactionSliceMetadata,
        outputs: impl Iterator<Item = &'a E::Output>,
        epilogue_txn_idx: TxnIndex,
        block_limit_processor: &ExplicitSyncWrapper<BlockGasLimitProcessor<T, S>>,
        environment: &AptosEnvironment,
    ) -> Result<Option<T>, PanicError> {
        // We only do this for block (when the block_id is returned). For other cases
        // like state sync or replay, the BlockEpilogue txn should already in the input
        // and we don't need to add one here.
        if let Some(block_id) = transaction_slice_metadata.append_state_checkpoint_to_block() {
            let epilogue_txn = self.gen_block_epilogue(
                block_id,
                block,
                outputs,
                epilogue_txn_idx,
                block_limit_processor.acquire().get_block_end_info(),
                environment.features(),
            )?;

            Ok(Some(epilogue_txn))
        } else {
            Ok(None)
        }
    }
}
