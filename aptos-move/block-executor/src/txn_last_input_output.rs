// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    captured_reads::{CapturedReads, DataRead, ReadKind},
    code_cache_global::{add_module_write_to_module_cache, GlobalModuleCache},
    errors::ParallelBlockExecutionError,
    explicit_sync_wrapper::ExplicitSyncWrapper,
    limit_processor::BlockGasLimitProcessor,
    scheduler_wrapper::SchedulerWrapper,
    task::{BeforeMaterializationOutput, ExecutionStatus, TransactionOutput},
    txn_commit_hook::TransactionCommitHook,
    types::ReadWriteSummary,
};
use aptos_infallible::Mutex;
use aptos_logger::error;
use aptos_mvhashmap::{types::TxnIndex, MVHashMap};
use aptos_types::{
    error::{code_invariant_error, PanicError, PanicOr},
    on_chain_config::BlockGasLimitType,
    state_store::state_value::StateValueMetadata,
    transaction::BlockExecutableTransaction as Transaction,
    vm::modules::AptosModuleExtension,
    write_set::WriteOp,
};
use arc_swap::ArcSwapOption;
use crossbeam::utils::CachePadded;
use fail::fail_point;
use move_binary_format::CompiledModule;
use move_core_types::{language_storage::ModuleId, value::MoveTypeLayout};
use move_vm_runtime::{Module, RuntimeEnvironment};
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
use std::{
    collections::{BTreeSet, HashMap, HashSet},
    fmt::Debug,
    iter::{empty, Iterator},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use triomphe::Arc as TriompheArc;

type TxnInput<T> = CapturedReads<T, ModuleId, CompiledModule, Module, AptosModuleExtension>;

macro_rules! with_success_or_skip_rest {
    // The simple form for a single method call.
    ($self:ident, $txn_idx:ident, $f:ident, $fallback:expr) => {
        with_success_or_skip_rest!(
            $self,
            $txn_idx,
            |t| t.before_materialization().map(|inner| inner.$f()),
            Ok($fallback)
        )
    };
    // The flexible form for any expression.
    ($self:ident, $txn_idx:ident, | $t:ident | $body:expr, $fallback:expr) => {{
        let wrapper = $self.output_wrappers[$txn_idx as usize].lock();
        let status_kind = wrapper.output_status_kind.clone();
        match (&status_kind, &wrapper.output) {
            (OutputStatusKind::Success, Some($t)) | (OutputStatusKind::SkipRest, Some($t)) => $body,
            (OutputStatusKind::Abort(_), None)
            | (OutputStatusKind::SpeculativeExecutionAbortError, None)
            | (OutputStatusKind::DelayedFieldsCodeInvariantError, None)
            | (OutputStatusKind::None, None) => $fallback,
            // The remaining arms are all unreachable.
            (OutputStatusKind::Success, None)
            | (OutputStatusKind::SkipRest, None)
            | (OutputStatusKind::Abort(_), Some(_))
            | (OutputStatusKind::SpeculativeExecutionAbortError, Some(_))
            | (OutputStatusKind::DelayedFieldsCodeInvariantError, Some(_))
            | (OutputStatusKind::None, Some(_)) => {
                unreachable!(
                    "Inconsistent wrapper status kind {:?} and output {:?}",
                    status_kind, wrapper.output
                )
            },
        }
    }};
    // The flexible form for any expression where the output needs to be mutable.
    ($self:ident, $txn_idx:ident, | mut $t:ident | $body:expr, $fallback:expr) => {{
        let mut wrapper = $self.output_wrappers[$txn_idx as usize].lock();
        let status_kind = wrapper.output_status_kind.clone();
        match (&status_kind, &mut wrapper.output) {
            (OutputStatusKind::Success, Some($t)) | (OutputStatusKind::SkipRest, Some($t)) => $body,
            (OutputStatusKind::Abort(_), None)
            | (OutputStatusKind::SpeculativeExecutionAbortError, None)
            | (OutputStatusKind::DelayedFieldsCodeInvariantError, None)
            | (OutputStatusKind::None, None) => $fallback,
            // The remaining arms are all unreachable.
            (OutputStatusKind::Success, None)
            | (OutputStatusKind::SkipRest, None)
            | (OutputStatusKind::Abort(_), Some(_))
            | (OutputStatusKind::SpeculativeExecutionAbortError, Some(_))
            | (OutputStatusKind::DelayedFieldsCodeInvariantError, Some(_))
            | (OutputStatusKind::None, Some(_)) => {
                unreachable!(
                    "Inconsistent wrapper status kind {:?} and output {:?}",
                    status_kind, wrapper.output
                )
            },
        }
    }};
}

#[derive(Debug, PartialEq, Clone)]
enum OutputStatusKind {
    Success,
    SkipRest,
    Abort(String),
    SpeculativeExecutionAbortError,
    DelayedFieldsCodeInvariantError,
    None,
}

struct OutputWrapper<T: Transaction, O: TransactionOutput<Txn = T>> {
    output: Option<O>,
    maybe_read_write_summary: Option<ReadWriteSummary<T>>,
    maybe_approx_output_size: Option<u64>,
    output_status_kind: OutputStatusKind,
}

impl<T: Transaction, O: TransactionOutput<Txn = T>> OutputWrapper<T, O> {
    fn empty_with_status(output_status_kind: OutputStatusKind) -> Self {
        Self {
            output: None,
            maybe_read_write_summary: None,
            maybe_approx_output_size: None,
            output_status_kind,
        }
    }

    fn from_execution_status<E: Debug>(
        output: ExecutionStatus<O, E>,
        read_set: &TxnInput<T>,
        block_gas_limit_type: &BlockGasLimitType,
        user_txn_bytes_len: u64,
    ) -> Result<Self, PanicError> {
        let is_skip_rest = matches!(output, ExecutionStatus::SkipRest(_));

        Ok(match output {
            ExecutionStatus::Success(output) | ExecutionStatus::SkipRest(output) => {
                let output_before_guard = output.before_materialization()?;

                let maybe_approx_output_size =
                    block_gas_limit_type.block_output_limit().map(|_| {
                        output_before_guard.output_approx_size()
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
                            output_before_guard.get_write_summary(),
                        )
                    });
                drop(output_before_guard);

                Self {
                    output: Some(output),
                    maybe_approx_output_size,
                    maybe_read_write_summary,
                    output_status_kind: if is_skip_rest {
                        OutputStatusKind::SkipRest
                    } else {
                        OutputStatusKind::Success
                    },
                }
            },
            ExecutionStatus::Abort(err) => {
                Self::empty_with_status(OutputStatusKind::Abort(format!("{:?}", err)))
            },
            ExecutionStatus::SpeculativeExecutionAbortError(_) => {
                Self::empty_with_status(OutputStatusKind::SpeculativeExecutionAbortError)
            },
            ExecutionStatus::DelayedFieldsCodeInvariantError(_) => {
                Self::empty_with_status(OutputStatusKind::DelayedFieldsCodeInvariantError)
            },
        })
    }

    fn take_output(&mut self) -> Result<O, PanicError> {
        self.check_success_or_skip_status()?;

        self.output.take().ok_or_else(|| {
            code_invariant_error("[BlockSTM]: Output must be recorded after execution")
        })
    }

    fn check_success_or_skip_status(&self) -> Result<&O, PanicError> {
        if self.output_status_kind != OutputStatusKind::Success
            && self.output_status_kind != OutputStatusKind::SkipRest
        {
            return Err(code_invariant_error(format!(
                "Output status {:?}!= success or skip rest",
                self.output_status_kind
            )));
        }

        Ok(self
            .output
            .as_ref()
            .expect("Output must be set when status is success or skip rest"))
    }
}

