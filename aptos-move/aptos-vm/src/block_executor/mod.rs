// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

pub(crate) mod vm_wrapper;

use crate::counters::{BLOCK_EXECUTOR_CONCURRENCY, BLOCK_EXECUTOR_EXECUTE_BLOCK_SECONDS};
use aptos_aggregator::{
    delayed_change::DelayedChange, delta_change_set::DeltaOp, resolver::TAggregatorV1View,
};
use aptos_block_executor::{
    code_cache_global_manager::AptosModuleCacheManager,
    errors::BlockExecutionError,
    executor::BlockExecutor,
    explicit_sync_wrapper::ExplicitSyncWrapper,
    task::{ExecutorTask, TransactionOutput as BlockExecutorTransactionOutput},
    txn_commit_hook::TransactionCommitHook,
    txn_provider::TxnProvider,
    types::InputOutputKey,
};
use aptos_types::{
    block_executor::{
        config::BlockExecutorConfig, transaction_slice_metadata::TransactionSliceMetadata,
    },
    contract_event::ContractEvent,
    error::{code_invariant_error, PanicError},
    fee_statement::FeeStatement,
    state_store::{state_key::StateKey, state_value::StateValueMetadata, StateView, StateViewId},
    transaction::{
        signature_verified_transaction::SignatureVerifiedTransaction, BlockOutput,
        TransactionOutput, TransactionStatus,
    },
    write_set::WriteOp,
};
use aptos_vm_logging::{flush_speculative_logs, init_speculative_logs};
use aptos_vm_types::{
    abstract_write_op::AbstractResourceWriteOp, module_write_set::ModuleWrite, output::VMOutput,
    resolver::ResourceGroupSize,
};
use move_core_types::{
    language_storage::StructTag,
    value::MoveTypeLayout,
    vm_status::{StatusCode, VMStatus},
};
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
use once_cell::sync::{Lazy, OnceCell};
use std::{
    collections::{BTreeMap, HashSet},
    marker::PhantomData,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use vm_wrapper::AptosExecutorTask;

static RAYON_EXEC_POOL: Lazy<Arc<rayon::ThreadPool>> = Lazy::new(|| {
    Arc::new(
        rayon::ThreadPoolBuilder::new()
            .num_threads(num_cpus::get())
            .thread_name(|index| format!("par_exec-{}", index))
            .build()
            .unwrap(),
    )
});

/// Output type wrapper used by block executor. VM output is stored first, then
/// transformed into TransactionOutput type that is returned.
#[derive(Debug)]
pub struct AptosTransactionOutput {
    // BlockSTM maintains an invariant that executions of the same txn (with different
    // incarnations) do not happen concurrently, at the end of which vm_output is
    // recorded. Multiple workers / threads can concurrently access the recorded
    // output (e.g. for validation and post-processing purposes), but these accesses
    // are read-only until incorporate_materialized_txn_output is called (or
    // set_txn_output_for_non_dynamic_change_set, which does not commonly happen).
    //
    // One notable exception is legacy_sequential_materialize_agg_v1, which modifies
    // vm_output but is only used in sequential execution.
    //
    // The most important invariant provided by the system is that calls accessing
    // vm_output fully complete before incorporate_materialized_txn_output is called,
    // and some methods that first access committed output and (if unset) then vm_output,
    // are only done so to support both sequential and parallel execution flows
    // (in some cases sequential materializes earlier).
    // TODO(BlockSTMv2): unify the patterns across sequential and parallel execution,
    // and enforce invariants w. PanicError.
    vm_output: ExplicitSyncWrapper<Option<VMOutput>>,
    committed_output: OnceCell<TransactionOutput>,

    // For defensive purposes, since vm_output is not under a lock, if there is a bug
    // and a reading interface that is supposed to always finish before materialization
    // calls that take vm_output (and store committed_output), we can use this flag to
    // make sure there is no data race w. read. We achieve this by setting the flag
    // at the beginning of the materialization APIs and checking at the end of the
    // read-only interfaces.
    vm_output_taken: AtomicBool,
}

impl AptosTransactionOutput {
    pub fn new(output: VMOutput) -> Self {
        Self {
            vm_output: ExplicitSyncWrapper::new(Some(output)),
            committed_output: OnceCell::new(),
            vm_output_taken: AtomicBool::new(false),
        }
    }

    /// Helper method to get vm_output using fence_and_dereference.
    /// TODO: change all callers to use PanicError, currently unwrapping.
    fn get_vm_output(&self) -> Result<&VMOutput, PanicError> {
        self.vm_output
            .fence_and_dereference()
            .as_ref()
            .ok_or_else(|| code_invariant_error("VM output must be set"))
    }

    pub(crate) fn committed_output(&self) -> &TransactionOutput {
        self.committed_output.get().unwrap()
    }

    fn take_output(mut self) -> TransactionOutput {
        match self.committed_output.take() {
            Some(output) => output,
            // TODO: revisit whether we should always get it via committed, or o.w. create a
            // dedicated API without creating empty data structures.
            // This is currently used because we do not commit skip_output() transactions.
            None => self
                .vm_output
                .acquire()
                .dereference_mut()
                .take()
                .expect("Output must be set")
                .into_transaction_output()
                .expect("Transaction output is not already materialized"),
        }
    }
}

impl BlockExecutorTransactionOutput for AptosTransactionOutput {
    type Txn = SignatureVerifiedTransaction;

    /// Execution output for transactions that comes after SkipRest signal or when there was a
    /// problem creating the output (e.g. group serialization issue).
    fn skip_output() -> Self {
        Self::new(VMOutput::empty_with_status(TransactionStatus::Retry))
    }

    fn discard_output(discard_code: StatusCode) -> Self {
        Self::new(VMOutput::empty_with_status(TransactionStatus::Discard(
            discard_code,
        )))
    }

    fn incorporate_materialized_txn_output(
        &self,
        aggregator_v1_writes: Vec<(StateKey, WriteOp)>,
        materialized_resource_write_set: Vec<(StateKey, WriteOp)>,
        materialized_events: Vec<ContractEvent>,
    ) -> Result<(), PanicError> {
        self.vm_output_taken.store(true, Ordering::Relaxed);
        assert!(
            self.committed_output
                .set(
                    self.vm_output
                        .acquire()
                        .dereference_mut()
                        .take()
                        .expect("Output must be set to incorporate materialized data")
                        .into_transaction_output_with_materialized_write_set(
                            aggregator_v1_writes,
                            materialized_resource_write_set,
                            materialized_events,
                        )?,
                )
                .is_ok(),
            "Could not combine VMOutput with the materialized resource and event data"
        );
        Ok(())
    }

    fn set_txn_output_for_non_dynamic_change_set(&self) {
        self.vm_output_taken.store(true, Ordering::Relaxed);
        assert!(
            self.committed_output
                .set(
                    self.vm_output
                        .acquire()
                        .dereference_mut()
                        .take()
                        .expect("Output must be set to incorporate materialized data")
                        .into_transaction_output()
                        .expect("We should be able to always convert to transaction output"),
                )
                .is_ok(),
            "Could not combine VMOutput with the materialized resource and event data"
        );
    }

    // This legacy method is only used in sequential execution, which is why modifying the
    // vm_output is safe.
    // TODO: convert to the same flow as parallel execution and remove this method.
    fn legacy_sequential_materialize_agg_v1(
        &self,
        view: &impl TAggregatorV1View<Identifier = StateKey>,
    ) {
        self.vm_output
            .dereference_mut()
            .as_mut()
            .expect("Output must be set")
            .try_materialize(view)
            .expect("Delta materialization failed");

        assert!(
            !self.vm_output_taken.load(Ordering::Relaxed),
            "Must complete before vm output is taken"
        );
    }

    /// TODO: Make these methods only rely on vm_output or committed_output, so we can
    /// program defensively against buggy access patterns.

    /// Returns the fee statement of the transaction.
    fn fee_statement(&self) -> FeeStatement {
        if let Some(committed_output) = self.committed_output.get() {
            if let Ok(Some(fee_statement)) = committed_output.try_extract_fee_statement() {
                return fee_statement;
            }
            return FeeStatement::zero();
        }
        *self.get_vm_output().unwrap().fee_statement()
    }

    /// Returns true iff the TransactionsStatus is Retry.
    fn is_retry(&self) -> bool {
        if let Some(committed_output) = self.committed_output.get() {
            committed_output.status().is_retry()
        } else {
            self.get_vm_output().unwrap().status().is_retry()
        }
    }

    /// Returns true iff it has a new epoch event.
    fn has_new_epoch_event(&self) -> bool {
        self.committed_output
            .get()
            .expect("Must call after commit.")
            .has_new_epoch_event()
    }

    /// Returns true iff the execution status is Keep(Success).
    fn is_success(&self) -> bool {
        if let Some(committed_output) = self.committed_output.get() {
            committed_output
                .status()
                .as_kept_status()
                .map_or(false, |status| status.is_success())
        } else {
            self.get_vm_output()
                .unwrap()
                .status()
                .as_kept_status()
                .map_or(false, |status| status.is_success())
        }
    }

    /// !!! [CAUTION] !!!: methods below should never be called after or concurrently with
    /// incorporating materialized output, as materialization consumes vm_output.
    /// This is additionally enforced by vm_output_taken flag.
    /// TODO: get rid of cloning data-structures as much as possible, use PanicError.
    fn resource_group_write_set(
        &self,
    ) -> impl Iterator<
        Item = (
            StateKey,
            WriteOp,
            ResourceGroupSize,
            BTreeMap<StructTag, (WriteOp, Option<Arc<MoveTypeLayout>>)>,
        ),
    > {
        let ret = self
            .get_vm_output()
            .unwrap()
            .resource_write_set()
            .iter()
            .flat_map(|(key, write)| {
                // TODO: consider storing these separately so we don't have to always
                // transfer all writes.
                if let AbstractResourceWriteOp::WriteResourceGroup(group_write) = write {
                    Some((
                        key.clone(),
                        group_write.metadata_op().clone(),
                        group_write
                            .maybe_group_op_size()
                            .unwrap_or(ResourceGroupSize::zero_combined()),
                        group_write
                            .inner_ops()
                            .iter()
                            .map(|(tag, (op, maybe_layout))| {
                                (tag.clone(), (op.clone(), maybe_layout.clone()))
                            })
                            .collect(),
                    ))
                } else {
                    None
                }
            });

        assert!(
            !self.vm_output_taken.load(Ordering::Relaxed),
            "Must complete before vm output is taken"
        );

        ret
    }

    fn resource_group_key_and_tags_ref(
        &self,
    ) -> impl Iterator<Item = (&StateKey, HashSet<&StructTag>)> {
        let ret = self
            .get_vm_output()
            .unwrap()
            .resource_write_set()
            .iter()
            .flat_map(|(key, write)| {
                // TODO: consider storing these separately so we don't have to always
                // transfer all writes.
                if let AbstractResourceWriteOp::WriteResourceGroup(group_write) = write {
                    let tags = group_write.inner_ops().keys().collect();
                    Some((key, tags))
                } else {
                    None
                }
            });

        assert!(
            !self.vm_output_taken.load(Ordering::Relaxed),
            "Must complete before vm output is taken"
        );

        ret
    }

    /// More efficient implementation to avoid unnecessarily cloning inner_ops.
    fn resource_group_metadata_ops(&self) -> Vec<(StateKey, WriteOp)> {
        let ret = self
            .get_vm_output()
            .unwrap()
            .resource_write_set()
            .iter()
            .flat_map(|(key, write)| {
                if let AbstractResourceWriteOp::WriteResourceGroup(group_write) = write {
                    Some((key.clone(), group_write.metadata_op().clone()))
                } else {
                    None
                }
            })
            .collect();

        assert!(
            !self.vm_output_taken.load(Ordering::Relaxed),
            "Must complete before vm output is taken"
        );

        ret
    }

    /// Should never be called after incorporating materialized output, as that consumes vm_output.
    fn resource_write_set(&self) -> Vec<(StateKey, Arc<WriteOp>, Option<Arc<MoveTypeLayout>>)> {
        let ret = self
            .get_vm_output()
            .unwrap()
            .resource_write_set()
            .iter()
            .flat_map(|(key, write)| match write {
                AbstractResourceWriteOp::Write(write_op) => {
                    Some((key.clone(), Arc::new(write_op.clone()), None))
                },
                AbstractResourceWriteOp::WriteWithDelayedFields(write) => Some((
                    key.clone(),
                    Arc::new(write.write_op.clone()),
                    Some(write.layout.clone()),
                )),
                _ => None,
            })
            .collect();

        assert!(
            !self.vm_output_taken.load(Ordering::Relaxed),
            "Must complete before vm output is taken"
        );

        ret
    }

    /// Should never be called after incorporating materialized output, as that consumes vm_output.
    fn module_write_set(&self) -> Vec<ModuleWrite<WriteOp>> {
        let ret = self
            .get_vm_output()
            .unwrap()
            .module_write_set()
            .values()
            .cloned()
            .collect();

        assert!(
            !self.vm_output_taken.load(Ordering::Relaxed),
            "Must complete before vm output is taken"
        );

        ret
    }

    /// Should never be called after incorporating materialized output, as that consumes vm_output.
    fn aggregator_v1_write_set(&self) -> BTreeMap<StateKey, WriteOp> {
        let ret = self
            .get_vm_output()
            .unwrap()
            .aggregator_v1_write_set()
            .clone();

        assert!(
            !self.vm_output_taken.load(Ordering::Relaxed),
            "Must complete before vm output is taken"
        );

        ret
    }

    /// Should never be called after incorporating materialized output, as that consumes vm_output.
    fn aggregator_v1_delta_set(&self) -> Vec<(StateKey, DeltaOp)> {
        let ret = self
            .get_vm_output()
            .unwrap()
            .aggregator_v1_delta_set()
            .iter()
            .map(|(key, op)| (key.clone(), *op))
            .collect();

        assert!(
            !self.vm_output_taken.load(Ordering::Relaxed),
            "Must complete before vm output is taken"
        );

        ret
    }

    /// Should never be called after incorporating materialized output, as that consumes vm_output.
    fn delayed_field_change_set(&self) -> BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>> {
        let ret = self
            .get_vm_output()
            .unwrap()
            .delayed_field_change_set()
            .clone();

        assert!(
            !self.vm_output_taken.load(Ordering::Relaxed),
            "Must complete before vm output is taken"
        );

        ret
    }

    fn reads_needing_delayed_field_exchange(
        &self,
    ) -> Vec<(StateKey, StateValueMetadata, Arc<MoveTypeLayout>)> {
        let ret = self
            .get_vm_output()
            .unwrap()
            .resource_write_set()
            .iter()
            .flat_map(|(key, write)| {
                if let AbstractResourceWriteOp::InPlaceDelayedFieldChange(change) = write {
                    Some((key.clone(), change.metadata.clone(), change.layout.clone()))
                } else {
                    None
                }
            })
            .collect();

        assert!(
            !self.vm_output_taken.load(Ordering::Relaxed),
            "Must complete before vm output is taken"
        );

        ret
    }

    fn group_reads_needing_delayed_field_exchange(&self) -> Vec<(StateKey, StateValueMetadata)> {
        let ret = self
            .get_vm_output()
            .unwrap()
            .resource_write_set()
            .iter()
            .flat_map(|(key, write)| {
                if let AbstractResourceWriteOp::ResourceGroupInPlaceDelayedFieldChange(change) =
                    write
                {
                    Some((key.clone(), change.metadata.clone()))
                } else {
                    None
                }
            })
            .collect();

        assert!(
            !self.vm_output_taken.load(Ordering::Relaxed),
            "Must complete before vm output is taken"
        );

        ret
    }

    /// Should never be called after incorporating materialized output, as that consumes vm_output.
    fn get_events(&self) -> Vec<(ContractEvent, Option<MoveTypeLayout>)> {
        let ret = self.get_vm_output().unwrap().events().to_vec();

        assert!(
            !self.vm_output_taken.load(Ordering::Relaxed),
            "Must complete before vm output is taken"
        );

        ret
    }

    fn output_approx_size(&self) -> u64 {
        let ret = self.get_vm_output().unwrap().materialized_size();

        assert!(
            !self.vm_output_taken.load(Ordering::Relaxed),
            "Must complete before vm output is taken"
        );

        ret
    }

    fn get_write_summary(&self) -> HashSet<InputOutputKey<StateKey, StructTag>> {
        let output = self.get_vm_output().unwrap();
        let mut writes = HashSet::new();

        for (state_key, write) in output.resource_write_set() {
            match write {
                AbstractResourceWriteOp::Write(_)
                | AbstractResourceWriteOp::WriteWithDelayedFields(_) => {
                    writes.insert(InputOutputKey::Resource(state_key.clone()));
                },
                AbstractResourceWriteOp::WriteResourceGroup(write) => {
                    for tag in write.inner_ops().keys() {
                        writes.insert(InputOutputKey::Group(state_key.clone(), tag.clone()));
                    }
                },
                AbstractResourceWriteOp::InPlaceDelayedFieldChange(_)
                | AbstractResourceWriteOp::ResourceGroupInPlaceDelayedFieldChange(_) => {
                    // No conflicts on resources from in-place delayed field changes.
                    // Delayed fields conflicts themselves are handled via
                    // delayed_field_change_set below.
                },
            }
        }

        for identifier in output.delayed_field_change_set().keys() {
            writes.insert(InputOutputKey::DelayedField(*identifier));
        }

        assert!(
            !self.vm_output_taken.load(Ordering::Relaxed),
            "Must complete before vm output is taken"
        );

        writes
    }
}

pub struct AptosBlockExecutorWrapper<
    E: ExecutorTask<
        Txn = SignatureVerifiedTransaction,
        Error = VMStatus,
        Output = AptosTransactionOutput,
    >,
> {
    _phantom: PhantomData<E>,
}

impl<
        E: ExecutorTask<
            Txn = SignatureVerifiedTransaction,
            Error = VMStatus,
            Output = AptosTransactionOutput,
        >,
    > AptosBlockExecutorWrapper<E>
{
    pub fn execute_block_on_thread_pool<
        S: StateView + Sync,
        L: TransactionCommitHook<Output = AptosTransactionOutput>,
        TP: TxnProvider<SignatureVerifiedTransaction> + Sync,
    >(
        executor_thread_pool: Arc<rayon::ThreadPool>,
        signature_verified_block: &TP,
        state_view: &S,
        module_cache_manager: &AptosModuleCacheManager,
        config: BlockExecutorConfig,
        transaction_slice_metadata: TransactionSliceMetadata,
        transaction_commit_listener: Option<L>,
    ) -> Result<BlockOutput<TransactionOutput>, VMStatus> {
        let _timer = BLOCK_EXECUTOR_EXECUTE_BLOCK_SECONDS.start_timer();

        let num_txns = signature_verified_block.num_txns();
        if state_view.id() != StateViewId::Miscellaneous {
            // Speculation is disabled in Miscellaneous context, which is used by testing and
            // can even lead to concurrent execute_block invocations, leading to errors on flush.
            init_speculative_logs(num_txns);
        }

        BLOCK_EXECUTOR_CONCURRENCY.set(config.local.concurrency_level as i64);

        let mut module_cache_manager_guard = module_cache_manager.try_lock(
            &state_view,
            &config.local.module_cache_config,
            transaction_slice_metadata,
        )?;

        let executor = BlockExecutor::<SignatureVerifiedTransaction, E, S, L, TP>::new(
            config,
            executor_thread_pool,
            transaction_commit_listener,
        );

        let ret = executor.execute_block(
            signature_verified_block,
            state_view,
            &transaction_slice_metadata,
            &mut module_cache_manager_guard,
        );
        match ret {
            Ok(block_output) => {
                let (transaction_outputs, block_epilogue_txn) = block_output.into_inner();
                let output_vec: Vec<_> = transaction_outputs
                    .into_iter()
                    .map(|output| output.take_output())
                    .collect();

                // Flush the speculative logs of the committed transactions.
                let pos = output_vec.partition_point(|o| !o.status().is_retry());

                if state_view.id() != StateViewId::Miscellaneous {
                    // Speculation is disabled in Miscellaneous context, which is used by testing and
                    // can even lead to concurrent execute_block invocations, leading to errors on flush.
                    flush_speculative_logs(pos);
                }

                Ok(BlockOutput::new(output_vec, block_epilogue_txn))
            },
            Err(BlockExecutionError::FatalBlockExecutorError(PanicError::CodeInvariantError(
                err_msg,
            ))) => Err(VMStatus::Error {
                status_code: StatusCode::DELAYED_FIELD_OR_BLOCKSTM_CODE_INVARIANT_ERROR,
                sub_status: None,
                message: Some(err_msg),
            }),
            Err(BlockExecutionError::FatalVMError(err)) => Err(err),
        }
    }

    /// Uses shared thread pool to execute blocks.
    pub(crate) fn execute_block<
        S: StateView + Sync,
        L: TransactionCommitHook<Output = AptosTransactionOutput>,
        TP: TxnProvider<SignatureVerifiedTransaction> + Sync,
    >(
        signature_verified_block: &TP,
        state_view: &S,
        module_cache_manager: &AptosModuleCacheManager,
        config: BlockExecutorConfig,
        transaction_slice_metadata: TransactionSliceMetadata,
        transaction_commit_listener: Option<L>,
    ) -> Result<BlockOutput<TransactionOutput>, VMStatus> {
        Self::execute_block_on_thread_pool::<S, L, TP>(
            Arc::clone(&RAYON_EXEC_POOL),
            signature_verified_block,
            state_view,
            module_cache_manager,
            config,
            transaction_slice_metadata,
            transaction_commit_listener,
        )
    }
}

// Same as AptosBlockExecutorWrapper with AptosExecutorTask
pub type AptosVMBlockExecutorWrapper = AptosBlockExecutorWrapper<AptosExecutorTask>;
