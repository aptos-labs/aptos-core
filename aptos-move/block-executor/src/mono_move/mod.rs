// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Implementation of single transaction executor for MonoMove.

use crate::{
    captured_reads::TxnInput,
    code_cache_global::GlobalModuleCache,
    errors::ResourceGroupSerializationError,
    executor_utilities::Materializer,
    single_transaction_executor::{SharedViewArgs, SingleTransactionExecutor, ViewMode},
    task::{ExecutionStatus, TxnOutput},
    types::InputOutputKey,
};
use aptos_aggregator::delayed_change::DelayedChange;
use aptos_mvhashmap::{
    types::{Incarnation, TxnIndex},
    versioned_data::VersionedData,
    versioned_delayed_fields::TVersionedDelayedFieldView,
    versioned_group_data::VersionedGroupData,
};
use aptos_types::{
    block_executor::value::SpeculativeValue,
    error::{code_invariant_error, PanicError, PanicOr},
    fee_statement::FeeStatement,
    state_store::{state_key::StateKey, state_value::StateValue, table::TableHandle, TStateView},
    transaction::{
        signature_verified_transaction::SignatureVerifiedTransaction, AuxiliaryInfo,
        BlockExecutableTransaction as Transaction, TransactionAuxiliaryData, TransactionOutput,
    },
    vm::modules::AptosModuleExtension,
    write_set::{WriteOpKind, WriteSet},
};
use aptos_vm_environment::environment::AptosEnvironment;
use aptos_vm_types::resolver::ResourceGroupSize;
use mono_move_aptos_state_view_providers::StateViewModuleProvider;
use mono_move_aptos_transaction_executor::{
    production_natives, AptosTransactionExecutor, TxnOutcome,
};
use mono_move_core::{
    storage::resource_provider::{InMemoryStorageKey, ReadPin},
    struct_tag_of,
};
use mono_move_global_context::{ExecutionGuard, GlobalContext};
use mono_move_runtime::{SessionEffects, WriteClass};
use move_binary_format::CompiledModule;
use move_core_types::language_storage::{ModuleId, StructTag};
use move_vm_runtime::Module;
use move_vm_types::{code::SyncModuleCache, delayed_values::delayed_field_id::DelayedFieldID};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    ptr::NonNull,
    sync::Arc,
};

mod provider;
use provider::BlockSTMSequentialProvider;

/// An in-memory write produced by MonoMove transaction execution. This write
/// can be later converted to storage format if needed.
#[derive(Clone)]
pub enum MonoValue {
    Write {
        ptr: NonNull<u8>,
        kind: WriteOpKind,
        /// Pins the allocation backing the pointer. As long as we hold the pin,
        /// using the pointer is safe.
        pin: Arc<dyn ReadPin>,
    },
    Deletion,
}

// SAFETY: The value stores the pointer to the immutable value in a frozen,
// never-mutated arena. The ref-counted pointer to arena is always carried along
// so it is safe to dereference this pointer and share between threads.
unsafe impl Send for MonoValue {}
unsafe impl Sync for MonoValue {}

impl SpeculativeValue for MonoValue {
    fn eq_value(&self, _other: &Self) -> bool {
        // False here only forces re-validation, so safe to use.
        // TODO(perf): carry layouts so that structs can be validated as a single memcmp?
        false
    }

    fn eq_metadata(&self, _other: &Self) -> bool {
        // TODO(cleanup): refactor group metadata to not be a write op?
        false
    }

    fn bytes_len(&self) -> Option<usize> {
        // TODO(cleanup): this is only used for memory logging, revisit.
        match self {
            // A heap value's serialized size is unknown until materialization.
            MonoValue::Write { .. } => None,
            MonoValue::Deletion => None,
        }
    }

    fn write_op_kind(&self) -> WriteOpKind {
        match self {
            MonoValue::Write { kind, .. } => kind.clone(),
            MonoValue::Deletion => WriteOpKind::Deletion,
        }
    }
}

/// A MonoMove transaction's read set.
// TODO(completeness): implement when parallel execution is supported.
#[derive(Default)]
pub struct MonoReads;

impl TxnInput for MonoReads {
    type Key = InMemoryStorageKey;
    // TODO(perf): can use InternedType here, but current trait requires ordering
    // and serialize.
    type Tag = StructTag;
    type Value = MonoValue;

    fn validate_data_reads(
        &self,
        _data_map: &VersionedData<Self::Key, Self::Value>,
        _idx_to_validate: TxnIndex,
    ) -> bool {
        // TODO(completeness): add validation support.
        true
    }

    fn validate_group_reads(
        &self,
        _group_map: &VersionedGroupData<Self::Key, Self::Tag, Self::Value>,
        _idx_to_validate: TxnIndex,
    ) -> bool {
        // TODO(completeness): add validation support.
        true
    }