pub struct TxnLastInputOutput<T: Transaction, O: TransactionOutput<Txn = T>> {
    inputs: Vec<CachePadded<ArcSwapOption<TxnInput<T>>>>, // txn_idx -> input (read set).

    output_wrappers: Vec<CachePadded<Mutex<OutputWrapper<T, O>>>>,
    // Used to record if the latest incarnation of a txn was a failure due to the
    // speculative nature of parallel execution.
    speculative_failures: Vec<CachePadded<AtomicBool>>,
}

impl<T: Transaction, O: TransactionOutput<Txn = T>> TxnLastInputOutput<T, O> {
    /// num_txns passed here is typically larger than the number of txns in the block,
    /// currently by 1 to account for the block epilogue txn.
    pub fn new(num_txns: TxnIndex) -> Self {
        Self {
            inputs: (0..num_txns)
                .map(|_| CachePadded::new(ArcSwapOption::empty()))
                .collect(),
            output_wrappers: (0..num_txns)
                .map(|_| {
                    CachePadded::new(Mutex::new(OutputWrapper::empty_with_status(
                        OutputStatusKind::None,
                    )))
                })
                .collect(),
            speculative_failures: (0..num_txns)
                .map(|_| CachePadded::new(AtomicBool::new(false)))
                .collect(),
        }
    }

