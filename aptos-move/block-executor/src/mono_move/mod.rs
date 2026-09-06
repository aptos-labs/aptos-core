// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Implementation of single transaction executor for MonoMove.

use crate::{
    errors::ResourceGroupSerializationError,
    executor_utilities::Materializer,
    single_transaction_executor::{SharedViewArgs, SingleTransactionExecutor, ViewMode},
    task::{ExecutionStatus, TxnOutput},
    types::InputOutputKey,
};
use aptos_aggregator::delayed_change::DelayedChange;
use aptos_mvhashmap::types::TxnIndex;
use aptos_types::{
    block_executor::value::SpeculativeValue,
    error::{code_invariant_error, PanicError, PanicOr},
    fee_statement::FeeStatement,
    state_store::{state_key::StateKey, state_value::StateValue, TStateView},
    transaction::{
        signature_verified_transaction::SignatureVerifiedTransaction, AuxiliaryInfo,
        BlockExecutableTransaction as Transaction, TransactionAuxiliaryData, TransactionOutput,
    },
    write_set::{WriteOpKind, WriteSet},
};
use aptos_vm_environment::environment::AptosEnvironment;
use aptos_vm_types::resolver::ResourceGroupSize;
use bytes::Bytes;
use mono_move_aptos_state_view_providers::StateViewModuleProvider;
use mono_move_aptos_transaction_executor::{
    production_natives, AptosDataProvider, AptosTransactionExecutor, TxnOutcome,
};
use mono_move_core::{
    nominal_tag,
    storage::resource_provider::{InMemoryStorageKey, ReadPin},
    types::InternedType,
};
use mono_move_global_context::{ExecutionGuard, GlobalContext};
use mono_move_runtime::{SegmentedArena, SessionEffects, WriteClass};
use move_core_types::language_storage::{ModuleId, StructTag};
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    ptr::NonNull,
    sync::Arc,
};

mod provider;
mod reads;

use provider::{BlockSTMParallelProvider, BlockSTMSequentialProvider};
/// The combinatorial tests drive the speculative read loops with their own key
/// and tag types.
#[cfg(test)]
pub(crate) use provider::{ParallelReader, StorageBase};
pub use reads::MonoReads;

/// An in-memory write produced by MonoMove transaction execution. This write
/// can be later converted to storage format if needed.
#[derive(Clone)]
pub enum MonoValue {
    /// A resource-group member exactly as storage holds it. A group's stored
    /// blob decodes to bytes per member and does not name the member's type, so
    /// members enter the map like this and the first reader that knows the type
    /// replaces the entry with [`MonoValue::Write`].
    RawFromStorage(Bytes),
    Write {
        ptr: NonNull<u8>,
        /// The value's interned type, so that any transaction assembling the
        /// enclosing group's blob can serialize this member.
        ty: InternedType,
        kind: WriteOpKind,
        /// Pins the allocation backing the pointer. As long as we hold the pin,
        /// using the pointer is safe.
        pin: Arc<dyn ReadPin>,
    },
    Deletion,
    /// The value Block-STM versions at a resource group's own slot. MonoMove
    /// carries no group metadata, but the slot still has to hold an entry: the
    /// block executor writes and removes it alongside the group's members.
    GroupMetadata,
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

    fn eq_metadata(&self, other: &Self) -> bool {
        // Only a group slot carries metadata, and MonoMove's is always the same.
        matches!(
            (self, other),
            (MonoValue::GroupMetadata, MonoValue::GroupMetadata)
        )
    }

    fn bytes_len(&self) -> Option<usize> {
        // TODO(cleanup): this is only used for memory logging, revisit.
        //
        // Returning `None` everywhere also keeps every group size at
        // `ResourceGroupSize::zero_combined()`, so group sizes never
        // participate in validation.
        match self {
            // A heap value's serialized size is unknown until materialization.
            MonoValue::Write { .. } => None,
            MonoValue::RawFromStorage(_) => None,
            MonoValue::Deletion => None,
            MonoValue::GroupMetadata => None,
        }
    }

    fn write_op_kind(&self) -> WriteOpKind {
        match self {
            MonoValue::Write { kind, .. } => kind.clone(),
            MonoValue::RawFromStorage(_) | MonoValue::GroupMetadata => WriteOpKind::Modification,
            MonoValue::Deletion => WriteOpKind::Deletion,
        }
    }
}