    fn validate_delayed_field_reads(
        &self,
        _delayed_fields: &dyn TVersionedDelayedFieldView<DelayedFieldID>,
        _idx_to_validate: TxnIndex,
    ) -> Result<bool, PanicError> {
        // TODO(completeness): add validation support.
        Ok(true)
    }

    fn legacy_validate_module_reads(
        &self,
        _global_module_cache: &GlobalModuleCache<
            ModuleId,
            CompiledModule,
            Module,
            AptosModuleExtension,
        >,
        _per_block_module_cache: &SyncModuleCache<
            ModuleId,
            CompiledModule,
            Module,
            AptosModuleExtension,
            Option<TxnIndex>,
        >,
        _maybe_updated_module_keys: Option<&BTreeSet<ModuleId>>,
    ) -> bool {
        // Module publishing in the middle of the block is not supported by
        // MonoMove.
        true
    }

    fn record_delayed_field_application_failure(&mut self) {
        // TODO(completeness): add parallel MonoMove support.
    }

    fn incarnation(&self) -> Option<Incarnation> {
        // TODO(completeness): add parallel MonoMove support.
        None
    }

    fn is_incorrect_use(&self) -> bool {
        // TODO(completeness): add parallel MonoMove support.
        false
    }

    fn get_read_summary(&self) -> HashSet<InputOutputKey<Self::Key, Self::Tag>> {
        // TODO(completeness): add parallel MonoMove support.
        HashSet::new()
    }
}

/// The VM's [`TxnOutcome`] is kept during speculative execution. It is only
/// converted to storage output representation after commit.
pub enum MonoTxnOutput {
    Executed {
        outcome: TxnOutcome,
        /// Whether this transaction emitted a reconfiguration (new-epoch) event,
        /// after which the block executor skips the remaining transactions.
        skips_rest: bool,
    },
    /// Signals block executor that this transaction has to be skipped and
    /// retried later.
    SkippedToRetry,
}

// SAFETY: Output holds reads and writes which are pointers. But those pointers
// point to a frozen heap which outlives the output or lives as ling as the output.
// The keys (interned types) live in the global arena, and they also outlive the
// output. Hence, sharing it across worker threads is sound.
unsafe impl Send for MonoTxnOutput {}
unsafe impl Sync for MonoTxnOutput {}

impl std::fmt::Debug for MonoTxnOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO(cleanup): change TxnOutput: Debug so that this is safe here.
        match self {
            MonoTxnOutput::Executed { .. } => f.write_str("MonoTxnOutput::Executed"),
            MonoTxnOutput::SkippedToRetry => f.write_str("MonoTxnOutput::SkippedToRetry"),
        }
    }
}

impl MonoTxnOutput {
    /// The frozen effects of a committed, executed transaction, or `None` for a
    /// discard, an empty-effects commit, or a skip (all write nothing).
    fn effects(&self) -> Option<&SessionEffects> {
        match self {
            MonoTxnOutput::Executed { outcome, .. } => match outcome {
                TxnOutcome::Executed { effects, .. } => Some(effects),
                TxnOutcome::Discarded(_) => None,
                TxnOutcome::ExecutedNoEffects(_) => None,
                // TODO(correctness): Revisit this arm: unexpected system txn errors
                //   should be handled at execution time!
                TxnOutcome::UnexpectedSystemTransactionFailure(_) => None,
            },
            MonoTxnOutput::SkippedToRetry => None,
        }
    }
}

impl TxnOutput for MonoTxnOutput {
    type CommittedOutput = TransactionOutput;
    type Key = InMemoryStorageKey;
    // TODO(perf): can use InternedType here, but current trait requires ordering
    // and serialize.
    type Tag = StructTag;
    type Txn = SignatureVerifiedTransaction;
    type Value = MonoValue;

    fn skip_output() -> Self {
        MonoTxnOutput::SkippedToRetry
    }

    fn resource_write_set(&self) -> HashMap<Self::Key, Self::Value> {
        let Some(effects) = self.effects() else {
            return HashMap::new();
        };

        let pin: Arc<dyn ReadPin> = effects.frozen_heap();
        effects
            .read_write_set()
            .writes_unordered()
            .map(|(key, class, _group)| {
                let value = match class {
                    WriteClass::Creation(ptr) => MonoValue::Write {
                        ptr,
                        kind: WriteOpKind::Creation,
                        pin: pin.clone(),
                    },
                    WriteClass::Modification(ptr) => MonoValue::Write {
                        ptr,
                        kind: WriteOpKind::Modification,
                        pin: pin.clone(),
                    },
                    WriteClass::Deletion => MonoValue::Deletion,
                };
                (key.clone(), value)
            })
            .collect()
    }

