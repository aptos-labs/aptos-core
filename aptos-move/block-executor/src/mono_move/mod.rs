// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Implementation of single transaction executor for MonoMove.

use crate::{
    errors::ResourceGroupSerializationError,
    executor_utilities::Materializer,
    single_transaction_executor::{
        SharedViewArgs, SingleTransactionExecutor, ViewMode, MEMBER_GROUP_WRITES,
    },
    task::{ExecutionStatus, TxnOutput},
    types::InputOutputKey,
};
use aptos_aggregator::delayed_change::DelayedChange;
use aptos_infallible::Mutex;
use aptos_mvhashmap::types::{MVDataOutput, TxnIndex};
use aptos_types::{
    block_executor::value::SpeculativeValue,
    error::{code_invariant_error, PanicError, PanicOr},
    fee_statement::FeeStatement,
    state_store::{state_key::StateKey, state_value::StateValue, TStateView},
    transaction::{
        signature_verified_transaction::SignatureVerifiedTransaction, AuxiliaryInfo,
        BlockExecutableTransaction as Transaction, Transaction as AptosTransaction,
        TransactionAuxiliaryData, TransactionOutput,
    },
    write_set::{WriteOp, WriteOpKind, WriteSet},
};
use aptos_vm_environment::environment::AptosEnvironment;
use aptos_vm_types::resolver::ResourceGroupSize;
use bytes::Bytes;
use mono_move_aptos_state_view_providers::StateViewModuleProvider;
use mono_move_aptos_transaction_executor::{
    production_natives, AptosDataProvider, AptosTransactionExecutor, DiscardReason, GroupMembers,
    TxnOutcome,
};
use mono_move_core::{
    nominal_tag,
    storage::resource_provider::{InMemoryStorageKey, ReadPin},
    types::InternedType,
};
use mono_move_global_context::{ExecutionGuard, GlobalContext};
use mono_move_runtime::{serialize, SegmentedArena, SessionEffects, WriteClass};
use move_core_types::language_storage::{ModuleId, StructTag};
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
use once_cell::sync::Lazy;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    ptr::NonNull,
    sync::{Arc, OnceLock},
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
    GroupMetadata {
        /// The group's members as the writing transaction's materialization
        /// assembled them, once it has. A later writer of the group merges
        /// on top of this instead of collecting and serializing every member
        /// again.
        assembled: Arc<OnceLock<Arc<GroupMembers>>>,
    },
}

