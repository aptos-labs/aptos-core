// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    captured_reads::CapturedReads,
    code_cache_global::{add_module_write_to_module_cache, GlobalModuleCache},
    code_cache_global_manager::AptosModuleCacheManagerGuard,
    counters::{
        self, BLOCK_EXECUTOR_INNER_EXECUTE_BLOCK, PARALLEL_EXECUTION_SECONDS,
        RAYON_EXECUTION_SECONDS, TASK_EXECUTE_SECONDS, TASK_VALIDATE_SECONDS, VM_INIT_SECONDS,
        WORK_WITH_TASK_SECONDS,
    },
    errors::*,
    executor_utilities::*,
    explicit_sync_wrapper::ExplicitSyncWrapper,
    limit_processor::BlockGasLimitProcessor,
    scheduler::{DependencyStatus, ExecutionTaskType, Scheduler, SchedulerTask, Wave},
    scheduler_v2::{AbortManager, SchedulerV2, TaskKind},
    scheduler_wrapper::SchedulerWrapper,
    task::{ExecutionStatus, ExecutorTask, TransactionOutput},
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
    on_chain_config::{BlockGasLimitType, Features},
    state_store::{state_value::StateValue, TStateView},
    transaction::{
        block_epilogue::BlockEndInfo, AuxiliaryInfoTrait, BlockExecutableTransaction, BlockOutput,
        FeeDistribution,
    },
    vm::modules::AptosModuleExtension,
    write_set::{TransactionWrite, WriteOp},
};
use aptos_vm_environment::environment::AptosEnvironment;
use aptos_vm_logging::{alert, clear_speculative_txn_logs, init_speculative_logs, prelude::*};
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
use scopeguard::defer;
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    marker::{PhantomData, Sync},
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc,
    },
};