    fn delayed_field_change_set(&self) -> BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>> {
        // TODO(completeness): support delayed fields.
        BTreeMap::new()
    }

    fn resource_group_write_set(
        &self,
    ) -> HashMap<
        Self::Key,
        (
            Self::Value,
            ResourceGroupSize,
            BTreeMap<Self::Tag, Self::Value>,
        ),
    > {
        HashMap::new()
    }

    fn for_each_resource_key(
        &self,
        callback: &mut dyn FnMut(&Self::Key) -> Result<(), PanicError>,
    ) -> Result<(), PanicError> {
        for key in self.effects().into_iter().flat_map(|effects| {
            effects
                .read_write_set()
                .writes_unordered()
                .map(|(key, _, _)| key)
        }) {
            callback(key)?;
        }
        Ok(())
    }

    fn for_each_resource_group_key_and_tags(
        &self,
        _callback: &mut dyn FnMut(&Self::Key, HashSet<&Self::Tag>) -> Result<(), PanicError>,
    ) -> Result<(), PanicError> {
        Ok(())
    }

    fn for_each_module_write(
        &self,
        _callback: &mut dyn FnMut(&ModuleId, StateValue) -> Result<(), PanicError>,
    ) -> Result<(), PanicError> {
        // MonoMove does not publish modules - nothing to do.
        Ok(())
    }

    fn fee_statement(&self) -> FeeStatement {
        match self {
            MonoTxnOutput::Executed { outcome, .. } => match outcome {
                TxnOutcome::Executed { fee_statement, .. } => *fee_statement,
                TxnOutcome::Discarded(_) => FeeStatement::zero(),
                TxnOutcome::ExecutedNoEffects(_) => FeeStatement::zero(),
                TxnOutcome::UnexpectedSystemTransactionFailure(_) => FeeStatement::zero(),
            },
            MonoTxnOutput::SkippedToRetry => FeeStatement::zero(),
        }
    }

    fn has_new_epoch_event(&self) -> bool {
        match self {
            MonoTxnOutput::Executed { skips_rest, .. } => *skips_rest,
            MonoTxnOutput::SkippedToRetry => false,
        }
    }

    fn output_approx_size(&self) -> u64 {
        // TODO(metering): size the serialized writes for the block output limit.
        0
    }

    fn get_write_summary(&self) -> HashSet<InputOutputKey<Self::Key, Self::Tag>> {
        // TODO(completeness): support write summaries.
        HashSet::new()
    }

    fn storage_keys_read(&self) -> impl Iterator<Item = &Self::Key> {
        // TODO(completeness): support read keys information.
        std::iter::empty()
    }

    fn storage_keys_written(&self) -> impl Iterator<Item = &Self::Key> {
        // TODO(completeness): support written keys information.
        std::iter::empty()
    }

    fn check_materialization(&self, _materializer: &impl Materializer<Self::Txn>) -> bool {
        // TODO(security): can add extra checks here?
        true
    }
}

/// A per-worker MonoMove executor.
pub struct MonoTransactionExecutor {
    ctx: Arc<GlobalContext>,
    /// The block's environment (features, configs).
    environment: AptosEnvironment,
    /// This executor's worker ID.
    worker_id: u32,
}

impl MonoTransactionExecutor {
    fn execution_guard(&self) -> Result<ExecutionGuard<'_>, PanicError> {
        self.ctx
            .try_execution_context(self.worker_id as usize)
            .ok_or_else(|| code_invariant_error("Failed to obtain execution context for worker"))
    }
}

impl SingleTransactionExecutor for MonoTransactionExecutor {
    type AuxiliaryInfo = AuxiliaryInfo;
    type Input = MonoReads;
    type Key = InMemoryStorageKey;
    type Output = MonoTxnOutput;
    type Tag = StructTag;
    type Txn = SignatureVerifiedTransaction;
    type Value = MonoValue;

    fn init(
        environment: &AptosEnvironment,
        ctx: Arc<GlobalContext>,
        _state_view: &impl TStateView<Key = <Self::Txn as Transaction>::Key>,
        worker_id: u32,
        _async_runtime_checks_enabled: bool,
    ) -> Self {
        Self {
            ctx,
            environment: environment.clone(),
            worker_id,
        }
    }