impl MonoValue {
    pub(crate) fn group_metadata() -> Self {
        MonoValue::GroupMetadata {
            assembled: Arc::new(OnceLock::new()),
        }
    }
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
            (
                MonoValue::GroupMetadata { .. },
                MonoValue::GroupMetadata { .. }
            )
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
            MonoValue::GroupMetadata { .. } => None,
        }
    }

    fn write_op_kind(&self) -> WriteOpKind {
        match self {
            MonoValue::Write { kind, .. } => kind.clone(),
            MonoValue::RawFromStorage(_) | MonoValue::GroupMetadata { .. } => {
                WriteOpKind::Modification
            },
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
        /// Writes that land in a storage slot of their own.
        resource_writes: HashMap<InMemoryStorageKey, MonoValue>,
        /// Writes that land inside a resource group, keyed by the group's own
        /// storage slot and tagged by the member's struct tag.
        group_writes: HashMap<InMemoryStorageKey, BTreeMap<StructTag, MonoValue>>,
        /// Every storage slot the transaction read, for hot state promotion. A
        /// group member's read counts for the group's own slot, which is what
        /// storage holds. May repeat keys; the accumulator dedups.
        read_keys: Vec<InMemoryStorageKey>,
        /// Only a block epilogue carries this: the keys its payload asks to
        /// make hot. Taken from the payload rather than recomputed, so a
        /// replay promotes the same keys the block's execution decided on.
        to_make_hot: Option<BTreeSet<StateKey>>,
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

/// The frozen effects of a committed, executed transaction, or `None` for a
/// discard or an empty-effects commit (both write nothing).
fn committed_effects(outcome: &TxnOutcome) -> Option<&SessionEffects> {
    match outcome {
        TxnOutcome::Executed { effects, .. } => Some(effects),
        TxnOutcome::Discarded { .. } => None,
        TxnOutcome::ExecutedNoEffects { .. } => None,
        // TODO(correctness): Revisit this arm: unexpected system txn errors
        //   should be handled at execution time!
        TxnOutcome::UnexpectedSystemTransactionFailure(_) => None,
        TxnOutcome::Panic(_) => None,
    }
}

impl MonoTxnOutput {
    /// Splits the transaction's writes into flat storage slots and
    /// resource-group members. The block executor reads both back several times
    /// per commit, so the effects are walked once here.
    ///
    /// A group member whose type has no struct tag cannot be written to storage
    /// at all, so it is left out; materialization reports it as a failure.
    fn executed(
        outcome: TxnOutcome,
        skips_rest: bool,
        to_make_hot: Option<BTreeSet<StateKey>>,
    ) -> Self {
        let mut resource_writes = HashMap::new();
        let mut group_writes: HashMap<_, BTreeMap<_, _>> = HashMap::new();
        let mut read_keys = Vec::new();

        if let Some(effects) = committed_effects(&outcome) {
            let pin: Arc<dyn ReadPin> = effects.frozen_heap();
            read_keys.extend(
                effects
                    .read_write_set()
                    .reads_unordered()
                    .map(|(key, _, group)| match group {
                        None => key.clone(),
                        Some(group_ty) => {
                            InMemoryStorageKey::resource_group(key.address(), group_ty)
                        },
                    }),
            );
            for (key, class, group) in effects.read_write_set().writes_unordered() {
                let ty = key.value_ty();
                let value = written_value(class, ty, &pin);
                match group {
                    None => {
                        resource_writes.insert(key.clone(), value);
                    },
                    Some(group_ty) => {
                        let Ok(tag) = nominal_tag(ty) else {
                            continue;
                        };
                        group_writes
                            .entry(InMemoryStorageKey::resource_group(key.address(), group_ty))
                            .or_default()
                            .insert(tag, value);
                    },
                }
            }
        }

        MonoTxnOutput::Executed {
            outcome,
            skips_rest,
            resource_writes,
            group_writes,
            read_keys,
            to_make_hot,
        }
    }
}

/// The hot state promotions a block epilogue's output carries: the keys its
/// payload names, plus the slots the epilogue itself read without writing,
/// which the payload was computed too early to include.
fn epilogue_hotness(
    to_make_hot: BTreeSet<StateKey>,
    read_keys: &[InMemoryStorageKey],
    written: &WriteSet,
) -> Result<BTreeSet<StateKey>, PanicError> {
    let mut hotness = to_make_hot;
    for key in read_keys {
        let state_key = key
            .as_state_key()
            .map_err(|e| code_invariant_error(format!("Failed to build a state key: {e:#}")))?;
        if written.get_write_op(&state_key).is_none() {
            hotness.insert(state_key);
        }
    }
    Ok(hotness)
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
        match self {
            MonoTxnOutput::Executed {
                resource_writes, ..
            } => resource_writes.clone(),
            MonoTxnOutput::SkippedToRetry => HashMap::new(),
        }
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
        let MonoTxnOutput::Executed { group_writes, .. } = self else {
            return HashMap::new();
        };
        group_writes
            .iter()
            .map(|(group_key, members)| {
                (
                    group_key.clone(),
                    (
                        MonoValue::group_metadata(),
                        // Every `MonoValue` reports no serialized length, so
                        // this is also what the map computes for the group.
                        ResourceGroupSize::zero_combined(),
                        members.clone(),
                    ),
                )
            })
            .collect()
    }

    fn for_each_resource_key(
        &self,
        callback: &mut dyn FnMut(&Self::Key) -> Result<(), PanicError>,
    ) -> Result<(), PanicError> {
        let MonoTxnOutput::Executed {
            resource_writes, ..
        } = self
        else {
            return Ok(());
        };
        for key in resource_writes.keys() {
            callback(key)?;
        }
        Ok(())
    }

    fn for_each_resource_group_key_and_tags(
        &self,
        callback: &mut dyn FnMut(&Self::Key, HashSet<&Self::Tag>) -> Result<(), PanicError>,
    ) -> Result<(), PanicError> {
        let MonoTxnOutput::Executed { group_writes, .. } = self else {
            return Ok(());
        };
        for (group_key, members) in group_writes {
            callback(group_key, members.keys().collect())?;
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
        let read_keys = match self {
            MonoTxnOutput::Executed { read_keys, .. } => Some(read_keys),
            MonoTxnOutput::SkippedToRetry => None,
        };
        read_keys.into_iter().flatten()
    }

    fn storage_keys_written(&self) -> impl Iterator<Item = &Self::Key> {
        // A group member's write lands in the group's slot, so that is the key
        // the hot state sees written.
        let writes = match self {
            MonoTxnOutput::Executed {
                resource_writes,
                group_writes,
                ..
            } => Some(resource_writes.keys().chain(group_writes.keys())),
            MonoTxnOutput::SkippedToRetry => None,
        };
        writes.into_iter().flatten()
    }

    fn check_materialization(&self, _materializer: &impl Materializer<Self::Txn>) -> bool {
        // TODO(security): can add extra checks here?
        true
    }
}

/// Whether MonoMove charges gas. Off by default while its gas schedule is a
/// placeholder; `MONO_MOVE_METERED=1` turns it on for experiments, which
/// changes what a transaction writes (fee, burn, supply) and so its output
/// size.
static METERED: Lazy<bool> = Lazy::new(|| std::env::var_os("MONO_MOVE_METERED").is_some());

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
        let executor = AptosTransactionExecutor::new(
            guard,
            natives,
            &module_provider,
            data_provider,
            &self.environment,
            usage,
        );
        let executor = if *METERED {
            executor
        } else {
            executor.without_metering()
        };
        let outcome = executor.execute_transaction(inner_txn, auxiliary_info);

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

        let to_make_hot = match txn {
            SignatureVerifiedTransaction::Valid(AptosTransaction::BlockEpilogue(payload)) => {
                payload.try_get_keys_to_make_hot().cloned()
            },
            SignatureVerifiedTransaction::Valid(_) | SignatureVerifiedTransaction::Invalid(_) => {
                None
            },
        };

        Ok((
            ExecutionStatus::Executed {
                output: MonoTxnOutput::executed(outcome, skips_rest, to_make_hot),
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
        let (outcome, read_keys, to_make_hot) = match output {
            MonoTxnOutput::Executed {
                outcome,
                read_keys,
                to_make_hot,
                ..
            } => (outcome, read_keys, to_make_hot),
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

        let output = match mode {
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
                output
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
                let (output, groups) = outcome
                    .materialize_with(
                        &guard,
                        &provider,
                        features,
                        auxiliary_data,
                        *MEMBER_GROUP_WRITES,
                    )
                    .map_err(materialization_failed)?;
                // Hand each assembled group to this transaction's entry at the
                // group's slot, for the next writer of the group to merge on.
                for (group_key, members) in groups {
                    if let Ok(MVDataOutput::Versioned(_, MonoValue::GroupMetadata { assembled })) =
                        versioned_map
                            .data()
                            .fetch_data_no_record(&group_key, txn_idx + 1)
                    {
                        let _ = assembled.set(Arc::new(members.unwrap_or_default()));
                    }
                }
                output
            },
        };

        let Some(to_make_hot) = to_make_hot else {
            return Ok(output);
        };
        let (mut write_set, events, gas_used, status, auxiliary_data) = output.unpack();
        write_set.add_hotness(epilogue_hotness(to_make_hot, &read_keys, &write_set)?);
        Ok(TransactionOutput::new(
            write_set,
            events,
            gas_used,
            status,
            auxiliary_data,
        ))
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

    fn supports_member_group_writes(&self) -> bool {
        true
    }

    fn uses_execution_pool() -> bool {
        true
    }

    fn member_storage_key(
        &self,
        group_key: &InMemoryStorageKey,
        tag: &StructTag,
    ) -> Result<StateKey, PanicError> {
        StateKey::resource(&group_key.address(), tag)
            .map_err(|e| code_invariant_error(format!("Failed to build a member key: {e:#}")))
    }

    fn finalize_group_writes<S: TStateView<Key = StateKey> + Sync>(
        &self,
        group_keys: Vec<InMemoryStorageKey>,
        _shared: SharedViewArgs<'_, S>,
        mode: ViewMode<'_, Self::Input>,
        next_txn_idx: TxnIndex,
    ) -> Result<Vec<(StateKey, WriteOp)>, PanicError> {
        let ViewMode::Parallel { versioned_map, .. } = mode else {
            return Err(code_invariant_error(
                "Group writes are only assembled after parallel execution",
            ));
        };
        let group_data = versioned_map.group_data();
        let assemble = |guard: &ExecutionGuard<'_>,
                        group_key: InMemoryStorageKey|
         -> Result<Option<(StateKey, WriteOp)>, PanicError> {
            let existed = !group_data.group_members_at(&group_key, 0)?.is_empty();
            let members = group_data.group_members_at(&group_key, next_txn_idx)?;
            let blob = if members.is_empty() {
                None
            } else {
                let members = members
                    .into_iter()
                    .map(|(tag, value)| {
                        let bytes = match value {
                            MonoValue::RawFromStorage(bytes) => bytes,
                            MonoValue::Write { ptr, ty, .. } => {
                                // SAFETY: the entry holding this pointer pins the frozen
                                // heap it points into, and that heap is never mutated.
                                let blob =
                                    unsafe { serialize(guard, ptr.as_ptr(), ty) }.map_err(|e| {
                                        code_invariant_error(format!(
                                            "Failed to serialize a group member: {e}"
                                        ))
                                    })?;
                                Bytes::from(blob)
                            },
                            MonoValue::Deletion | MonoValue::GroupMetadata { .. } => {
                                return Err(code_invariant_error(
                                    "A deleted member or group metadata was assembled",
                                ))
                            },
                        };
                        Ok((tag, bytes))
                    })
                    .collect::<Result<BTreeMap<_, _>, PanicError>>()?;
                let blob = bcs::to_bytes(&members)
                    .map_err(|e| code_invariant_error(format!("Failed to encode a group: {e}")))?;
                Some(Bytes::from(blob))
            };
            let op = match (existed, blob) {
                (false, Some(bytes)) => WriteOp::legacy_creation(bytes),
                (true, Some(bytes)) => WriteOp::legacy_modification(bytes),
                (true, None) => WriteOp::legacy_deletion(),
                (false, None) => return Ok(None),
            };
            let state_key = group_key
                .as_state_key()
                .map_err(|e| code_invariant_error(format!("Failed to build a state key: {e:#}")))?;
            Ok(Some((state_key, op)))
        };
        // The workers have returned their arenas, so each chunk takes one: a
        // block writes thousands of groups and assembling them serially would
        // cost more than the per-transaction assembly it replaces.
        let ctx = &self.ctx;
        let num_workers = ctx.num_execution_workers().max(1);
        let chunk_len = group_keys.len().div_ceil(num_workers).max(1);
        let chunks = group_keys.chunks(chunk_len).collect::<Vec<_>>();
        let results = Mutex::new(Vec::with_capacity(chunks.len()));
        rayon::scope(|scope| {
            for (worker_id, chunk) in chunks.into_iter().enumerate() {
                let results = &results;
                let assemble = &assemble;
                scope.spawn(move |_| {
                    let chunk_writes = ctx
                        .try_execution_context(worker_id)
                        .ok_or_else(|| code_invariant_error("No free arena to assemble groups"))
                        .and_then(|guard| {
                            chunk
                                .iter()
                                .filter_map(|key| assemble(&guard, key.clone()).transpose())
                                .collect::<Result<Vec<_>, PanicError>>()
                        });
                    results.lock().push((worker_id, chunk_writes));
                });
            }
        });
        let mut results = results.into_inner();
        results.sort_by_key(|(worker_id, _)| *worker_id);
        let mut writes = Vec::with_capacity(group_keys.len());
        for (_, chunk_writes) in results {
            writes.extend(chunk_writes?);
        }
        Ok(writes)
    }
}
