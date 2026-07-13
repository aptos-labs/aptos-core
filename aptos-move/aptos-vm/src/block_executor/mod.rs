// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

pub(crate) mod vm_wrapper;

use crate::counters::{BLOCK_EXECUTOR_CONCURRENCY, BLOCK_EXECUTOR_EXECUTE_BLOCK_SECONDS};
use aptos_aggregator::delayed_change::DelayedChange;
use aptos_block_executor::{
    code_cache_global_manager::AptosModuleCacheManager,
    errors::BlockExecutionError,
    executor::BlockExecutor,
    task::{
        BeforeMaterializationOutput, ExecutorTask,
        TransactionOutput as BlockExecutorTransactionOutput,
    },
    txn_commit_hook::TransactionCommitHook,
    txn_provider::TxnProvider,
    types::InputOutputKey,
};
use aptos_types::{
    block_executor::{
        config::BlockExecutorConfig, transaction_slice_metadata::TransactionSliceMetadata,
        value::ValueWithLayout,
    },
    contract_event::ContractEvent,
    error::{code_invariant_error, PanicError},
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
    abstract_write_op::AbstractResourceWriteOp,
    module_write_set::ModuleWrite,
    output::{UnorderedReadSet, VMOutput},
    resolver::ResourceGroupSize,
};
use move_core_types::{
    language_storage::StructTag,
    value::MoveTypeLayout,
    vm_status::{StatusCode, VMStatus},
};
use move_vm_runtime::execution_tracing::Trace;
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    marker::PhantomData,
};
use triomphe::Arc as TriompheArc;
use vm_wrapper::AptosExecutorTask;

/// Output type wrapper used by block executor. VM output is stored, and
/// transformed into TransactionOutput type on materialization.
#[derive(Debug)]
pub struct AptosTransactionOutput {
    vm_output: Option<VMOutput>,
    /// State keys read by the VM during the execution (incarnation) that produced this output.
    ///
    /// TODO(HotState): also consider recording the read kind (exists/metadata/value) and the
    /// observed slot hotness, so the promotion policy can filter on them.
    read_set: UnorderedReadSet,
}

impl AptosTransactionOutput {
    pub fn new(output: VMOutput) -> Self {
        Self::new_with_read_set(output, UnorderedReadSet::default())
    }

    pub fn new_with_read_set(output: VMOutput, read_set: UnorderedReadSet) -> Self {
        Self {
            vm_output: Some(output),
            read_set,
        }
    }
}

/// Before materialization guard wrapper that holds a read lock.
pub struct BeforeMaterializationGuard<'a> {
    guard: &'a VMOutput,
    read_set: &'a UnorderedReadSet,
}

