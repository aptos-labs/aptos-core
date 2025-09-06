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
    error::PanicError,
    fee_statement::FeeStatement,
    state_store::{state_key::StateKey, state_value::StateValueMetadata, StateView, StateViewId},
    transaction::{
        signature_verified_transaction::SignatureVerifiedTransaction, AuxiliaryInfo, BlockOutput,
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
use parking_lot::{RwLock, RwLockReadGuard};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    marker::PhantomData,
    sync::Arc,
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
    vm_output: RwLock<Option<VMOutput>>,
    committed_output: OnceCell<TransactionOutput>,
}

impl AptosTransactionOutput {
    pub fn new(output: VMOutput) -> Self {
        Self {
            vm_output: RwLock::new(Some(output)),
            committed_output: OnceCell::new(),
        }
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
                .write()
                .take()
                .expect("Output must be set")
                .into_transaction_output()
                .expect("Transaction output is not already materialized"),
        }
    }
}

/// Guard wrapper that provides AsRef access to module write set without cloning.
pub struct ModuleWriteSetGuard<'a> {
    guard: RwLockReadGuard<'a, Option<VMOutput>>,
}

impl<'a> ModuleWriteSetGuard<'a> {
    fn new(rwlock: &'a RwLock<Option<VMOutput>>) -> Self {
        Self {
            guard: rwlock.read(),
        }
    }
}