struct SharedSyncParams<'a, T, E, S>
where
    T: BlockExecutableTransaction,
    E: ExecutorTask<Txn = T>,
    S: TStateView<Key = T::Key> + Sync,
{
    // TODO: should not need to pass base view.
    base_view: &'a S,
    scheduler: &'a SchedulerV2,
    versioned_cache: &'a MVHashMap<T::Key, T::Tag, T::Value, DelayedFieldID>,
    global_module_cache:
        &'a GlobalModuleCache<ModuleId, CompiledModule, Module, AptosModuleExtension>,
    last_input_output: &'a TxnLastInputOutput<T, E::Output, E::Error>,
    delayed_field_id_counter: &'a AtomicU32,
    block_limit_processor: &'a ExplicitSyncWrapper<BlockGasLimitProcessor<T>>,
    final_results: &'a ExplicitSyncWrapper<Vec<E::Output>>,
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

    fn process_execution_result<'a>(
        execution_result: &'a ExecutionStatus<E::Output, E::Error>,
        read_set: &mut CapturedReads<T, ModuleId, CompiledModule, Module, AptosModuleExtension>,
        txn_idx: TxnIndex,
    ) -> Result<Option<&'a E::Output>, PanicError> {
        match execution_result {
            ExecutionStatus::Success(output) | ExecutionStatus::SkipRest(output) => {
                Ok(Some(output))
            },
            ExecutionStatus::SpeculativeExecutionAbortError(_msg) => {
                // TODO(BlockSTMv2): cleaner to rename or distinguish V2 early abort
                // from DeltaApplicationFailure.
                read_set.capture_delayed_field_read_error(&PanicOr::Or(
                    MVDelayedFieldsError::DeltaApplicationFailure,
                ));
                Ok(None)
            },
            ExecutionStatus::Abort(_err) => Ok(None),
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

        let mut resource_group_write_set =
            maybe_output.map_or(HashMap::new(), |output| output.resource_group_write_set());

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
            for (id, change) in output.delayed_field_change_set().into_iter() {
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
        signature_verified_block: &TP,
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

        // TODO(BlockSTMv2): proper integration w. execution pooling for performance.
        let txn = signature_verified_block.get_txn(idx_to_execute);
        let auxiliary_info = signature_verified_block.get_auxiliary_info(idx_to_execute);

        let mut abort_manager = AbortManager::new(idx_to_execute, incarnation, scheduler);
        let sync_view = LatestView::new(
            base_view,
            global_module_cache,
            runtime_environment,
            ViewState::Sync(parallel_state),
            idx_to_execute,
        );
        let execution_result =
            executor.execute_transaction(&sync_view, txn, &auxiliary_info, idx_to_execute);

        let mut read_set = sync_view.take_parallel_reads();
        if read_set.is_incorrect_use() {
            return Err(code_invariant_error(format!(
                "Incorrect use detected in CapturedReads after executing txn = {idx_to_execute} incarnation = {incarnation}"
            )));
        }

        let maybe_output =
            Self::process_execution_result(&execution_result, &mut read_set, idx_to_execute)?;

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
        let mut resource_write_set = vec![];
        if let Some(output) = maybe_output {
            resource_write_set = output.resource_write_set();
            for (key, value, maybe_layout) in resource_write_set.clone().into_iter() {
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
            for (key, value) in output.aggregator_v1_write_set().into_iter() {
                prev_modified_aggregator_v1_keys.remove(&key);

                versioned_cache.data().write(
                    key,
                    idx_to_execute,
                    incarnation,
                    Arc::new(value),
                    None,
                );
            }
            for (key, delta) in output.aggregator_v1_delta_set().into_iter() {
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

        last_input_output.record(
            idx_to_execute,
            read_set,
            execution_result,
            resource_write_set,
        );

        // It is important to call finish_execution after recording the input/output.
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
    ) -> Result<bool, PanicOr<ParallelBlockExecutionError>> {
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

        let mut prev_modified_resource_keys = last_input_output
            .modified_resource_keys(idx_to_execute)
            .map_or_else(HashSet::new, |keys| keys.map(|(k, _)| k).collect());
        let mut prev_modified_group_keys: HashMap<T::Key, HashSet<T::Tag>> = last_input_output
            .modified_group_key_and_tags_cloned(idx_to_execute)
            .into_iter()
            .collect();

        let mut read_set = sync_view.take_parallel_reads();
        if read_set.is_incorrect_use() {
            return Err(PanicOr::from(code_invariant_error(format!(
                "Incorrect use detected in CapturedReads after executing txn = {} incarnation = {}",
                idx_to_execute, incarnation
            ))));
        }

        let processed_output =
            Self::process_execution_result(&execution_result, &mut read_set, idx_to_execute)?;

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
            for (group_key, (group_metadata_op, group_size, group_ops)) in
                output.resource_group_write_set().into_iter()
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

            let resource_write_set = output.resource_write_set();

            // Then, process resource & aggregator_v1 writes.
            for (k, v, maybe_layout) in resource_write_set.clone().into_iter().chain(
                output
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
            for (k, d) in output.aggregator_v1_delta_set().into_iter() {
                if !prev_modified_resource_keys.remove(&k) {
                    needs_suffix_validation = true;
                }
                versioned_cache.data().add_delta(k, idx_to_execute, d);
            }

            Ok(resource_write_set)
        };

        let resource_write_set = match processed_output {
            Some(output) => apply_updates(output)?,
            None => vec![],
        };

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

        last_input_output.record(
            idx_to_execute,
            read_set,
            execution_result,
            resource_write_set,
        );
        Ok(needs_suffix_validation)
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
        let read_set = last_input_output.read_set(idx_to_validate).ok_or_else(|| {
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
        if blockstm_v2_incarnation < incarnation_to_validate {
            return Err(code_invariant_error(format!(
                "For txn_idx {}, read set incarnation {} < incarnation to validate {}",
                idx_to_validate, blockstm_v2_incarnation, incarnation_to_validate
            )));
        }
        if blockstm_v2_incarnation > incarnation_to_validate {
            // No need to validate as a newer incarnation has already been executed
            // and recorded its output.
            return Ok(true);
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
        let read_set = last_input_output
            .read_set(idx_to_validate)
            .expect("[BlockSTM]: Prior read-set must be recorded");

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

    fn update_transaction_on_abort(
        txn_idx: TxnIndex,
        last_input_output: &TxnLastInputOutput<T, E::Output, E::Error>,
        versioned_cache: &MVHashMap<T::Key, T::Tag, T::Value, DelayedFieldID>,
    ) {
        counters::SPECULATIVE_ABORT_COUNT.inc();

        // Any logs from the aborted execution should be cleared and not reported.
        clear_speculative_txn_logs(txn_idx as usize);

        // Not valid and successfully aborted, mark the latest write/delta sets as estimates.
        if let Some(keys) = last_input_output.modified_resource_keys(txn_idx) {
            for (k, _) in keys {
                versioned_cache.data().mark_estimate(&k, txn_idx);
            }
        }

        // Group metadata lives in same versioned cache as data / resources.
        // We are not marking metadata change as estimate, but after a transaction execution
        // changes metadata, suffix validation is guaranteed to be triggered. Estimation affecting
        // execution behavior is left to size, which uses a heuristic approach.
        last_input_output
            .for_each_resource_group_key_and_tags(txn_idx, |key, tags| {
                versioned_cache
                    .group_data()
                    .mark_estimate(key, txn_idx, tags);
                Ok(())
            })
            .expect("Passed closure always returns Ok");

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
        versioned_cache: &MVHashMap<T::Key, T::Tag, T::Value, DelayedFieldID>,
        scheduler: &Scheduler,
    ) -> Result<SchedulerTask, PanicError> {
        let aborted = !valid && scheduler.try_abort(txn_idx, incarnation);

        if aborted {
            Self::update_transaction_on_abort(txn_idx, last_input_output, versioned_cache);
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
        let read_set = last_input_output
            .read_set(txn_idx)
            .ok_or_else(|| code_invariant_error("Read set must be recorded"))?;

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

    /// This method may be executed by different threads / workers, but is guaranteed to be executed
    /// non-concurrently by the scheduling in parallel executor. This allows to perform light logic
    /// related to committing a transaction in a simple way and without excessive synchronization
    /// overhead. On the other hand, materialization that happens after commit (& after this method)
    /// is concurrent and deals with logic such as patching delayed fields / resource groups
    /// in outputs, which is heavier (due to serialization / deserialization, copies, etc). Moreover,
    /// since prepare_and_queue_commit_ready_txns takes care of synchronization in the flat-combining
    /// way, the materialization can be almost embarrassingly parallelizable.
    /// TODO(BlockSTMv2): Change the signature to use shared_sync_params.
    #[allow(clippy::too_many_arguments)]
    fn prepare_and_queue_commit_ready_txn(
        &self,
        txn_idx: TxnIndex,
        incarnation: Incarnation,
        num_txns: TxnIndex,
        block_gas_limit_type: &BlockGasLimitType,
        scheduler: SchedulerWrapper,
        versioned_cache: &MVHashMap<T::Key, T::Tag, T::Value, DelayedFieldID>,
        last_input_output: &TxnLastInputOutput<T, E::Output, E::Error>,
        block_limit_processor: &ExplicitSyncWrapper<BlockGasLimitProcessor<T>>,
        base_view: &S,
        global_module_cache: &GlobalModuleCache<
            ModuleId,
            CompiledModule,
            Module,
            AptosModuleExtension,
        >,
        runtime_environment: &RuntimeEnvironment,
        start_shared_counter: u32,
        shared_counter: &AtomicU32,
        executor: &E,
        block: &TP,
        num_workers: usize,
    ) -> Result<(), PanicOr<ParallelBlockExecutionError>> {
        let block_limit_processor = &mut block_limit_processor.acquire();
        let mut side_effect_at_commit = false;

        if !Self::validate_and_commit_delayed_fields(
            txn_idx,
            versioned_cache,
            last_input_output,
            scheduler.is_v2(),
        )? {
            // Transaction needs to be re-executed, one final time.
            side_effect_at_commit = true;
            counters::SPECULATIVE_ABORT_COUNT.inc();

            let parallel_state = ParallelState::new(
                versioned_cache,
                scheduler,
                start_shared_counter,
                shared_counter,
                incarnation + 1,
            );

            match scheduler.as_v2() {
                None => {
                    Self::update_transaction_on_abort(txn_idx, last_input_output, versioned_cache);
                    // We are going to skip reducing validation index here, as we
                    // are executing immediately, and will reduce it unconditionally
                    // after execution, inside finish_execution_during_commit.
                    // Because of that, we can also ignore _needs_suffix_validation result.
                    let _needs_suffix_validation = Self::execute(
                        txn_idx,
                        incarnation + 1,
                        block.get_txn(txn_idx),
                        &block.get_auxiliary_info(txn_idx),
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
                    scheduler.direct_abort(txn_idx, incarnation, true)?;
                    Self::execute_v2(
                        worker_id,
                        txn_idx,
                        incarnation + 1,
                        block,
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
            )
            .unwrap_or(false)
            {
                return Err(code_invariant_error(format!(
                    "Delayed field validation after re-execution failed for txn {}",
                    txn_idx
                ))
                .into());
            }
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

        last_input_output
            .check_fatal_vm_error(txn_idx)
            .map_err(PanicOr::Or)?;
        // Handle a potential vm error, then check invariants on the recorded outputs.
        last_input_output.check_execution_status_during_commit(txn_idx)?;

        if let Some(fee_statement) = last_input_output.fee_statement(txn_idx) {
            let approx_output_size = block_gas_limit_type.block_output_limit().and_then(|_| {
                last_input_output
                    .output_approx_size(txn_idx)
                    .map(|approx_output| {
                        approx_output
                            + if block_gas_limit_type.include_user_txn_size_in_block_output() {
                                block.get_txn(txn_idx).user_txn_bytes_len()
                            } else {
                                0
                            } as u64
                    })
            });
            let txn_read_write_summary = block_gas_limit_type.conflict_penalty_window().map(|_| {
                last_input_output.record_storage_keys_read(txn_idx);
                last_input_output.get_txn_read_write_summary(txn_idx)
            });

            // For committed txns with Success status, calculate the accumulated gas costs.
            block_limit_processor.accumulate_fee_statement(
                fee_statement,
                txn_read_write_summary,
                approx_output_size,
            );

            if txn_idx < num_txns - 1 && block_limit_processor.should_end_block_parallel() {
                // Set the execution output status to be SkipRest, to skip the rest of the txns.
                last_input_output.update_to_skip_rest(txn_idx)?;
            }
        }

        let skips = last_input_output.block_skips_rest_at_idx(txn_idx);

        // Add before halt, so SchedulerV2 can organically observe and process post commit
        // processing tasks even after it has halted.
        scheduler.add_to_post_commit(txn_idx)?;

        // While the above propagate errors and lead to eventually halting parallel execution,
        // below we may halt the execution without an error in cases when:
        // a) all transactions are scheduled for committing
        // b) we skip_rest after a transaction
        // Either all txn committed, or a committed txn caused an early halt.
        if (txn_idx + 1 == num_txns || skips) && scheduler.halt() {
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

        Ok(())
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

    fn materialize_txn_commit(
        &self,
        txn_idx: TxnIndex,
        versioned_cache: &MVHashMap<T::Key, T::Tag, T::Value, DelayedFieldID>,
        scheduler: SchedulerWrapper,
        start_shared_counter: u32,
        shared_counter: &AtomicU32,
        last_input_output: &TxnLastInputOutput<T, E::Output, E::Error>,
        base_view: &S,
        global_module_cache: &GlobalModuleCache<
            ModuleId,
            CompiledModule,
            Module,
            AptosModuleExtension,
        >,
        runtime_environment: &RuntimeEnvironment,
        total_txns_to_materialize: &AtomicU32,
        final_results: &ExplicitSyncWrapper<Vec<E::Output>>,
    ) -> Result<(), PanicError> {
        // Do a final validation for safety as a part of (parallel) post-processing.
        // Delayed fields are already validated in the sequential commit hook.
        if !Self::validate(
            txn_idx,
            last_input_output,
            global_module_cache,
            versioned_cache,
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
            versioned_cache,
            scheduler,
            start_shared_counter,
            shared_counter,
            0,
            // Incarnation does not matter here (no re-execution & interrupts)
            // TODO(BlockSTMv2): we could still provide the latest incarnation.
        );
        let latest_view = LatestView::new(
            base_view,
            global_module_cache,
            runtime_environment,
            ViewState::Sync(parallel_state),
            txn_idx,
        );

        let finalized_groups = groups_to_finalize!(last_input_output, txn_idx)
            .map(|((group_key, metadata_op), is_read_needing_exchange)| {
                let (finalized_group, group_size) = versioned_cache
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

        let resource_write_set = last_input_output.take_resource_write_set(txn_idx);
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
            versioned_cache,
            base_view,
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
        )?;
        if let Some(txn_commit_listener) = &self.transaction_commit_hook {
            match last_input_output.txn_output(txn_idx).unwrap().as_ref() {
                ExecutionStatus::Success(output) | ExecutionStatus::SkipRest(output) => {
                    txn_commit_listener.on_transaction_committed(txn_idx, output);
                },
                ExecutionStatus::Abort(_) => {
                    txn_commit_listener.on_execution_aborted(txn_idx);
                },
                ExecutionStatus::SpeculativeExecutionAbortError(msg)
                | ExecutionStatus::DelayedFieldsCodeInvariantError(msg) => {
                    panic!("Cannot be materializing with {}", msg);
                },
            }
        }

        let mut final_results = final_results.acquire();

        match last_input_output.take_output(txn_idx)? {
            ExecutionStatus::Success(t) => {
                final_results[txn_idx as usize] = t;
            },
            ExecutionStatus::SkipRest(t) => {
                final_results[txn_idx as usize] = t;
                total_txns_to_materialize.store(txn_idx + 1, Ordering::SeqCst);
            },
            ExecutionStatus::Abort(_) => (),
            ExecutionStatus::SpeculativeExecutionAbortError(msg)
            | ExecutionStatus::DelayedFieldsCodeInvariantError(msg) => {
                panic!("Cannot be materializing with {}", msg);
            },
        };
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn worker_loop(
        &self,
        environment: &AptosEnvironment,
        block: &TP,
        transaction_slice_metadata: &TransactionSliceMetadata,
        last_input_output: &TxnLastInputOutput<T, E::Output, E::Error>,
        versioned_cache: &MVHashMap<T::Key, T::Tag, T::Value, DelayedFieldID>,
        scheduler: &Scheduler,
        // TODO: should not need to pass base view.
        base_view: &S,
        global_module_cache: &GlobalModuleCache<
            ModuleId,
            CompiledModule,
            Module,
            AptosModuleExtension,
        >,
        skip_module_reads_validation: &AtomicBool,
        start_shared_counter: u32,
        shared_counter: &AtomicU32,
        block_limit_processor: &ExplicitSyncWrapper<BlockGasLimitProcessor<T>>,
        final_results: &ExplicitSyncWrapper<Vec<E::Output>>,
        block_epilogue_txn: &ExplicitSyncWrapper<Option<T>>,
        num_txns_materialized: &AtomicU32,
        total_txns_to_materialize: &AtomicU32,
        num_running_workers: &AtomicU32,
        num_workers: usize,
    ) -> Result<(), PanicOr<ParallelBlockExecutionError>> {
        defer!( num_running_workers.fetch_sub(1, Ordering::SeqCst); );

        let num_txns = block.num_txns();
        let init_timer = VM_INIT_SECONDS.start_timer();
        let executor = E::init(environment, base_view);
        drop(init_timer);

        // Shared environment used by each executor.
        let runtime_environment = environment.runtime_environment();

        let _timer = WORK_WITH_TASK_SECONDS.start_timer();
        let mut scheduler_task = SchedulerTask::Retry;
        let scheduler_wrapper = SchedulerWrapper::V1(scheduler, skip_module_reads_validation);

        let drain_commit_queue = || -> Result<bool, PanicError> {
            let mut block_epilogue_executed = false;
            while let Ok(txn_idx) = scheduler.pop_from_commit_queue() {
                self.materialize_txn_commit(
                    txn_idx,
                    versioned_cache,
                    scheduler_wrapper,
                    start_shared_counter,
                    shared_counter,
                    last_input_output,
                    base_view,
                    global_module_cache,
                    runtime_environment,
                    total_txns_to_materialize,
                    final_results,
                )?;

                if txn_idx == num_txns as u32 {
                    break;
                }

                let num_txns_materialized =
                    num_txns_materialized.fetch_add(1, Ordering::SeqCst) + 1;
                let total_txns_to_materialize = total_txns_to_materialize.load(Ordering::SeqCst);

                if num_txns_materialized < total_txns_to_materialize {
                    continue;
                } else if num_txns_materialized != total_txns_to_materialize {
                    return Err(code_invariant_error(
                        format!("num_txns_materialized {num_txns_materialized} should never be larger than total_txns_to_materialize {total_txns_to_materialize}."),
                    ));
                }

                while num_running_workers.load(Ordering::SeqCst) > 1 {
                    std::hint::spin_loop();
                }

                let mut outputs = final_results.acquire();
                let has_reconfig = outputs
                    .iter()
                    .rposition(|t| !t.is_retry())
                    .map_or(false, |idx| outputs[idx].has_new_epoch_event());

                // We don't have BlockEpilogue txn for epoch ending block, due to several
                // historical reasons.
                if !has_reconfig {
                    // We only do this for block (when the block_id is returned). For other cases
                    // like state sync or replay, the BlockEpilogue txn should already in the input
                    // and we don't need to add one here.
                    if let Some(block_id) =
                        transaction_slice_metadata.append_state_checkpoint_to_block()
                    {
                        // There could be some txns skipped, we need to make sure the values in
                        // mvhashmap with corresponding indices are properly cleared.
                        versioned_cache.remove_all_at_or_after_for_epilogue(
                            num_txns_materialized,
                            num_txns as u32,
                        );

                        let txn = self.gen_block_epilogue(
                            block_id,
                            block,
                            outputs.dereference(),
                            block_limit_processor.acquire().get_block_end_info(),
                            environment.features(),
                        );
                        outputs.dereference_mut().push(E::Output::skip_output()); // placeholder
                                                                                  // Check if existing auxiliary infos are None to maintain consistency
                        let block_epilogue_aux_info = if num_txns > 0 {
                            // Sample a few transactions to check the auxiliary info pattern
                            let sample_aux_infos: Vec<_> = (0..std::cmp::min(num_txns, 3))
                                .map(|i| block.get_auxiliary_info(i as TxnIndex))
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

                        if Self::execute(
                            num_txns as u32,
                            0,
                            &txn,
                            &block_epilogue_aux_info,
                            last_input_output,
                            versioned_cache,
                            &executor,
                            base_view,
                            global_module_cache,
                            runtime_environment,
                            ParallelState::new(
                                versioned_cache,
                                scheduler_wrapper,
                                start_shared_counter,
                                shared_counter,
                                0,
                            ),
                        ) != Ok(false)
                        {
                            return Err(code_invariant_error(
                                "BlockEpilogue txn should not fail or need validation.",
                            ));
                        }

                        if Self::validate_and_commit_delayed_fields(
                            num_txns as u32,
                            versioned_cache,
                            last_input_output,
                            false,
                        ) != Ok(true)
                        {
                            return Err(code_invariant_error(
                                "BlockEpilogue txn should not need re-execution for delayed fields.",
                            ));
                        };
                        *block_epilogue_txn.acquire().dereference_mut() = Some(txn);
                        block_epilogue_executed = true;
                        scheduler.add_to_commit_queue(num_txns as u32);
                    }
                }
            }
            Ok(block_epilogue_executed)
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

                    self.prepare_and_queue_commit_ready_txn(
                        txn_idx,
                        incarnation,
                        num_txns as u32,
                        &self.config.onchain.block_gas_limit_type,
                        scheduler_wrapper,
                        versioned_cache,
                        last_input_output,
                        block_limit_processor,
                        base_view,
                        global_module_cache,
                        runtime_environment,
                        start_shared_counter,
                        shared_counter,
                        &executor,
                        block,
                        num_workers,
                    )?;
                }
                scheduler.queueing_commits_mark_done();
            }

            let block_epilogue_executed = drain_commit_queue()?;
            if block_epilogue_executed {
                scheduler_task = SchedulerTask::Done;
            }

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
                ) => {
                    let needs_suffix_validation = Self::execute(
                        txn_idx,
                        incarnation,
                        block.get_txn(txn_idx),
                        &block.get_auxiliary_info(txn_idx),
                        last_input_output,
                        versioned_cache,
                        &executor,
                        base_view,
                        global_module_cache,
                        runtime_environment,
                        ParallelState::new(
                            versioned_cache,
                            scheduler_wrapper,
                            start_shared_counter,
                            shared_counter,
                            incarnation,
                        ),
                    )?;
                    scheduler.finish_execution(txn_idx, incarnation, needs_suffix_validation)?
                },
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
        block: &TP,
        environment: &AptosEnvironment,
        worker_id: u32,
        num_workers: u32,
        shared_sync_params: &SharedSyncParams<'_, T, E, S>,
        start_delayed_field_id_counter: u32,
    ) -> Result<(), PanicOr<ParallelBlockExecutionError>> {
        let num_txns = block.num_txns() as u32;
        let executor = {
            let _init_timer = VM_INIT_SECONDS.start_timer();
            E::init(&environment.clone(), shared_sync_params.base_view)
        };

        let _work_with_task_timer = WORK_WITH_TASK_SECONDS.start_timer();

        // Shared environment used by each executor.
        let runtime_environment = environment.runtime_environment();

        let scheduler = shared_sync_params.scheduler;
        let base_view = shared_sync_params.base_view;
        let last_input_output = shared_sync_params.last_input_output;
        let versioned_cache = shared_sync_params.versioned_cache;
        let global_module_cache = shared_sync_params.global_module_cache;
        let delayed_field_id_counter = shared_sync_params.delayed_field_id_counter;
        let scheduler_wrapper = SchedulerWrapper::V2(scheduler, worker_id);

        loop {
            while scheduler.commit_hooks_try_lock() {
                // Perform sequential commit hooks.
                while let Some((txn_idx, incarnation)) = scheduler.start_commit()? {
                    self.prepare_and_queue_commit_ready_txn(
                        txn_idx,
                        incarnation,
                        num_txns,
                        &self.config.onchain.block_gas_limit_type,
                        scheduler_wrapper,
                        versioned_cache,
                        last_input_output,
                        shared_sync_params.block_limit_processor,
                        base_view,
                        global_module_cache,
                        runtime_environment,
                        start_delayed_field_id_counter,
                        delayed_field_id_counter,
                        &executor,
                        block,
                        num_workers as usize,
                    )?;
                }

                scheduler.commit_hooks_unlock();
            }

            let next_task = scheduler.next_task(worker_id);
            match next_task? {
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
                        block,
                        last_input_output,
                        versioned_cache,
                        &executor,
                        base_view,
                        shared_sync_params.global_module_cache,
                        runtime_environment,
                        ParallelState::new(
                            versioned_cache,
                            scheduler_wrapper,
                            start_delayed_field_id_counter,
                            delayed_field_id_counter,
                            incarnation,
                        ),
                        scheduler,
                    )?;
                },
                TaskKind::PostCommitProcessing(txn_idx) => {
                    self.materialize_txn_commit(
                        txn_idx,
                        versioned_cache,
                        scheduler_wrapper,
                        start_delayed_field_id_counter,
                        delayed_field_id_counter,
                        last_input_output,
                        base_view,
                        shared_sync_params.global_module_cache,
                        runtime_environment,
                        // TODO(BlockSTMv2): fix w. block epilogue support
                        &AtomicU32::new(0),
                        shared_sync_params.final_results,
                    )?;
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
    fn finalize_parallel_execution(
        &self,
        shared_maybe_error: &AtomicBool,
        has_remaining_commit_tasks: bool,
        final_results: ExplicitSyncWrapper<Vec<E::Output>>,
        block_epilogue_txn: Option<T>,
        mut versioned_cache: MVHashMap<T::Key, T::Tag, T::Value, DelayedFieldID>,
        scheduler: impl Send + 'static,
        last_input_output: TxnLastInputOutput<T, E::Output, E::Error>,
        module_cache_manager_guard: &mut AptosModuleCacheManagerGuard,
    ) -> Result<BlockOutput<T, E::Output>, ()> {
        // Check for errors or remaining commit tasks before any side effects.
        let mut has_error = shared_maybe_error.load(Ordering::SeqCst);
        if !has_error && has_remaining_commit_tasks {
            alert!("[BlockSTM]: commit tasks not drained after parallel execution");
            shared_maybe_error.store(true, Ordering::Relaxed);
            has_error = true;
        }

        if has_error {
            // Does not hurt to asynchronously drop even in the error case.
            DEFAULT_DROPPER.schedule_drop((last_input_output, scheduler, versioned_cache));

            return Err(());
        }

        // Update state counters & insert verified modules into cache (safe after error check).
        counters::update_state_counters(versioned_cache.stats(), true);
        module_cache_manager_guard
            .module_cache_mut()
            .insert_verified(versioned_cache.take_modules_iter())
            .map_err(|err| {
                alert!("[BlockSTM] Encountered panic error: {:?}", err);
            })?;

        // Explicit async drops
        DEFAULT_DROPPER.schedule_drop((last_input_output, scheduler, versioned_cache));

        // Return final result
        Ok(BlockOutput::new(
            final_results.into_inner(),
            block_epilogue_txn,
        ))
    }

    #[allow(dead_code)]
    pub(crate) fn execute_transactions_parallel_v2(
        &self,
        signature_verified_block: &TP,
        base_view: &S,
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
        let final_results = ExplicitSyncWrapper::new(Vec::with_capacity(num_txns));
        {
            final_results
                .acquire()
                .resize_with(num_txns, E::Output::skip_output);
        }
        let block_limit_processor = ExplicitSyncWrapper::new(BlockGasLimitProcessor::new(
            self.config.onchain.block_gas_limit_type.clone(),
            self.config.onchain.block_gas_limit_override(),
            num_txns,
        ));
        let num_txns = num_txns as u32;

        let start_delayed_field_id_counter = gen_id_start_value(false);
        let delayed_field_id_counter = AtomicU32::new(start_delayed_field_id_counter);

        let shared_maybe_error = AtomicBool::new(false);
        let last_input_output = TxnLastInputOutput::new(num_txns);
        let versioned_cache = MVHashMap::new();
        let scheduler = SchedulerV2::new(num_txns, num_workers);

        let shared_sync_params: SharedSyncParams<'_, T, E, S> = SharedSyncParams {
            base_view,
            scheduler: &scheduler,
            versioned_cache: &versioned_cache,
            global_module_cache: module_cache_manager_guard.module_cache(),
            last_input_output: &last_input_output,
            delayed_field_id_counter: &delayed_field_id_counter,
            block_limit_processor: &block_limit_processor,
            final_results: &final_results,
        };
        let worker_ids: Vec<u32> = (0..num_workers).collect();

        let timer = RAYON_EXECUTION_SECONDS.start_timer();
        self.executor_thread_pool.scope(|s| {
            for worker_id in &worker_ids {
                s.spawn(|_| {
                    if let Err(err) = self.worker_loop_v2(
                        signature_verified_block,
                        module_cache_manager_guard.environment(),
                        *worker_id,
                        num_workers,
                        &shared_sync_params,
                        start_delayed_field_id_counter,
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
                });
            }
        });
        drop(timer);

        self.finalize_parallel_execution(
            &shared_maybe_error,
            !scheduler.post_commit_processing_queue_is_empty(),
            final_results,
            None, // BlockSTMv2 doesn't handle block epilogue yet.
            versioned_cache,
            scheduler,
            last_input_output,
            module_cache_manager_guard,
        )
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

        let versioned_cache = MVHashMap::new();
        let start_shared_counter = gen_id_start_value(false);
        let shared_counter = AtomicU32::new(start_shared_counter);

        let num_txns = signature_verified_block.num_txns();
        if num_txns == 0 {
            return Ok(BlockOutput::new(vec![], None));
        }

        let num_workers = self.config.local.concurrency_level.min(num_txns / 2).max(2);
        let block_limit_processor = ExplicitSyncWrapper::new(BlockGasLimitProcessor::new(
            self.config.onchain.block_gas_limit_type.clone(),
            self.config.onchain.block_gas_limit_override(),
            num_txns + 1,
        ));
        let shared_maybe_error = AtomicBool::new(false);

        let final_results = ExplicitSyncWrapper::new(Vec::with_capacity(num_txns + 1));

        {
            final_results
                .acquire()
                .resize_with(num_txns, E::Output::skip_output);
        }

        let block_epilogue_txn = ExplicitSyncWrapper::new(None);

        let num_txns = num_txns as u32;

        let skip_module_reads_validation = AtomicBool::new(true);
        // +1 for potential BlockEpilogue txn.
        let last_input_output = TxnLastInputOutput::new(num_txns + 1);
        let scheduler = Scheduler::new(num_txns);
        let num_txns_materialized = AtomicU32::new(0);
        let total_txns_to_materialize = AtomicU32::new(num_txns);
        let num_running_workers = AtomicU32::new(num_workers as u32);

        let timer = RAYON_EXECUTION_SECONDS.start_timer();
        self.executor_thread_pool.scope(|s| {
            for _ in 0..num_workers {
                s.spawn(|_| {
                    if let Err(err) = self.worker_loop(
                        module_cache_manager_guard.environment(),
                        signature_verified_block,
                        transaction_slice_metadata,
                        &last_input_output,
                        &versioned_cache,
                        &scheduler,
                        base_view,
                        module_cache_manager_guard.module_cache(),
                        &skip_module_reads_validation,
                        start_shared_counter,
                        &shared_counter,
                        &block_limit_processor,
                        &final_results,
                        &block_epilogue_txn,
                        &num_txns_materialized,
                        &total_txns_to_materialize,
                        &num_running_workers,
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
                });
            }
        });
        drop(timer);

        self.finalize_parallel_execution(
            &shared_maybe_error,
            scheduler.pop_from_commit_queue().is_ok(),
            final_results,
            block_epilogue_txn.into_inner(),
            versioned_cache,
            scheduler,
            last_input_output,
            module_cache_manager_guard,
        )
    }

    fn gen_block_epilogue(
        &self,
        block_id: HashValue,
        signature_verified_block: &TP,
        outputs: &[E::Output],
        block_end_info: BlockEndInfo,
        features: &Features,
    ) -> T {
        // TODO(grao): Remove this check once AIP-88 is fully enabled.
        if !self
            .config
            .onchain
            .block_gas_limit_type
            .add_block_limit_outcome_onchain()
        {
            return T::state_checkpoint(block_id);
        }
        if !features.is_calculate_transaction_fee_for_distribution_enabled() {
            return T::block_epilogue_v0(block_id, block_end_info);
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
        assert!(
            outputs.len() == signature_verified_block.num_txns(),
            "Output must have same size as input."
        );

        for (i, output) in outputs.iter().enumerate() {
            // TODO(grao): Also include other transactions that is "Keep" if we are confident
            // that we successfully charge enough gas amount as it appears in the FeeStatement
            // for every corner cases.
            if !output.is_success() {
                continue;
            }
            let txn = signature_verified_block.get_txn(i as TxnIndex);
            if let Some(user_txn) = txn.try_as_signed_user_txn() {
                let auxiliary_info = signature_verified_block.get_auxiliary_info(i as TxnIndex);
                let proposer_index = auxiliary_info.proposer_index();
                if let Some(proposer_index) = proposer_index {
                    let gas_price = user_txn.gas_unit_price();
                    let fee_statement = output.fee_statement();
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
        T::block_epilogue_v1(block_id, block_end_info, FeeDistribution::new(amount))
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
        output: &E::Output,
        resource_write_set: Vec<(T::Key, Arc<T::Value>, Option<Arc<MoveTypeLayout>>)>,
    ) -> Result<(), SequentialBlockExecutionError<E::Error>> {
        for (key, write_op, layout) in resource_write_set.into_iter() {
            unsync_map.write(key, write_op, layout);
        }

        for (group_key, (metadata_op, group_size, group_ops)) in
            output.resource_group_write_set().into_iter()
        {
            unsync_map.insert_group_ops(&group_key, group_ops, group_size)?;
            unsync_map.write(group_key, Arc::new(metadata_op), None);
        }

        for (key, write_op) in output.aggregator_v1_write_set().into_iter() {
            unsync_map.write(key, Arc::new(write_op), None);
        }

        for write in output.module_write_set().as_ref().values() {
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
        let mut ret = Vec::with_capacity(num_txns);
        let mut block_limit_processor = BlockGasLimitProcessor::<T>::new(
            self.config.onchain.block_gas_limit_type.clone(),
            self.config.onchain.block_gas_limit_override(),
            num_txns,
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

                    let sequential_reads = latest_view.take_sequential_reads();
                    let read_write_summary = self
                        .config
                        .onchain
                        .block_gas_limit_type
                        .conflict_penalty_window()
                        .map(|_| {
                            // TODO(HotState): This probably should go outside this map so it's
                            // done regardless of block_gas_limit_type, but there are some test
                            // failures to sort out. Put it here to unblock some e2e testing first.
                            // Same with parallel execution.
                            output.record_read_set(sequential_reads.get_storage_keys_read());
                            ReadWriteSummary::new(
                                sequential_reads.get_read_summary(),
                                output.get_write_summary(),
                            )
                        });

                    block_limit_processor.accumulate_fee_statement(
                        fee_statement,
                        read_write_summary,
                        approx_output_size,
                    );

                    output.legacy_sequential_materialize_agg_v1(&latest_view);
                    assert_eq!(
                        output.aggregator_v1_delta_set().len(),
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
                        let serialization_error = output
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
                            || output.resource_group_write_set().into_iter().any(
                                |(group_key, (_, output_group_size, group_ops))| {
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
                                },
                            );

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
                    let resource_write_set = output.resource_write_set();
                    Self::apply_output_sequential(
                        idx as TxnIndex,
                        runtime_environment,
                        module_cache_manager_guard.module_cache(),
                        &unsync_map,
                        &output,
                        resource_write_set.clone(),
                    )?;

                    // If dynamic change set materialization part (indented for clarity/variable scope):
                    {
                        let finalized_groups = groups_to_finalize!(output,)
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
                            output,
                            unsync_map,
                        )?;
                        // Replace delayed field id with values in resource write set and read set.
                        let materialized_resource_write_set = map_id_to_values_in_write_set(
                            resource_writes_to_materialize,
                            &latest_view,
                        )?;

                        // Replace delayed field id with values in events
                        let materialized_events = map_id_to_values_events(
                            Box::new(output.get_events().into_iter()),
                            &latest_view,
                        )?;

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
                    if last_output.has_new_epoch_event() {
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
                            &ret,
                            block_limit_processor.get_block_end_info(),
                            module_cache_manager_guard.environment().features(),
                        ));
                    } else {
                        info!("Reach epoch ending, do not append BlockEpilogue txn, block_id: {block_id:?}.");
                    }
                }
                idx = num_txns;
            }
        }

        block_limit_processor
            .finish_sequential_update_counters_and_log_info(ret.len() as u32, num_txns as u32);

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
                unimplemented!("BlockSTMv2 is not fully implemented");
                // self.execute_transactions_parallel_v2(
                //     signature_verified_block,
                //     base_view,
                //     module_cache_manager_guard,
                // )
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
}