impl BeforeMaterializationOutput<SignatureVerifiedTransaction> for BeforeMaterializationGuard<'_> {
    fn fee_statement(&self) -> FeeStatement {
        *self.guard.fee_statement()
    }

    fn has_new_epoch_event(&self) -> bool {
        self.guard
            .events()
            .iter()
            .map(|(event, _)| event)
            .any(ContractEvent::is_new_epoch_event)
    }

    fn output_approx_size(&self) -> u64 {
        self.guard.materialized_size()
    }

    fn get_write_summary(&self) -> HashSet<InputOutputKey<StateKey, StructTag>> {
        let mut writes = HashSet::new();

        for (state_key, write) in self.guard.resource_write_set() {
            match write {
                AbstractResourceWriteOp::Write(..)
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

        for identifier in self.guard.delayed_field_change_set().keys() {
            writes.insert(InputOutputKey::DelayedField(*identifier));
        }

        writes
    }

    fn storage_keys_read(&self) -> impl Iterator<Item = &StateKey> {
        self.read_set.iter()
    }

    fn storage_keys_written(&self) -> impl Iterator<Item = &StateKey> {
        // Every key receiving a value write becomes hot via the write itself, so the hot
        // state accumulator must treat it as written and not promote it separately. Unlike
        // get_write_summary (conflict detection), this includes in-place delayed field
        // rewrites and module writes. The accumulator dedups, so the chained keys need not
        // be unique.
        self.guard
            .resource_write_set()
            .keys()
            .chain(self.guard.module_write_set().keys())
    }

    // TODO: get rid of the cloning data-structures in the following APIs.
    fn resource_group_write_set(
        &self,
    ) -> HashMap<
        StateKey,
        (
            ValueWithLayout<WriteOp>,
            ResourceGroupSize,
            BTreeMap<StructTag, ValueWithLayout<WriteOp>>,
        ),
    > {
        self.guard
            .resource_write_set()
            .iter()
            .flat_map(|(key, write)| {
                if let AbstractResourceWriteOp::WriteResourceGroup(group_write) = write {
                    Some((
                        key.clone(),
                        (
                            ValueWithLayout::Exchanged(
                                TriompheArc::new(group_write.metadata_op().clone()),
                                None,
                            ),
                            group_write
                                .maybe_group_op_size()
                                .unwrap_or(ResourceGroupSize::zero_combined()),
                            group_write
                                .inner_ops()
                                .iter()
                                .map(|(tag, (op, maybe_layout))| {
                                    (
                                        tag.clone(),
                                        ValueWithLayout::Exchanged(
                                            TriompheArc::new(op.clone()),
                                            maybe_layout.clone(),
                                        ),
                                    )
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

    fn for_each_resource_key(
        &self,
        callback: &mut dyn FnMut(&StateKey) -> Result<(), PanicError>,
    ) -> Result<(), PanicError> {
        for key in self
            .guard
            .resource_write_set()
            .iter()
            .flat_map(|(key, write)| match write {
                AbstractResourceWriteOp::Write(..)
                | AbstractResourceWriteOp::WriteWithDelayedFields(_) => Some(key),
                _ => None,
            })
        {
            callback(key)?;
        }
        Ok(())
    }

    fn for_each_resource_group_key_and_tags(
        &self,
        callback: &mut dyn FnMut(&StateKey, HashSet<&StructTag>) -> Result<(), PanicError>,
    ) -> Result<(), PanicError> {
        for (key, tags) in self
            .guard
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
        self.guard
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

    fn resource_write_set(&self) -> HashMap<StateKey, ValueWithLayout<WriteOp>> {
        self.guard
            .resource_write_set()
            .iter()
            .flat_map(|(key, write)| match write {
                AbstractResourceWriteOp::Write(write_op, _) => Some((
                    key.clone(),
                    ValueWithLayout::Exchanged(TriompheArc::new(write_op.clone()), None),
                )),
                AbstractResourceWriteOp::WriteWithDelayedFields(write) => Some((
                    key.clone(),
                    ValueWithLayout::Exchanged(
                        TriompheArc::new(write.write_op.clone()),
                        Some(write.layout.clone()),
                    ),
                )),
                _ => None,
            })
            .collect()
    }

    /// Should never be called after incorporating materialized output, as that consumes vm_output.
    fn module_write_set(&self) -> &BTreeMap<StateKey, ModuleWrite<WriteOp>> {
        self.guard.module_write_set()
    }

    /// Should never be called after incorporating materialized output, as that consumes vm_output.
    fn delayed_field_change_set(&self) -> BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>> {
        self.guard.delayed_field_change_set().clone()
    }

    fn reads_needing_delayed_field_exchange(
        &self,
    ) -> Vec<(StateKey, StateValueMetadata, TriompheArc<MoveTypeLayout>)> {
        self.guard
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
        self.guard
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
        self.guard.events().to_vec()
    }

    // For legacy interfaces, there are more efficient alternatives in BlockSTMv2.
    // For now we do get the benefits of comparing different implementations.
    // TODO: consider adjusting sequential execution and BlockSTMv1 to use the superior
    // patterns and remove these legacy interfaces (needs to be done carefully).
    //
    // Internally clones and also allocates a new vector. Used for BlockSTMv1 only.
    fn legacy_v1_resource_group_tags(&self) -> Vec<(StateKey, HashSet<StructTag>)> {
        self.guard
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
}

impl BlockExecutorTransactionOutput for AptosTransactionOutput {
    type BeforeMaterializationGuard<'a> = BeforeMaterializationGuard<'a>;
    type CommittedOutput = TransactionOutput;
    type Txn = SignatureVerifiedTransaction;

    /// Execution output for transactions that comes after SkipRest signal or when there was a
    /// problem creating the output (e.g. group serialization issue).
    fn skip_output() -> Self {
        Self::new(VMOutput::empty_with_status(TransactionStatus::Retry))
    }

    fn before_materialization<'a>(&'a self) -> Result<BeforeMaterializationGuard<'a>, PanicError> {
        Ok(BeforeMaterializationGuard {
            guard: self
                .vm_output
                .as_ref()
                .ok_or_else(|| code_invariant_error("Output must be set but not materialized"))?,
            read_set: &self.read_set,
        })
    }

    fn incorporate_materialized_txn_output(
        &mut self,
        materialized_resource_write_set: Vec<(StateKey, WriteOp)>,
        materialized_events: Vec<ContractEvent>,
    ) -> Result<(Self::CommittedOutput, Trace), PanicError> {
        // Before creating the output, extract the trace for replay.
        let mut vm_output = self
            .vm_output
            .take()
            .expect("Output must be set to incorporate materialized data");
        let trace = vm_output.take_trace();

        let committed_output = vm_output.into_transaction_output_with_materialized_write_set(
            materialized_resource_write_set,
            materialized_events,
        )?;
        Ok((committed_output, trace))
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
    pub fn execute_block<
        S: StateView + Sync,
        L: TransactionCommitHook<TransactionOutput>,
        TP: TxnProvider<SignatureVerifiedTransaction, AuxiliaryInfo> + Sync,
    >(
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

                // Flush the speculative logs of the committed transactions.
                let pos = transaction_outputs.partition_point(|o| !o.status().is_retry());

                if state_view.id() != StateViewId::Miscellaneous {
                    // Speculation is disabled in Miscellaneous context, which is used by testing and
                    // can even lead to concurrent execute_block invocations, leading to errors on flush.
                    flush_speculative_logs(pos);
                }

                Ok(BlockOutput::new(transaction_outputs, block_epilogue_txn))
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
}

// Same as AptosBlockExecutorWrapper with AptosExecutorTask
pub type AptosVMBlockExecutorWrapper = AptosBlockExecutorWrapper<AptosExecutorTask>;