/// Builds the map value for a write of `ty`, pinned to the heap it lives in.
fn written_value(class: WriteClass, ty: InternedType, pin: &Arc<dyn ReadPin>) -> MonoValue {
    match class {
        WriteClass::Creation(ptr) => MonoValue::Write {
            ptr,
            ty,
            kind: WriteOpKind::Creation,
            pin: pin.clone(),
        },
        WriteClass::Modification(ptr) => MonoValue::Write {
            ptr,
            ty,
            kind: WriteOpKind::Modification,
            pin: pin.clone(),
        },
        WriteClass::Deletion => MonoValue::Deletion,
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
// point to a frozen heap which outlives the output or lives as long as the output.
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
                TxnOutcome::Discarded { .. } => None,
                TxnOutcome::ExecutedNoEffects { .. } => None,
                // TODO(correctness): Revisit this arm: unexpected system txn errors
                //   should be handled at execution time!
                TxnOutcome::UnexpectedSystemTransactionFailure(_) => None,
            },
            MonoTxnOutput::SkippedToRetry => None,
        }
    }

    /// The transaction's writes that land inside a resource group, keyed by the
    /// group's own storage slot and tagged by the member's struct tag.
    ///
    /// A member whose type has no struct tag cannot be written to storage at
    /// all, so it is left out; materialization reports it as a failure.
    fn group_writes(&self) -> HashMap<InMemoryStorageKey, BTreeMap<StructTag, MonoValue>> {
        let Some(effects) = self.effects() else {
            return HashMap::new();
        };
        let pin: Arc<dyn ReadPin> = effects.frozen_heap();

        let mut groups: HashMap<_, BTreeMap<_, _>> = HashMap::new();
        for (key, class, group) in effects.read_write_set().writes_unordered() {
            let Some(group_ty) = group else {
                continue;
            };
            let ty = key.value_ty();
            let Ok(tag) = nominal_tag(ty) else {
                continue;
            };
            groups
                .entry(InMemoryStorageKey::resource_group(key.address(), group_ty))
                .or_default()
                .insert(tag, written_value(class, ty, &pin));
        }
        groups
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
            // Group members belong to their group's slot, not to a flat one.
            .filter(|(_, _, group)| group.is_none())
            .map(|(key, class, _)| (key.clone(), written_value(class, key.value_ty(), &pin)))
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
        self.group_writes()
            .into_iter()
            .map(|(group_key, members)| {
                (
                    group_key,
                    (
                        MonoValue::GroupMetadata,
                        // Every `MonoValue` reports no serialized length, so
                        // this is also what the map computes for the group.
                        ResourceGroupSize::zero_combined(),
                        members,
                    ),
                )
            })
            .collect()
    }

    fn for_each_resource_key(
        &self,
        callback: &mut dyn FnMut(&Self::Key) -> Result<(), PanicError>,
    ) -> Result<(), PanicError> {
        for key in self.effects().into_iter().flat_map(|effects| {
            effects
                .read_write_set()
                .writes_unordered()
                .filter(|(_, _, group)| group.is_none())
                .map(|(key, _, _)| key)
        }) {
            callback(key)?;
        }
        Ok(())
    }

    fn for_each_resource_group_key_and_tags(
        &self,
        callback: &mut dyn FnMut(&Self::Key, HashSet<&Self::Tag>) -> Result<(), PanicError>,
    ) -> Result<(), PanicError> {
        for (group_key, members) in self.group_writes() {
            callback(&group_key, members.keys().collect())?;
        }
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
                TxnOutcome::Discarded { .. } => FeeStatement::zero(),
                TxnOutcome::ExecutedNoEffects { .. } => FeeStatement::zero(),
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
    /// Where this worker materializes the values it reads from storage. Only
    /// used for parallel execution; sequential execution shares one arena
    /// through the unsync map.
    arena: SegmentedArena,
}

impl MonoTransactionExecutor {
    fn execution_guard(&self) -> Result<ExecutionGuard<'_>, PanicError> {
        let worker_id = self.worker_id as usize;
        // Locking an arena the context was not built for panics, and a panic in
        // a worker thread takes the node down. Fail the block instead.
        if worker_id >= self.ctx.num_execution_workers() {
            return Err(code_invariant_error(format!(
                "Worker {worker_id} has no arena: the global context was built \
                 for {} workers",
                self.ctx.num_execution_workers()
            )));
        }
        self.ctx
            .try_execution_context(worker_id)
            .ok_or_else(|| code_invariant_error("Failed to obtain execution context for worker"))
    }

    /// Runs the transaction against `data_provider`, returning its outcome and
    /// whether it cut the block short with a reconfiguration event.
    fn run<'g, S: TStateView<Key = StateKey> + Sync, P: AptosDataProvider>(
        &self,
        guard: &'g ExecutionGuard<'g>,
        base_view: &S,
        data_provider: &P,
        txn: &SignatureVerifiedTransaction,
        auxiliary_info: &AuxiliaryInfo,
    ) -> Result<(TxnOutcome, bool), PanicError> {
        // TODO(security): All transactions are valid, V1 VM unwraps. Consider
        //   doing the same here or return a meaningful error.
        let SignatureVerifiedTransaction::Valid(inner_txn) = txn else {
            return Err(code_invariant_error(
                "All transactions must be valid for execution",
            ));
        };

        let module_provider = StateViewModuleProvider::new(base_view);
        let natives = production_natives();

        // TODO(cleanup): usage can be cached in per-block cache.
        let usage = base_view
            .get_usage()
            .map_err(|e| code_invariant_error(format!("MonoMove: state usage read failed: {e}")))?;

        // TODO(completeness): Run metered execution. For now using no metering
        // simplifies tests.
        let outcome = AptosTransactionExecutor::new(
            guard,
            natives,
            &module_provider,
            data_provider,
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
        Ok((outcome, skips_rest))
    }
}