    pub(crate) fn record<E: Debug>(
        &self,
        txn_idx: TxnIndex,
        input: TxnInput<T>,
        output: ExecutionStatus<O, E>,
        block_gas_limit_type: &BlockGasLimitType,
        user_txn_bytes_len: u64,
    ) -> Result<(), PanicError> {
        self.speculative_failures[txn_idx as usize].store(false, Ordering::Relaxed);
        *self.output_wrappers[txn_idx as usize].lock() = OutputWrapper::from_execution_status(
            output,
            &input,
            block_gas_limit_type,
            user_txn_bytes_len,
        )?;
        self.inputs[txn_idx as usize].store(Some(Arc::new(input)));

        Ok(())
    }

    pub(crate) fn record_speculative_failure(&self, txn_idx: TxnIndex) {
        self.speculative_failures[txn_idx as usize].store(true, Ordering::Relaxed);
    }

    pub fn fetch_exchanged_data(
        &self,
        key: &T::Key,
        txn_idx: TxnIndex,
    ) -> Result<(TriompheArc<T::Value>, TriompheArc<MoveTypeLayout>), PanicError> {
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

    // Alongside the latest read set, returns the indicator of whether the latest
    // incarnation of the txn resulted in a speculative failure.
    pub(crate) fn read_set(&self, txn_idx: TxnIndex) -> Option<(Arc<TxnInput<T>>, bool)> {
        let input = self.inputs[txn_idx as usize].load_full()?;
        let speculative_failure =
            self.speculative_failures[txn_idx as usize].load(Ordering::Relaxed);
        Some((input, speculative_failure))
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
        block_limit_processor: &mut BlockGasLimitProcessor<T>,
        maybe_block_epilogue_txn_idx: &ExplicitSyncWrapper<Option<TxnIndex>>,
        scheduler: &SchedulerWrapper,
    ) -> Result<(), PanicOr<ParallelBlockExecutionError>> {
        let mut output_wrapper = self.output_wrappers[txn_idx as usize].lock();
        let maybe_read_write_summary = output_wrapper.maybe_read_write_summary.take();

        // Transaction cannot be committed with below statuses, as:
        // - Speculative error must have failed validation.
        // - Execution w. delayed field code error propagates the error directly
        // and does not finish execution. Similar for FatalVMError / abort.
        // - None means there is no output to commit.
        // check_success_or_skip_status below returns an invariant error for all
        // these cases, but we handle Abort case separately first.
        if let OutputStatusKind::Abort(msg) = &output_wrapper.output_status_kind {
            // Fatal VM error.
            error!(
                "FatalVMError from parallel execution {:?} at txn {}",
                msg, txn_idx
            );
            return Err(PanicOr::Or(ParallelBlockExecutionError::FatalVMError));
        }
        let output_before_guard = output_wrapper
            .check_success_or_skip_status()?
            .before_materialization()?;

        let (mut skips_rest, mut must_create_epilogue_txn) =
            if output_wrapper.output_status_kind == OutputStatusKind::SkipRest {
                (true, !output_before_guard.has_new_epoch_event())
            } else {
                assert!(output_wrapper.output_status_kind == OutputStatusKind::Success);
                (
                    false,
                    txn_idx == num_txns - 1 && !output_before_guard.has_new_epoch_event(),
                )
            };
        let fee_statement = output_before_guard.fee_statement();

        // For committed txns, calculate the accumulated gas costs.
        block_limit_processor.accumulate_fee_statement(
            fee_statement,
            maybe_read_write_summary,
            output_wrapper.maybe_approx_output_size,
        );

        if txn_idx < num_txns - 1
            && block_limit_processor.should_end_block_parallel()
            && !skips_rest
        {
            if output_wrapper.output_status_kind == OutputStatusKind::Success {
                must_create_epilogue_txn |= !output_before_guard.has_new_epoch_event();
                drop(output_before_guard);
                output_wrapper.output_status_kind = OutputStatusKind::SkipRest;
            }
            skips_rest = true;
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

        if must_create_epilogue_txn {
            *maybe_block_epilogue_txn_idx.acquire().dereference_mut() = Some(txn_idx + 1);
        }

        Ok(())
    }

    pub(crate) fn notify_listener<L: TransactionCommitHook>(
        &self,
        txn_idx: TxnIndex,
        txn_listener: &L,
    ) -> Result<(), PanicError> {
        let output_wrapper = self.output_wrappers[txn_idx as usize].lock();
        match output_wrapper.output_status_kind {
            OutputStatusKind::Success | OutputStatusKind::SkipRest => {
                txn_listener.on_transaction_committed(
                    txn_idx,
                    output_wrapper
                        .output
                        .as_ref()
                        .expect("Output must be set when status is success or skip rest")
                        .committed_output(),
                );
            },
            OutputStatusKind::Abort(_) => {
                txn_listener.on_execution_aborted(txn_idx);
            },
            OutputStatusKind::SpeculativeExecutionAbortError
            | OutputStatusKind::DelayedFieldsCodeInvariantError
            | OutputStatusKind::None => {
                return Err(code_invariant_error(format!(
                    "Unexpected output status kind {:?}",
                    output_wrapper.output_status_kind
                )));
            },
        }

        Ok(())
    }

    pub(crate) fn for_each_resource_key_no_aggregator_v1(
        &self,
        txn_idx: TxnIndex,
        mut callback: impl FnMut(&T::Key) -> Result<(), PanicError>,
    ) -> Result<(), PanicError> {
        let output_wrapper = self.output_wrappers[txn_idx as usize].lock();
        if let Some(output) = output_wrapper.output.as_ref() {
            output
                .before_materialization()?
                .for_each_resource_key_no_aggregator_v1(&mut callback)?;
        }

        Ok(())
    }

    /// Returns an error if callback returns an error.
    pub(crate) fn for_each_resource_group_key_and_tags(
        &self,
        txn_idx: TxnIndex,
        mut callback: impl FnMut(&T::Key, HashSet<&T::Tag>) -> Result<(), PanicError>,
    ) -> Result<(), PanicError> {
        let output_wrapper = self.output_wrappers[txn_idx as usize].lock();
        if let Some(output) = output_wrapper.output.as_ref() {
            output
                .before_materialization()?
                .for_each_resource_group_key_and_tags(&mut callback)?;
        }

        Ok(())
    }

    pub(crate) fn modified_group_key_and_tags_cloned(
        &self,
        txn_idx: TxnIndex,
    ) -> Vec<(T::Key, HashSet<T::Tag>)> {
        with_success_or_skip_rest!(self, txn_idx, legacy_v1_resource_group_tags, vec![])
            .expect("Output must be set")
    }

    // Extracts a set of resource paths (keys) written or updated during execution from
    // transaction output. The group keys are not included, and the boolean indicates
    // whether the resource is used as an AggregatorV1.
    // Used only in BlockSTMv1. BlockSTMv2 uses modified_resource_keys_no_aggregator_v1
    // and modified_aggregator_v1_keys methods below.
    pub(crate) fn modified_resource_keys(
        &self,
        txn_idx: TxnIndex,
    ) -> Option<impl Iterator<Item = (T::Key, bool)>> {
        with_success_or_skip_rest!(
            self,
            txn_idx,
            |t| {
                let inner = t.before_materialization().expect("Output must be set");
                Some(
                    inner
                        .resource_write_set()
                        .into_iter()
                        .map(|(k, (_, _))| (k, false))
                        .chain(
                            inner
                                .aggregator_v1_write_set()
                                .into_keys()
                                .map(|k| (k, true)),
                        )
                        .chain(
                            inner
                                .aggregator_v1_delta_set()
                                .into_keys()
                                .map(|k| (k, true)),
                        ),
                )
            },
            None
        )
    }

    pub(crate) fn modified_aggregator_v1_keys(
        &self,
        txn_idx: TxnIndex,
    ) -> Option<impl Iterator<Item = T::Key>> {
        with_success_or_skip_rest!(
            self,
            txn_idx,
            |t| {
                let inner = t.before_materialization().expect("Output must be set");
                Some(
                    inner
                        .aggregator_v1_write_set()
                        .into_keys()
                        .chain(inner.aggregator_v1_delta_set().into_keys()),
                )
            },
            None
        )
    }

    // The output needs to be Success or SkipRest, o.w. invariant error is returned.
    pub(crate) fn publish_module_write_set(
        &self,
        txn_idx: TxnIndex,
        global_module_cache: &GlobalModuleCache<
            ModuleId,
            CompiledModule,
            Module,
            AptosModuleExtension,
        >,
        versioned_cache: &MVHashMap<T::Key, T::Tag, T::Value, DelayedFieldID>,
        runtime_environment: &RuntimeEnvironment,
        scheduler: &SchedulerWrapper<'_>,
    ) -> Result<bool, PanicError> {
        let output_wrapper = self.output_wrappers[txn_idx as usize].lock();
        let output_before_guard = output_wrapper
            .check_success_or_skip_status()?
            .before_materialization()?;

        let mut published = false;
        let mut module_ids_for_v2 = BTreeSet::new();
        for write in output_before_guard.module_write_set().values() {
            published = true;
            if scheduler.is_v2() {
                module_ids_for_v2.insert(write.module_id().clone());
            }
            add_module_write_to_module_cache::<T>(
                write,
                txn_idx,
                runtime_environment,
                global_module_cache,
                versioned_cache.module_cache(),
            )?;
        }
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
        with_success_or_skip_rest!(
            self,
            txn_idx,
            |t| Some(
                t.before_materialization()
                    .expect("Output must be set")
                    .delayed_field_change_set()
                    .into_keys()
            ),
            None
        )
    }

    pub(crate) fn reads_needing_delayed_field_exchange(
        &self,
        txn_idx: TxnIndex,
    ) -> Vec<(T::Key, StateValueMetadata, TriompheArc<MoveTypeLayout>)> {
        with_success_or_skip_rest!(self, txn_idx, reads_needing_delayed_field_exchange, vec![])
            .expect("Output must be set")
    }

    pub(crate) fn group_reads_needing_delayed_field_exchange(
        &self,
        txn_idx: TxnIndex,
    ) -> Vec<(T::Key, StateValueMetadata)> {
        with_success_or_skip_rest!(
            self,
            txn_idx,
            group_reads_needing_delayed_field_exchange,
            vec![]
        )
        .expect("Output must be set")
    }

    pub(crate) fn aggregator_v1_delta_keys(
        &self,
        txn_idx: TxnIndex,
    ) -> Option<impl Iterator<Item = T::Key>> {
        with_success_or_skip_rest!(
            self,
            txn_idx,
            |t| Some(
                t.before_materialization()
                    .expect("Output must be set")
                    .aggregator_v1_delta_set()
                    .into_keys()
            ),
            None
        )
    }

    pub(crate) fn resource_group_metadata_ops(&self, txn_idx: TxnIndex) -> Vec<(T::Key, T::Value)> {
        with_success_or_skip_rest!(self, txn_idx, resource_group_metadata_ops, vec![])
            .expect("Output must be set")
    }

    pub(crate) fn events(
        &self,
        txn_idx: TxnIndex,
    ) -> Box<dyn Iterator<Item = (T::Event, Option<MoveTypeLayout>)>> {
        with_success_or_skip_rest!(
            self,
            txn_idx,
            |t| Box::new(
                t.before_materialization()
                    .expect("Output must be set")
                    .get_events()
                    .into_iter()
            ),
            Box::new(empty())
        )
    }

    pub(crate) fn resource_write_set(
        &self,
        txn_idx: TxnIndex,
    ) -> Result<
        HashMap<T::Key, (TriompheArc<T::Value>, Option<TriompheArc<MoveTypeLayout>>)>,
        PanicError,
    > {
        with_success_or_skip_rest!(self, txn_idx, resource_write_set, HashMap::new())
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
        with_success_or_skip_rest!(
            self,
            txn_idx,
            |mut t| t.incorporate_materialized_txn_output(
                delta_writes,
                patched_resource_write_set,
                patched_events
            ),
            Ok(())
        )
    }

    // Must be executed after parallel execution is done, grabs outputs.
    pub(crate) fn take_output(&self, txn_idx: TxnIndex) -> Result<O, PanicError> {
        self.output_wrappers[txn_idx as usize].lock().take_output()
    }
}
