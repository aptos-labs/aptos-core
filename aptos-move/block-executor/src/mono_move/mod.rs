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
    types::{
        Incarnation, MVDataError, MVDataOutput, StorageVersion, TxnIndex, Version as MVVersion,
    },
    versioned_data::VersionedData,
    versioned_delayed_fields::TVersionedDelayedFieldView,
    versioned_group_data::VersionedGroupData,
};
use aptos_types::{
    block_executor::value::SpeculativeValue,
    error::{code_invariant_error, PanicError, PanicOr},
    fee_statement::FeeStatement,
    state_store::{state_key::StateKey, state_value::StateValue, TStateView},
    transaction::{
        signature_verified_transaction::SignatureVerifiedTransaction, AuxiliaryInfo,
        BlockExecutableTransaction as Transaction, TransactionAuxiliaryData, TransactionOutput,
    },
    vm::modules::AptosModuleExtension,
    write_set::{WriteOpKind, WriteSet},
};
use aptos_vm_environment::environment::AptosEnvironment;
use aptos_vm_types::resolver::ResourceGroupSize;
use bytes::Bytes;
use group_chain::{
    committed_value, finish_group_post_commit, unbuilt_metadata_before, GroupMetadata, GroupView,
};
use mono_move_aptos_state_view_providers::StateViewModuleProvider;
use mono_move_aptos_transaction_executor::{
    production_natives, AptosTransactionExecutor, DiscardReason, TxnOutcome,
};
use mono_move_core::{
    nominal_tag,
    storage::resource_provider::{
        InMemoryStorageKey, ReadPin, ResourceProvider, ResourceProviderError, StorageRead, Version,
    },
    types::InternedType,
};
use mono_move_global_context::{ExecutionGuard, GlobalContext};
use mono_move_runtime::{serialize, SegmentedArena, SessionEffects, WriteClass};
use move_binary_format::CompiledModule;
use move_core_types::language_storage::{ModuleId, StructTag};
use move_vm_runtime::Module;
use move_vm_types::{code::SyncModuleCache, delayed_values::delayed_field_id::DelayedFieldID};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    ptr::NonNull,
    sync::Arc,
};

pub(crate) mod group_chain;
mod provider;

#[cfg(test)]
pub(crate) use provider::ParallelReader;
use provider::{BlockSTMParallelProvider, BlockSTMSequentialProvider};

/// An in-memory write produced by MonoMove transaction execution. This write
/// can be later converted to storage format if needed.
#[derive(Clone)]
pub enum MonoValue<K = InMemoryStorageKey> {
    Write {
        ptr: NonNull<u8>,
        kind: WriteOpKind,
        /// Pins the allocation backing the pointer. As long as we hold the pin,
        /// using the pointer is safe.
        pin: Arc<dyn ReadPin>,
    },
    Deletion,
    /// Group write metadata and cache. Group members live as their own
    /// writes/deletions under their keys.
    //
    // TODO(completeness): add state value metadata and size here.
    GroupMetadata {
        group: Arc<GroupMetadata<K>>,
    },
}

// SAFETY: The value stores the pointer to the immutable value in a frozen,
// never-mutated arena. The ref-counted pointer to arena is always carried along
// so it is safe to dereference this pointer and share between threads.
unsafe impl<K: Send + Sync> Send for MonoValue<K> {}
unsafe impl<K: Send + Sync> Sync for MonoValue<K> {}

impl<K> MonoValue<K> {
    /// Serves the value a map holds at a slot as a read at `version`.
    /// Sequential execution does not speculate, so it passes [`None`].
    fn as_storage_read(&self, version: Version) -> Result<StorageRead, ResourceProviderError> {
        Ok(match self {
            MonoValue::Write { ptr, pin, .. } => StorageRead::ExternalHeap {
                ptr: *ptr,
                version,
                pin: pin.clone(),
            },
            MonoValue::Deletion => StorageRead::DoesNotExist { version },
            // A group's own slot is never served to a transaction as a read.
            MonoValue::GroupMetadata { .. } => {
                return Err(ResourceProviderError::InvariantViolation(
                    "Group metadata served as a read".to_string(),
                ))
            },
        })
    }
}