    fn execute<S: TStateView<Key = <Self::Txn as Transaction>::Key> + Sync>(
        &self,
        shared: SharedViewArgs<'_, S>,
        mode: ViewMode<'_, Self::Input>,
        txn: &Self::Txn,
        auxiliary_info: &Self::AuxiliaryInfo,
        _txn_idx: TxnIndex,
    ) -> Result<(ExecutionStatus<Self::Output>, Self::Input), PanicError> {
        let ViewMode::Sequential { unsync_map, .. } = mode else {
            return Err(code_invariant_error(
                "MonoMove parallel execution is not yet supported",
            ));
        };

        // TODO(security): All transactions are valid, V1 VM unwraps. Consider
        //   doing the same here or return a meaningful error.
        let SignatureVerifiedTransaction::Valid(inner_txn) = txn else {
            return Err(code_invariant_error(
                "All transactions must be valid for execution",
            ));
        };

        let guard = self.execution_guard()?;
        let data_provider = BlockSTMSequentialProvider::new(&guard, shared.base_view, unsync_map);
        let module_provider = StateViewModuleProvider::new(shared.base_view);
        let natives = production_natives();

        // TODO(cleanup): usage can be cached in per-block cache.
        let usage = shared
            .base_view
            .get_usage()
            .map_err(|e| code_invariant_error(format!("MonoMove: state usage read failed: {e}")))?;

        // TODO(completeness): Run metered execution. For now using no metering
        // simplifies tests.
        let outcome = AptosTransactionExecutor::new(
            &guard,
            natives,
            &module_provider,
            &data_provider,
            &self.environment,
            usage,
        )
        .without_metering()
        .execute_transaction(inner_txn, auxiliary_info);

        // A reconfiguration (new epoch) event cuts the block early. Record this
        // in output so remaining transactions can be skipped.
        let skips_rest = outcome.has_new_epoch_event().map_err(|e| {
            code_invariant_error(format!("Failed to inspect events for reconfiguration: {e}"))
        })?;
        Ok((
            ExecutionStatus::Executed {
                output: MonoTxnOutput::Executed {
                    outcome,
                    skips_rest,
                },
                skips_rest,
            },
            MonoReads,
        ))
    }

    fn materialize<S: TStateView<Key = <Self::Txn as Transaction>::Key> + Sync>(
        &self,
        output: Self::Output,
        _input: &Self::Input,
        shared: SharedViewArgs<'_, S>,
        mode: ViewMode<'_, Self::Input>,
        _txn_idx: TxnIndex,
    ) -> Result<
        <Self::Output as TxnOutput>::CommittedOutput,
        PanicOr<ResourceGroupSerializationError>,
    > {
        let ViewMode::Sequential { unsync_map, .. } = mode else {
            return Err(PanicOr::CodeInvariantError(
                "MonoMove parallel execution is not yet supported".to_string(),
            ));
        };

        let outcome = match output {
            MonoTxnOutput::Executed { outcome, .. } => outcome,
            // A skipped output renders to an empty, kept-success transaction.
            MonoTxnOutput::SkippedToRetry => {
                return Ok(TransactionOutput::new_success_with_write_set(
                    WriteSet::default(),
                ))
            },
        };

        let guard = self.execution_guard()?;
        let provider = BlockSTMSequentialProvider::new(&guard, shared.base_view, unsync_map);

        // TODO(correctenss): currently system txn failure fails here and not at txn
        //   execution time. Refactor materialization so that this does not happen!
        let (output, groups) = outcome
            .materialize(
                &guard,
                &provider,
                self.environment.features(),
                // Legacy format, set to none because not used.
                TransactionAuxiliaryData::None,
            )
            .map_err(|e| {
                PanicOr::CodeInvariantError(format!("Failed to materialize outputs: {e}"))
            })?;

        // Cache each group this transaction assembled so a later transaction
        // touching the same group merges on top of it.
        for (group_key, members) in groups {
            unsync_map.insert_group(group_key, members);
        }

        Ok(output)
    }

    fn check_materialization<S: TStateView<Key = <Self::Txn as Transaction>::Key> + Sync>(
        &self,
        _output: &Self::Output,
        _shared: SharedViewArgs<'_, S>,
        _mode: ViewMode<'_, Self::Input>,
        _txn_idx: TxnIndex,
    ) -> bool {
        true
    }

    fn materialize_storage_key(&self, key: InMemoryStorageKey) -> Result<StateKey, PanicError> {
        match key {
            InMemoryStorageKey::Resource { address, ty } => {
                // TODO(correctness): a group member's `Resource` key lowers here
                //   to `StateKey::resource`, its own slot, not the enclosing group
                //   slot. To fix this we need a new InMemoryStorageKey type.
                struct_tag_of(ty)
                    .and_then(|tag| StateKey::resource(&address, &tag).ok())
                    .ok_or_else(|| code_invariant_error("Failed to build resource state key"))
            },
            InMemoryStorageKey::TableItem { handle, key, .. } => {
                Ok(StateKey::table_item(&TableHandle(handle.address()), &key))
            },
        }
    }
}
