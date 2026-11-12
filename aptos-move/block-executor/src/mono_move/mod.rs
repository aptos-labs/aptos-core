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
use group_chain::{
    committed_member_value, group_write, view_below, GroupCodec, GroupLink, MonoGroupCodec,
};
use mono_move_aptos_state_view_providers::StateViewModuleProvider;
use mono_move_aptos_transaction_executor::{
    production_natives, AptosTransactionExecutor, DiscardReason, TxnOutcome,
};
use mono_move_core::storage::resource_provider::{InMemoryStorageKey, ReadPin, ResourceProvider};
use mono_move_global_context::{ExecutionGuard, GlobalContext};
use mono_move_runtime::{SegmentedArena, SessionEffects, WriteClass};
use move_core_types::language_storage::ModuleId;
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    ptr::NonNull,
    sync::Arc,
};

pub(crate) mod group_chain;
mod provider;
mod reads;

use provider::{BlockSTMParallelProvider, BlockSTMSequentialProvider};
/// The combinatorial tests drive the speculative read loops with their own key
/// and tag types.
#[cfg(test)]
pub(crate) use provider::{ParallelReader, StorageBase};
pub use reads::{MonoReads, ReadSet, SessionReads};

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
    /// An entry at a resource group's own slot: the stored group, or one
    /// transaction's changes to it. Group members live under their own keys.
    //
    // TODO(completeness): add state value metadata here.
    GroupLink(Arc<GroupLink>),
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
        // MonoMove versions no group metadata. False here only forces
        // re-validation, so safe to use.
        false
    }

    fn bytes_len(&self) -> Option<usize> {
        // TODO(cleanup): this is only used for memory logging, revisit.
        match self {
            // A heap value's serialized size is unknown until materialization.
            MonoValue::Write { .. } => None,
            MonoValue::Deletion => None,
            MonoValue::GroupLink(_) => None,
        }
    }

    fn write_op_kind(&self) -> WriteOpKind {
        match self {
            MonoValue::Write { kind, .. } => kind.clone(),
            MonoValue::GroupLink(_) => WriteOpKind::Modification,
            MonoValue::Deletion => WriteOpKind::Deletion,
        }
    }
}

/// Builds the map value for a write, pinned to the heap it lives in.
fn written_value(class: WriteClass, pin: &Arc<dyn ReadPin>) -> MonoValue {
    match class {
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
        /// Writes keyed by their own storage slot. Resource-group members
        /// are keyed like plain resources, and each group with a written
        /// member also gets a [`MonoValue::GroupLink`] at its own slot.
        resource_writes: HashMap<InMemoryStorageKey, MonoValue>,
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
        TxnOutcome::Executed { effects, .. } => Some(effects.as_ref()),
        TxnOutcome::Discarded { .. } => None,
        TxnOutcome::ExecutedNoEffects(_) => None,
        // TODO(correctness): Revisit this arm: unexpected system txn errors
        //   should be handled at execution time!
        TxnOutcome::UnexpectedSystemTransactionFailure(_) => None,
        TxnOutcome::Panic(_) => None,
    }
}

impl MonoTxnOutput {
    /// Collects the transaction's writes. The block executor reads them back
    /// several times per commit, so the effects are walked once here.
    fn executed(outcome: TxnOutcome, skips_rest: bool) -> Self {
        let mut resource_writes = HashMap::new();

        if let Some(effects) = committed_effects(&outcome) {
            let pin: Arc<dyn ReadPin> = effects.frozen_heap();
            let mut changed_groups: HashMap<_, Vec<_>> = HashMap::new();
            for (key, class, group) in effects.read_write_set().writes_unordered() {
                if let Some(group_ty) = group {
                    changed_groups
                        .entry(InMemoryStorageKey::resource_group(key.address(), group_ty))
                        .or_default()
                        .push(key.value_ty());
                }
                resource_writes.insert(key.clone(), written_value(class, &pin));
            }
            for (group_key, changed) in changed_groups {
                resource_writes.insert(
                    group_key,
                    MonoValue::GroupLink(Arc::new(GroupLink::changed(changed))),
                );
            }
        }

        MonoTxnOutput::Executed {
            outcome,
            skips_rest,
            resource_writes,
        }
    }