impl<K: Clone + Send + Sync> SpeculativeValue for MonoValue<K> {
    fn eq_value(&self, _other: &Self) -> bool {
        // False here only forces re-validation, so safe to use.
        // TODO(perf): carry layouts so that structs can be validated as a single memcmp?
        false
    }

    fn eq_metadata(&self, _other: &Self) -> bool {
        // Block-STM only compares metadata on its resource-group path, which
        // MonoMove does not use: group members and metadata are written as
        // plain data.
        //
        // TODO(completeness): once group metadata carries the group's size, it
        // is validated together with resource writes.
        false
    }

    fn bytes_len(&self) -> Option<usize> {
        // TODO(cleanup): this is only used for memory logging, revisit.
        match self {
            // A heap value's serialized size is unknown until materialization.
            MonoValue::Write { .. } => None,
            MonoValue::Deletion => None,
            MonoValue::GroupMetadata { .. } => None,
        }
    }

    fn write_op_kind(&self) -> WriteOpKind {
        match self {
            MonoValue::Write { kind, .. } => kind.clone(),
            // TODO(completeness): store write op kind for group metadata?
            MonoValue::GroupMetadata { .. } => WriteOpKind::Modification,
            MonoValue::Deletion => WriteOpKind::Deletion,
        }
    }
}

/// A MonoMove transaction's read set, as Block-STM validates it.
pub struct MonoReads {
    /// The outcome whose read-write set holds the reads, shared with the
    /// transaction's output. [`None`] for a transaction whose reads were never
    /// collected: sequential execution, or a parallel execution that has
    /// nothing left to commit.
    outcome: Option<Arc<TxnOutcome>>,
    /// The incarnation these reads were captured at, for BlockSTMv2. [`None`]
    /// only under sequential execution, which does not validate.
    incarnation: Option<Incarnation>,
    /// Set when the transaction is known not to be committable regardless of
    /// what it read. Fails validation unconditionally.
    speculative_failure: bool,
}

// SAFETY: the outcome's effects only point into frozen heaps, which are never
// mutated again and which the effects and their reads pin. Their keys hold
// interned types from the global arena, which outlives the block.
unsafe impl Send for MonoReads {}
unsafe impl Sync for MonoReads {}

impl MonoReads {
    pub(crate) fn new(outcome: Arc<TxnOutcome>, incarnation: Incarnation) -> Self {
        Self {
            outcome: Some(outcome),
            incarnation: Some(incarnation),
            speculative_failure: false,
        }
    }

    /// A read set that validates trivially.
    pub(crate) fn empty(incarnation: Option<Incarnation>) -> Self {
        Self {
            outcome: None,
            incarnation,
            speculative_failure: false,
        }
    }
}

/// Whether a version recorded by the VM still names the entry Block-STM serves
/// now. A read of pre-block storage is recorded as [`None`].
pub(crate) fn version_matches(recorded: Version, observed: MVVersion) -> bool {
    match (recorded, observed) {
        (None, Err(StorageVersion)) => true,
        (Some(recorded), Ok(observed)) => recorded == observed,
        (None, Ok(_)) | (Some(_), Err(StorageVersion)) => false,
    }
}

impl TxnInput for MonoReads {
    type Key = InMemoryStorageKey;
    // MonoMove versions group members as plain resources, so Block-STM's group
    // tags are never used.
    type Tag = ();
    type Value = MonoValue;

    fn validate_data_reads(
        &self,
        data_map: &VersionedData<Self::Key, Self::Value>,
        idx_to_validate: TxnIndex,
    ) -> bool {
        if self.speculative_failure {
            return false;
        }
        let Some(effects) = self.outcome.as_ref().and_then(|outcome| outcome.effects()) else {
            return true;
        };
        effects
            .read_write_set()
            .reads_unordered()
            .all(|(key, recorded, _group)| {
                match data_map.fetch_data_no_record(key, idx_to_validate) {
                    Ok(MVDataOutput::Versioned(observed, _)) => version_matches(recorded, observed),
                    // A base value this transaction initialized cannot
                    // disappear, so this only happens if the entry was never
                    // there. Either way the recorded version is no longer
                    // backed by the map.
                    Err(MVDataError::Uninitialized) => false,
                    Err(MVDataError::Dependency(_)) => false,
                }
            })
    }