impl<'a> AsRef<BTreeMap<StateKey, ModuleWrite<WriteOp>>> for ModuleWriteSetGuard<'a> {
    fn as_ref(&self) -> &BTreeMap<StateKey, ModuleWrite<WriteOp>> {
        self.guard
            .as_ref()
            .expect("Output must be set to get module writes")
            .module_write_set()
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

    // TODO: get rid of the cloning data-structures in the following APIs.
    // This can be accomplished either by providing callbacks or by providing AsRef access to
    // a guard, which for different resources write types would require changing the
    // data-structures in the VMOutput / ChangeSet.

    // Interfaces below should never be called after incorporating materialized output,
    // as that consumes vm_output.

    fn resource_group_write_set(
        &self,
    ) -> HashMap<
        StateKey,
        (
            WriteOp,
            ResourceGroupSize,
            BTreeMap<StructTag, (WriteOp, Option<Arc<MoveTypeLayout>>)>,
        ),
    > {
        self.vm_output
            .read()
            .as_ref()
            .expect("Output must be set to get resource group writes")
            .resource_write_set()
            .iter()
            .flat_map(|(key, write)| {
                if let AbstractResourceWriteOp::WriteResourceGroup(group_write) = write {
                    Some((
                        key.clone(),
                        (
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
                        ),
                    ))
                } else {
                    None
                }
            })
            .collect()
    }

    fn for_each_resource_group_key_and_tags<F>(&self, mut callback: F) -> Result<(), PanicError>
    where
        F: FnMut(&StateKey, HashSet<&StructTag>) -> Result<(), PanicError>,
    {
        for (key, tags) in self
            .vm_output
            .read()
            .as_ref()
            .expect("Output must be set to get resource group writes")
            .resource_write_set()
            .iter()
            .flat_map(|(key, write)| {
                if let AbstractResourceWriteOp::WriteResourceGroup(group_write) = write {
                    Some((key, group_write.inner_ops().keys().collect()))
                } else {
                    None
                }
            })
        {
            callback(key, tags)?;
        }

        Ok(())
    }

    /// More efficient implementation to avoid unnecessarily cloning inner_ops.
    fn resource_group_metadata_ops(&self) -> Vec<(StateKey, WriteOp)> {
        self.vm_output
            .read()
            .as_ref()
            .expect("Output must be set to get metadata ops")
            .resource_write_set()
            .iter()
            .flat_map(|(key, write)| {
                if let AbstractResourceWriteOp::WriteResourceGroup(group_write) = write {
                    Some((key.clone(), group_write.metadata_op().clone()))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Should never be called after incorporating materialized output, as that consumes vm_output.
    fn resource_write_set(&self) -> Vec<(StateKey, Arc<WriteOp>, Option<Arc<MoveTypeLayout>>)> {
        self.vm_output
            .read()
            .as_ref()
            .expect("Output must be set to get resource writes")
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
            .collect()
    }

    /// Should never be called after incorporating materialized output, as that consumes vm_output.
    fn module_write_set(&self) -> impl AsRef<BTreeMap<StateKey, ModuleWrite<WriteOp>>> + '_ {
        ModuleWriteSetGuard::new(&self.vm_output)
    }

    /// Should never be called after incorporating materialized output, as that consumes vm_output.
    fn aggregator_v1_write_set(&self) -> BTreeMap<StateKey, WriteOp> {
        self.vm_output
            .read()
            .as_ref()
            .expect("Output must be set to get aggregator V1 writes")
            .aggregator_v1_write_set()
            .clone()
    }

    /// Should never be called after incorporating materialized output, as that consumes vm_output.
    fn aggregator_v1_delta_set(&self) -> BTreeMap<StateKey, DeltaOp> {
        self.vm_output
            .read()
            .as_ref()
            .expect("Output must be set to get deltas")
            .aggregator_v1_delta_set()
            .clone()
    }

    /// Should never be called after incorporating materialized output, as that consumes vm_output.
    fn delayed_field_change_set(&self) -> BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>> {
        self.vm_output
            .read()
            .as_ref()
            .expect("Output must be set to get aggregator change set")
            .delayed_field_change_set()
            .clone()
    }

    fn reads_needing_delayed_field_exchange(
        &self,
    ) -> Vec<(StateKey, StateValueMetadata, Arc<MoveTypeLayout>)> {
        self.vm_output
            .read()
            .as_ref()
            .expect("Output to be set to get reads")
            .resource_write_set()
            .iter()
            .flat_map(|(key, write)| {
                if let AbstractResourceWriteOp::InPlaceDelayedFieldChange(change) = write {
                    Some((key.clone(), change.metadata.clone(), change.layout.clone()))
                } else {
                    None
                }
            })
            .collect()
    }

    fn group_reads_needing_delayed_field_exchange(&self) -> Vec<(StateKey, StateValueMetadata)> {
        self.vm_output
            .read()
            .as_ref()
            .expect("Output to be set to get reads")
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
            .collect()
    }

    /// Should never be called after incorporating materialized output, as that consumes vm_output.
    fn get_events(&self) -> Vec<(ContractEvent, Option<MoveTypeLayout>)> {
        self.vm_output
            .read()
            .as_ref()
            .expect("Output must be set to get events")
            .events()
            .to_vec()
    }

    fn incorporate_materialized_txn_output(
        &self,
        aggregator_v1_writes: Vec<(StateKey, WriteOp)>,
        materialized_resource_write_set: Vec<(StateKey, WriteOp)>,
        materialized_events: Vec<ContractEvent>,
    ) -> Result<(), PanicError> {
        assert!(
            self.committed_output
                .set(
                    self.vm_output
                        .write()
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
        assert!(
            self.committed_output
                .set(
                    self.vm_output
                        .write()
                        .take()
                        .expect("Output must be set to incorporate materialized data")
                        .into_transaction_output()
                        .expect("We should be able to always convert to transaction output"),
                )
                .is_ok(),
            "Could not combine VMOutput with the materialized resource and event data"
        );
    }

    /// Returns the fee statement of the transaction.
    ///
    /// TODO(gelash): Consider defensive access pattern to committed_output / vm_output.
    fn fee_statement(&self) -> FeeStatement {
        if let Some(committed_output) = self.committed_output.get() {
            if let Ok(Some(fee_statement)) = committed_output.try_extract_fee_statement() {
                return fee_statement;
            }
            return FeeStatement::zero();
        }
        *self
            .vm_output
            .read()
            .as_ref()
            .expect("Output to be set to get fee statement")
            .fee_statement()
    }

    /// Returns true iff the TransactionsStatus is Retry.
    fn is_retry(&self) -> bool {
        if let Some(committed_output) = self.committed_output.get() {
            committed_output.status().is_retry()
        } else {
            self.vm_output
                .read()
                .as_ref()
                .expect("Either vm_output or committed_output must exist.")
                .status()
                .is_retry()
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
            self.vm_output
                .read()
                .as_ref()
                .expect("Either vm_output or committed_output must exist.")
                .status()
                .as_kept_status()
                .map_or(false, |status| status.is_success())
        }
    }

    fn output_approx_size(&self) -> u64 {
        let vm_output = self.vm_output.read();
        vm_output
            .as_ref()
            .expect("Output to be set to get approximate size")
            .materialized_size()
    }

    fn get_write_summary(&self) -> HashSet<InputOutputKey<StateKey, StructTag>> {
        let vm_output = self.vm_output.read();
        let output = vm_output
            .as_ref()
            .expect("Output to be set to get write summary");

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

        writes
    }

    // `read_keys` include all keys read, not just read-only ones.
    fn set_read_set(&self, read_keys: BTreeSet<StateKey>) {
        assert!(self.committed_output.get().is_none());

        self.vm_output
            .write()
            .as_mut()
            .unwrap()
            .set_read_set(read_keys);
    }

    fn get_storage_keys_written(&self) -> BTreeSet<StateKey> {
        if let Some(output) = self.committed_output.get() {
            return output
                .write_set()
                .write_op_iter()
                .map(|(key, _op)| key.clone())
                .collect();
        }

        let locked = self.vm_output.read();
        let vm_output = locked.as_ref().expect("Output needs to be set");

        let mut keys = BTreeSet::new();
        keys.extend(vm_output.resource_write_set().keys().cloned());
        keys.extend(vm_output.aggregator_v1_write_set().keys().cloned());
        keys.extend(vm_output.module_write_set().keys().cloned());
        keys
    }

    // Legacy interfaces, which means there are alternative, more efficient ways used in
    // BlockSTMv2. For now we do get the benefits of comparing different implementations.
    // TODO: consider adjusting sequential execution and BlockSTMv1 to use the superior
    // patterns and remove these legacy interfaces (needs to be done carefully).

    // Internally clones and also allocates a new vector. Used for BlockSTMv1 only.
    fn legacy_v1_resource_group_tags(&self) -> Vec<(StateKey, HashSet<StructTag>)> {
        self.vm_output
            .read()
            .as_ref()
            .expect("Output must be set to get metadata ops")
            .resource_write_set()
            .iter()
            .flat_map(|(key, write)| {
                if let AbstractResourceWriteOp::WriteResourceGroup(group_write) = write {
                    Some((
                        key.clone(),
                        group_write.inner_ops().keys().cloned().collect(),
                    ))
                } else {
                    None
                }
            })
            .collect()
    }

    // Used only by the sequential execution.
    fn legacy_sequential_materialize_agg_v1(
        &self,
        view: &impl TAggregatorV1View<Identifier = StateKey>,
    ) {
        self.vm_output
            .write()
            .as_mut()
            .expect("Output must be set to incorporate materialized data")
            .try_materialize(view)
            .expect("Delta materialization failed");
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
            AuxiliaryInfo = AuxiliaryInfo,
            Error = VMStatus,
            Output = AptosTransactionOutput,
        >,
    > AptosBlockExecutorWrapper<E>
{
    pub fn execute_block_on_thread_pool<
        S: StateView + Sync,
        L: TransactionCommitHook<Output = AptosTransactionOutput>,
        TP: TxnProvider<SignatureVerifiedTransaction, AuxiliaryInfo> + Sync,
    >(
        executor_thread_pool: Arc<rayon::ThreadPool>,
        signature_verified_block: &TP,
        state_view: &S,
        module_cache_manager: &AptosModuleCacheManager,
        config: BlockExecutorConfig,
        transaction_slice_metadata: TransactionSliceMetadata,
        transaction_commit_listener: Option<L>,
    ) -> Result<BlockOutput<SignatureVerifiedTransaction, TransactionOutput>, VMStatus> {
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

        let executor =
            BlockExecutor::<SignatureVerifiedTransaction, E, S, L, TP, AuxiliaryInfo>::new(
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
        TP: TxnProvider<SignatureVerifiedTransaction, AuxiliaryInfo> + Sync,
    >(
        signature_verified_block: &TP,
        state_view: &S,
        module_cache_manager: &AptosModuleCacheManager,
        config: BlockExecutorConfig,
        transaction_slice_metadata: TransactionSliceMetadata,
        transaction_commit_listener: Option<L>,
    ) -> Result<BlockOutput<SignatureVerifiedTransaction, TransactionOutput>, VMStatus> {
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