    /// The group links this transaction wrote, keyed by group slot.
    fn group_links(&self) -> Vec<(InMemoryStorageKey, Arc<GroupLink>)> {
        let MonoTxnOutput::Executed {
            resource_writes, ..
        } = self
        else {
            return vec![];
        };
        resource_writes
            .iter()
            .filter_map(|(key, value)| match value {
                MonoValue::GroupLink(link) => Some((key.clone(), link.clone())),
                MonoValue::Write { .. } | MonoValue::Deletion => None,
            })
            .collect()
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
        // Group members are written as plain resources.
        HashMap::new()
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
            MonoTxnOutput::Executed { outcome, .. } => match outcome {
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

        let (outcome, skips_rest, reads, unsync_map) = match mode {
            ViewMode::Sequential { unsync_map, .. } => {
                let provider = BlockSTMSequentialProvider::new(
                    &guard,
                    shared.base_view,
                    unsync_map,
                    &self.arena,
                );
                let (outcome, skips_rest) =
                    self.run(&guard, shared.base_view, &provider, txn, auxiliary_info)?;
                // Sequential execution never validates, so the read set is
                // not collected.
                (
                    outcome,
                    skips_rest,
                    MonoReads::empty(None),
                    Some(unsync_map),
                )
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
                let reads = match outcome.effects() {
                    Some(effects) => MonoReads::new(SessionReads(effects.clone()), incarnation),
                    None => MonoReads::empty(Some(incarnation)),
                };
                (outcome, skips_rest, reads, None)
            },
        };

        let output = MonoTxnOutput::executed(outcome, skips_rest);
        // Sequential execution applies this transaction's writes before it is
        // materialized, replacing the previous link at each group's slot. That
        // link's view is already built, so this transaction's link keeps it.
        if let Some(unsync_map) = unsync_map {
            for (group_key, link) in output.group_links() {
                let prev = match unsync_map.fetch_data(&group_key) {
                    Some(MonoValue::GroupLink(prev)) => prev.built_view().cloned(),
                    Some(MonoValue::Write { .. } | MonoValue::Deletion) | None => None,
                }
                .ok_or_else(|| {
                    code_invariant_error(format!(
                        "Group {group_key:?} was written before it was read"
                    ))
                })?;
                link.set_prev(prev)?;
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
        let links = output.group_links();
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
        // TODO(metering): the change set is not run through
        //   `ChangeSetConfigs::check_change_set`, so the per-transaction write,
        //   event and table-item caps are not enforced.
        let materialization_failed =
            |e| PanicOr::CodeInvariantError(format!("Failed to materialize outputs: {e}"));

        // Group members are left out: their groups are assembled below from
        // the links this transaction wrote.
        let partial = outcome
            .materialize_partial(&guard, features, auxiliary_data)
            .map_err(materialization_failed)?;

        let codec = MonoGroupCodec { guard: &guard };
        let mut group_writes = Vec::with_capacity(links.len());
        for (group_key, link) in &links {
            let (prev, view) = match &mode {
                ViewMode::Sequential { unsync_map, .. } => {
                    let prev = link.prev().cloned().ok_or_else(|| {
                        code_invariant_error("A sequential group link has no previous view")
                    })?;
                    // Sequential execution applied this transaction's writes
                    // before materializing it, and no later one has run yet.
                    let view = link.view_on(&codec, &prev, |member_ty| {
                        unsync_map
                            .fetch_data(&codec.member_key(group_key, member_ty))
                            .ok_or_else(|| {
                                code_invariant_error(format!(
                                    "No write for a member of {group_key:?} in sequential \
                                     execution"
                                ))
                            })
                    })?;
                    (prev, view)
                },
                ViewMode::Parallel { versioned_map, .. } => {
                    let data = versioned_map.data();
                    let prev = view_below(data, &codec, group_key, txn_idx)?;
                    let view = link.view_on(&codec, &prev, |member_ty| {
                        committed_member_value(data, &codec, group_key, member_ty, txn_idx)
                    })?;
                    (prev, view)
                },
            };
            let state_key = group_key
                .as_state_key()
                .map_err(|e| code_invariant_error(format!("Failed to build a state key: {e:#}")))?;
            group_writes.push((state_key, group_write(!prev.is_empty(), &view)?));
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