    fn validate_group_reads(
        &self,
        _group_map: &VersionedGroupData<Self::Key, Self::Tag, Self::Value>,
        _idx_to_validate: TxnIndex,
    ) -> bool {
        // Group members are validated as data reads.
        !self.speculative_failure
    }

    fn validate_delayed_field_reads(
        &self,
        _delayed_fields: &dyn TVersionedDelayedFieldView<DelayedFieldID>,
        _idx_to_validate: TxnIndex,
    ) -> Result<bool, PanicError> {
        // TODO(completeness): support delayed fields.
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
        // BlockSTMv1 has no channel of its own for a speculative failure and
        // reuses this one, so the flag must make validation fail even though
        // MonoMove has no delayed fields yet.
        self.speculative_failure = true;
    }

    fn incarnation(&self) -> Option<Incarnation> {
        self.incarnation
    }

    fn is_incorrect_use(&self) -> bool {
        // The legacy view records code invariant violations here because it
        // cannot return them through the VM. MonoMove's providers return them as
        // errors instead.
        //
        // TODO(correctness): such an error fails only the transaction, while the
        // legacy path fails the block.
        false
    }

    fn get_read_summary(&self) -> HashSet<InputOutputKey<Self::Key, Self::Tag>> {
        // TODO(perf): this disables the block-limit conflict heuristics.
        HashSet::new()
    }
}

/// The VM's [`TxnOutcome`] is kept during speculative execution. It is only
/// converted to storage output representation after commit.
pub enum MonoTxnOutput {
    Executed {
        /// Shared with the transaction's read set, which validates from the
        /// outcome's read-write set.
        outcome: Arc<TxnOutcome>,
        /// Whether this transaction emitted a reconfiguration (new-epoch) event,
        /// after which the block executor skips the remaining transactions.
        skips_rest: bool,
        /// One metadata entry per resource group this transaction wrote a member of,
        /// built once so that every caller shares it.
        group_metadata: Vec<(InMemoryStorageKey, Arc<GroupMetadata<InMemoryStorageKey>>)>,
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
    fn executed(outcome: Arc<TxnOutcome>, skips_rest: bool) -> Self {
        let mut output = MonoTxnOutput::Executed {
            outcome,
            skips_rest,
            group_metadata: vec![],
        };
        let written_groups = output
            .effects()
            .into_iter()
            .flat_map(|effects| effects.read_write_set().written_groups_unordered())
            .map(|(group_key, members)| {
                let metadata = GroupMetadata::from_changes(members.to_vec());
                (group_key.clone(), Arc::new(metadata))
            })
            .collect();
        match &mut output {
            MonoTxnOutput::Executed { group_metadata, .. } => *group_metadata = written_groups,
            MonoTxnOutput::SkippedToRetry => {},
        }
        output
    }

    /// The frozen effects of a committed, executed transaction, or `None` for a
    /// discard, an empty-effects commit, or a skip (all write nothing).
    fn effects(&self) -> Option<&SessionEffects> {
        match self {
            MonoTxnOutput::Executed { outcome, .. } => match outcome.as_ref() {
                TxnOutcome::Executed { effects, .. } => Some(effects),
                TxnOutcome::Discarded { .. } => None,
                TxnOutcome::ExecutedNoEffects(_) => None,
                // TODO(correctness): Revisit this arm: unexpected system txn errors
                //   should be handled at execution time!
                TxnOutcome::UnexpectedSystemTransactionFailure(_) => None,
                TxnOutcome::Panic(_) => None,
            },
            MonoTxnOutput::SkippedToRetry => None,
        }
    }

    /// The metadata of the resource groups this transaction wrote, keyed by group
    /// slot.
    fn group_metadata(&self) -> &[(InMemoryStorageKey, Arc<GroupMetadata<InMemoryStorageKey>>)] {
        match self {
            MonoTxnOutput::Executed { group_metadata, .. } => group_metadata,
            MonoTxnOutput::SkippedToRetry => &[],
        }
    }
}

impl TxnOutput for MonoTxnOutput {
    type CommittedOutput = TransactionOutput;
    type Key = InMemoryStorageKey;
    // Group members are versioned as plain resources, so there are no tags.
    type Tag = ();
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
            .chain(self.group_metadata().iter().map(|(group_key, metadata)| {
                let value = MonoValue::GroupMetadata {
                    group: metadata.clone(),
                };
                (group_key.clone(), value)
            }))
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
        // Group members are written as plain resources.
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
        for (group_key, _) in self.group_metadata() {
            callback(group_key)?;
        }
        Ok(())
    }