impl SingleTransactionExecutor for MonoTransactionExecutor {
    type AuxiliaryInfo = AuxiliaryInfo;
    type Input = MonoReads<InMemoryStorageKey, StructTag>;
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
            arena: SegmentedArena::new(),
        }
    }

    fn execute<S: TStateView<Key = <Self::Txn as Transaction>::Key> + Sync>(
        &self,
        shared: SharedViewArgs<'_, S>,
        mode: ViewMode<'_, Self::Input>,
        txn: &Self::Txn,
        auxiliary_info: &Self::AuxiliaryInfo,
        txn_idx: TxnIndex,
    ) -> Result<(ExecutionStatus<Self::Output>, Self::Input), PanicError> {
        let guard = self.execution_guard()?;

        let (outcome, skips_rest, reads) = match mode {
            ViewMode::Sequential { unsync_map, .. } => {
                let provider =
                    BlockSTMSequentialProvider::new(&guard, shared.base_view, unsync_map);
                let (outcome, skips_rest) =
                    self.run(&guard, shared.base_view, &provider, txn, auxiliary_info)?;
                // Sequential execution never validates, so the read set is
                // not collected.
                (outcome, skips_rest, MonoReads::empty(None))
            },
            ViewMode::Parallel {
                versioned_map,
                scheduler,
                incarnation,
                ..
            } => {
                let provider = BlockSTMParallelProvider::new(
                    &guard,
                    shared.base_view,
                    versioned_map,
                    scheduler,
                    txn_idx,
                    incarnation,
                    &self.arena,
                );
                let result = self.run(&guard, shared.base_view, &provider, txn, auxiliary_info);
                if provider.speculative_failure() {
                    // A read could not be served, so whatever the VM produced
                    // from it must not commit. This is checked before the run's
                    // own error: an error a doomed read caused is not real.
                    return Ok((
                        ExecutionStatus::SpeculativeFailure,
                        MonoReads::empty(Some(incarnation)),
                    ));
                }
                let (outcome, skips_rest) = result?;
                let reads = match outcome.read_write_set() {
                    Some(rws) => MonoReads::from_read_write_set(rws, incarnation)?,
                    None => MonoReads::empty(Some(incarnation)),
                };
                (outcome, skips_rest, reads)
            },
        };

        Ok((
            ExecutionStatus::Executed {
                output: MonoTxnOutput::Executed {
                    outcome,
                    skips_rest,
                },
                skips_rest,
            },
            reads,
        ))
    }

    fn materialize<S: TStateView<Key = <Self::Txn as Transaction>::Key> + Sync>(
        &self,
        output: Self::Output,
        _input: &Self::Input,
        shared: SharedViewArgs<'_, S>,
        mode: ViewMode<'_, Self::Input>,
        txn_idx: TxnIndex,
    ) -> Result<
        <Self::Output as TxnOutput>::CommittedOutput,
        PanicOr<ResourceGroupSerializationError>,
    > {
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
        let features = self.environment.features();
        // Legacy format, set to none because not used.
        let auxiliary_data = TransactionAuxiliaryData::None;

        // TODO(correctness): currently system txn failure fails here and not at txn
        //   execution time. Refactor materialization so that this does not happen!
        let materialization_failed =
            |e| PanicOr::CodeInvariantError(format!("Failed to materialize outputs: {e}"));

        match mode {
            ViewMode::Sequential { unsync_map, .. } => {
                let provider =
                    BlockSTMSequentialProvider::new(&guard, shared.base_view, unsync_map);
                let (output, groups) = outcome
                    .materialize(&guard, &provider, features, auxiliary_data)
                    .map_err(materialization_failed)?;

                // Cache each group this transaction assembled so a later
                // transaction touching the same group merges on top of it.
                for (group_key, members) in groups {
                    unsync_map.insert_group(group_key, members);
                }
                Ok(output)
            },
            ViewMode::Parallel {
                versioned_map,
                scheduler,
                incarnation,
                ..
            } => {
                let provider = BlockSTMParallelProvider::new(
                    &guard,
                    shared.base_view,
                    versioned_map,
                    scheduler,
                    txn_idx,
                    incarnation,
                    &self.arena,
                );
                // Groups are assembled from the versioned map on demand, so
                // nothing is cached back here.
                let (output, _groups) = outcome
                    .materialize(&guard, &provider, features, auxiliary_data)
                    .map_err(materialization_failed)?;
                Ok(output)
            },
        }
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
        key.as_state_key()
            .map_err(|e| code_invariant_error(format!("Failed to build a state key: {e:#}")))
    }
}