    fn for_each_resource_group_key_and_tags(
        &self,
        _callback: &mut dyn FnMut(&Self::Key, HashSet<&Self::Tag>) -> Result<(), PanicError>,
    ) -> Result<(), PanicError> {
        // Group members are written as plain resources.
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
            MonoTxnOutput::Executed { outcome, .. } => match outcome.as_ref() {
                TxnOutcome::Executed { fee_statement, .. } => *fee_statement,
                TxnOutcome::Discarded { .. } => FeeStatement::zero(),
                TxnOutcome::ExecutedNoEffects(_) => FeeStatement::zero(),
                TxnOutcome::UnexpectedSystemTransactionFailure(_) => FeeStatement::zero(),
                TxnOutcome::Panic(_) => FeeStatement::zero(),
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
        // TODO(correctness): report read keys, including module reads, so the
        // block epilogue promotes the same keys as the legacy VM.
        std::iter::empty()
    }

    fn storage_keys_written(&self) -> impl Iterator<Item = &Self::Key> {
        // TODO(correctness): report written keys, for hot-state promotion
        // parity with the legacy VM.
        std::iter::empty()
    }

    fn check_materialization(&self, _materializer: &impl Materializer<Self::Txn>) -> bool {
        // TODO(security): can add extra checks here?
        true
    }
}

/// The key of the member of type `member_ty` in a view.
//
// TODO(perf): cache the tag bytes per interned type.
fn member_tag(member_ty: InternedType) -> Result<Bytes, PanicError> {
    let tag = nominal_tag(member_ty)
        .map_err(|e| code_invariant_error(format!("Group member has no tag: {e:#}")))?;
    tag_bytes(&tag)
}

/// The BCS encoding of a member's struct tag, which keys it in a view.
fn tag_bytes(tag: &StructTag) -> Result<Bytes, PanicError> {
    bcs::to_bytes(tag)
        .map(Bytes::from)
        .map_err(|e| code_invariant_error(format!("Failed to encode a struct tag: {e}")))
}

/// What a transaction wrote to a group member at `key`: the member's key in the
/// group's view, and its new bytes or [`None`] if it was deleted.
fn member_change(
    guard: &ExecutionGuard<'_>,
    key: &InMemoryStorageKey,
    value: &MonoValue,
) -> Result<(Bytes, Option<Bytes>), PanicError> {
    let tag = member_tag(key.value_ty())?;
    let bytes = match value {
        // SAFETY: the value holds the pin on the frozen heap `ptr` points into,
        // which is never mutated again.
        MonoValue::Write { ptr, .. } => Some(
            unsafe { serialize(guard, ptr.as_ptr(), key.value_ty()) }
                .map(Bytes::from)
                .map_err(|e| {
                    code_invariant_error(format!("Failed to serialize a group member: {e}"))
                })?,
        ),
        MonoValue::Deletion => None,
        MonoValue::GroupMetadata { .. } => {
            return Err(code_invariant_error(format!(
                "Group member {key:?} holds group metadata"
            )))
        },
    };
    Ok((tag, bytes))
}

/// The view of `metadata`, which committed transaction `writer_idx` wrote,
/// built on top of `prev` unless it is built already.
fn build_committed_view(
    guard: &ExecutionGuard<'_>,
    data: &VersionedData<InMemoryStorageKey, MonoValue>,
    metadata: &GroupMetadata<InMemoryStorageKey>,
    prev: &GroupView,
    writer_idx: TxnIndex,
) -> Result<Arc<GroupView>, PanicError> {
    if let Some(view) = metadata.current() {
        return Ok(view);
    }
    let changes = metadata
        .changes()
        .iter()
        .map(|key| member_change(guard, key, &committed_value(data, key, writer_idx)?))
        .collect::<Result<Vec<_>, PanicError>>()?;
    Ok(metadata.build_view(prev, changes))
}

/// The group's view just before `txn_idx`. Builds the views of the committed
/// metadata entries below it that are not built yet.
fn view_before(
    guard: &ExecutionGuard<'_>,
    data: &VersionedData<InMemoryStorageKey, MonoValue>,
    group_key: &InMemoryStorageKey,
    txn_idx: TxnIndex,
) -> Result<Arc<GroupView>, PanicError> {
    let (mut view, unbuilt) = unbuilt_metadata_before(data, group_key, txn_idx)?;
    for (writer_idx, metadata) in unbuilt {
        view = build_committed_view(guard, data, &metadata, &view, writer_idx)?;
    }
    Ok(view)
}

/// A per-worker MonoMove executor.
pub struct MonoTransactionExecutor {
    ctx: Arc<GlobalContext>,
    /// The block's environment (features, configs).
    environment: AptosEnvironment,
    /// This executor's worker ID.
    worker_id: u32,
    /// Where this worker materializes the values it reads from storage.
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
    fn run<'g, S: TStateView<Key = StateKey> + Sync, P: ResourceProvider>(
        &self,
        guard: &'g ExecutionGuard<'g>,
        base_view: &S,
        data_provider: &P,
        txn: &SignatureVerifiedTransaction,
        auxiliary_info: &AuxiliaryInfo,
    ) -> Result<(TxnOutcome, bool), PanicError> {
        // A failed signature verification discards this transaction only, so a
        // block carrying one still executes the rest.
        let SignatureVerifiedTransaction::Valid(inner_txn) = txn else {
            return Ok((
                TxnOutcome::Discarded {
                    reason: DiscardReason::InvalidSignature,
                    effects: None,
                },
                false,
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

        if let TxnOutcome::Panic(err) = &outcome {
            return Err(code_invariant_error(format!(
                "MonoMove: could not close the session: {err:?}"
            )));
        }

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
    type Input = MonoReads;
    type Key = InMemoryStorageKey;
    type Output = MonoTxnOutput;
    type Tag = ();
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

        let (outcome, skips_rest, incarnation, unsync_map) = match mode {
            ViewMode::Sequential { unsync_map, .. } => {
                let provider = BlockSTMSequentialProvider::new(
                    &guard,
                    shared.base_view,
                    unsync_map,
                    &self.arena,
                );
                let (outcome, skips_rest) =
                    self.run(&guard, shared.base_view, &provider, txn, auxiliary_info)?;
                (outcome, skips_rest, None, Some(unsync_map))
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
                (outcome, skips_rest, Some(incarnation), None)
            },
        };

        // The read set validates from the outcome's read-write set, so it shares
        // the outcome with the output. The outcome is neither `Send` nor `Sync`;
        // both vouch for that.
        #[allow(clippy::arc_with_non_send_sync)]
        let outcome = Arc::new(outcome);
        let reads = match incarnation {
            Some(incarnation) => MonoReads::new(outcome.clone(), incarnation),
            // Sequential execution never validates, so the read set is not
            // collected.
            None => MonoReads::empty(None),
        };

        let output = MonoTxnOutput::executed(outcome, skips_rest);
        // Sequential execution applies this transaction's writes before it is
        // materialized, replacing the previous metadata at each group's slot.
        // That entry's view is already built, so this transaction's entry keeps
        // it.
        if let Some(unsync_map) = unsync_map {
            for (group_key, metadata) in output.group_metadata() {
                let prev = match unsync_map.fetch_data(group_key) {
                    Some(MonoValue::GroupMetadata { group }) => group.current(),
                    Some(MonoValue::Write { .. } | MonoValue::Deletion) | None => None,
                }
                .ok_or_else(|| {
                    code_invariant_error(format!(
                        "Group {group_key:?} was written before it was read"
                    ))
                })?;
                metadata.set_before(prev)?;
            }
        }

        Ok((ExecutionStatus::Executed { output, skips_rest }, reads))
    }

    fn materialize<S: TStateView<Key = <Self::Txn as Transaction>::Key> + Sync>(
        &self,
        output: Self::Output,
        _input: &Self::Input,
        _shared: SharedViewArgs<'_, S>,
        mode: ViewMode<'_, Self::Input>,
        txn_idx: TxnIndex,
    ) -> Result<
        <Self::Output as TxnOutput>::CommittedOutput,
        PanicOr<ResourceGroupSerializationError>,
    > {
        let (outcome, group_metadata) = match output {
            MonoTxnOutput::Executed {
                outcome,
                group_metadata,
                ..
            } => (outcome, group_metadata),
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
        // TODO(metering): the change set is not run through
        //   `ChangeSetConfigs::check_change_set`, so the per-transaction write,
        //   event and table-item caps are not enforced.
        let materialization_failed =
            |e| PanicOr::CodeInvariantError(format!("Failed to materialize outputs: {e}"));

        // Group members are left out: their groups are assembled below from
        // the metadata entries this transaction wrote.
        let partial = outcome
            .materialize_partial(&guard, features, auxiliary_data)
            .map_err(materialization_failed)?;

        let mut group_writes = Vec::with_capacity(group_metadata.len());
        for (group_key, metadata) in &group_metadata {
            let (prev, view) = match &mode {
                ViewMode::Sequential { unsync_map, .. } => {
                    let prev = metadata.before().cloned().ok_or_else(|| {
                        code_invariant_error("Sequential group metadata has no previous view")
                    })?;
                    // Sequential execution applied this transaction's writes
                    // before materializing it, and no later one has run yet.
                    let changes = metadata
                        .changes()
                        .iter()
                        .map(|key| {
                            let value = unsync_map.fetch_data(key).ok_or_else(|| {
                                code_invariant_error(format!(
                                    "No write at {key:?} in sequential execution"
                                ))
                            })?;
                            member_change(&guard, key, &value)
                        })
                        .collect::<Result<Vec<_>, PanicError>>()?;
                    let view = metadata.build_view(&prev, changes);
                    (prev, view)
                },
                ViewMode::Parallel { versioned_map, .. } => {
                    let data = versioned_map.data();
                    let prev = view_before(&guard, data, group_key, txn_idx)?;
                    let view = build_committed_view(&guard, data, metadata, &prev, txn_idx)?;
                    finish_group_post_commit(data, group_key, txn_idx, metadata)?;
                    (prev, view)
                },
            };
            let state_key = group_key
                .as_state_key()
                .map_err(|e| code_invariant_error(format!("Failed to build a state key: {e:#}")))?;
            group_writes.push((state_key, view.write_op(!prev.is_empty())?));
        }
        partial
            .with_group_writes(group_writes)
            .map_err(materialization_failed)
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

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_mvhashmap::MVHashMap;

    #[test]
    fn speculative_failure_fails_validation() {
        // BlockSTMv1 marks a doomed transaction this way and then relies on
        // validation to fail so the transaction re-executes.
        let map = MVHashMap::<InMemoryStorageKey, (), MonoValue, DelayedFieldID>::new();
        let mut reads = MonoReads::empty(Some(0));
        assert!(reads.validate_data_reads(map.data(), 0));
        assert!(reads.validate_group_reads(map.group_data(), 0));

        reads.record_delayed_field_application_failure();
        assert!(!reads.validate_data_reads(map.data(), 0));
        assert!(!reads.validate_group_reads(map.group_data(), 0));
    }
}
