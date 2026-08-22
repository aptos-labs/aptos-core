// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! MonoMove integration into Block-STM.
//!
//! This wires the MonoMove VM in as a [`SingleTransactionExecutor`] so a block
//! runs through the existing Block-STM loop (scheduler, multi-version map,
//! validation, commit) with `InMemoryStorageKey` / [`MonoValue`] as the map
//! key/value. The dispatch (in `aptos-vm`) selects it via the on-chain
//! `FeatureFlag::ENABLE_MONO_MOVE`.
//!
//! Milestone 1 is deliberately narrow: the sequential Block-STM path only
//! (`ViewMode::Sequential`, `UnsyncMap`), own-slot resources, table items, and
//! resource groups (no delayed fields or module publishing).
//! [`execute`](MonoTransactionExecutor::execute) reads through a
//! [`BlockSTMProvider`] (the multi-version map, then the base view) and runs the
//! single-transaction driver `AptosTransactionExecutor`, retaining the whole
//! [`TxnOutcome`] so [`materialize`](MonoTransactionExecutor::materialize) can
//! render it lazily at commit. Reads are not validated, so the dispatch site
//! forces `concurrency_level = 1`; the parallel arm is unimplemented.

use crate::{
    captured_reads::TxnInput,
    code_cache_global_manager::LegacyModuleCache,
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
    on_chain_config::Features,
    state_store::{state_key::StateKey, state_value::StateValue, table::TableHandle, TStateView},
    transaction::{
        signature_verified_transaction::SignatureVerifiedTransaction, AuxiliaryInfo,
        BlockExecutableTransaction as Transaction, ExecutionStatus as AptosExecutionStatus,
        TransactionAuxiliaryData, TransactionOutput, TransactionStatus,
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
use mono_move_core::{storage::resource_provider::InMemoryStorageKey, struct_tag_of};
use mono_move_global_context::{ExecutionGuard, GlobalContext};
use mono_move_runtime::{serialize, SessionEffects, WriteClass};
use move_binary_format::CompiledModule;
use move_core_types::language_storage::{ModuleId, StructTag};
use move_vm_runtime::Module;
use move_vm_types::{code::SyncModuleCache, delayed_values::delayed_field_id::DelayedFieldID};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    ptr::NonNull,
};

mod provider;

use provider::BlockSTMProvider;

/// A value in the multi-version map: a MonoMove transaction write, or a base /
/// split value materialized from storage. Group members are first-class entries
/// here under their own resource keys, just like own-slot resources; the group
/// blob is reassembled only at materialization.
///
/// Both writes and base values point into the map's append-only resource arena.
/// A transaction's session heap is dropped when its output is materialized, so
/// `execute` copies each write into the arena before it returns; the pointer
/// then shares the map's lifetime and needs no pin.
#[derive(Clone)]
pub enum MonoValue {
    Write {
        ptr: NonNull<u8>,
        kind: WriteOpKind,
    },
    Deletion,
}

// SAFETY: The pointer addresses a value living inside the map's append-only
// resource arena, which is never mutated or GC'd and shares the map's lifetime.
// Sharing the value read-only across worker threads is therefore sound.
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

/// A MonoMove transaction's read set. Stub: records nothing yet, so validation
/// trivially passes. The real impl records `(InMemoryStorageKey -> version)` and
/// compares versions against the multi-version map.
#[derive(Default)]
pub struct MonoReads;

impl TxnInput for MonoReads {
    type Key = InMemoryStorageKey;
    // TODO(perf): can use InternedType here?
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
        _global_module_cache: &LegacyModuleCache,
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
        // MonoMove. Instead, block executor needs to drain pending requests
        // to publish code.
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

/// A MonoMove transaction's output as seen by Block-STM.
///
/// The whole [`TxnOutcome`] is retained so [`materialize`] can render it lazily
/// at commit. The map-facing writes are staged into the map's arena at
/// `execute` time (the session heap is dropped at materialize) and served from
/// there: every write is its own resource key (a group member is just a
/// `Resource` key, no group awareness), reassembled into a group blob only at
/// materialization. `Skipped` is the empty, kept-success output Block-STM asks
/// for via [`TxnOutput::skip_output`].
///
/// [`materialize`]: MonoTransactionExecutor::materialize
pub enum MonoTxnOutput {
    // TODO(perf): remove box.
    Committed(Box<CommittedMonoOutput>),
    Skipped,
}

/// The retained state of a committed MonoMove transaction.
pub struct CommittedMonoOutput {
    /// The frozen effects, rendered into a `TransactionOutput` at commit.
    outcome: TxnOutcome,
    /// The map-facing write set, staged into the map's arena at `execute` time.
    /// Each write is its own resource key (group members included); the pointers
    /// live in the map's arena, so they stay valid after the session heap is
    /// dropped at materialize.
    writes: HashMap<InMemoryStorageKey, MonoValue>,
}

// SAFETY: `CommittedMonoOutput` holds `SessionEffects` (via `outcome`), whose
// write pointers address values inside a frozen owned `Heap`, arena-backed
// `MonoValue`s (in `writes`), and interned types (in the read-write-set keys)
// living in the block's `GlobalContext` arena. All outlive the output, which is
// read only once produced, so sharing it across worker threads is sound -- the
// same justification as `MonoValue`.
unsafe impl Send for MonoTxnOutput {}
unsafe impl Sync for MonoTxnOutput {}

impl MonoTxnOutput {
    /// The frozen effects of a committed, executed transaction, or `None` for a
    /// discard or a skip (both write nothing).
    fn effects(&self) -> Option<&SessionEffects> {
        match self {
            MonoTxnOutput::Committed(c) => match &c.outcome {
                TxnOutcome::Executed { effects, .. } => Some(effects),
                TxnOutcome::Discarded(_) => None,
            },
            MonoTxnOutput::Skipped => None,
        }
    }

    /// The keys this transaction wrote to the map, in a nondeterministic order.
    /// A group member is its own `Resource` key. Callers reaching consensus
    /// must lower these to `StateKey` first.
    fn write_keys(&self) -> impl Iterator<Item = &InMemoryStorageKey> {
        self.effects().into_iter().flat_map(|effects| {
            effects
                .read_write_set
                .writes_unordered()
                .map(|(key, _, _)| key)
        })
    }
}

impl TxnOutput for MonoTxnOutput {
    type CommittedOutput = TransactionOutput;
    type Key = InMemoryStorageKey;
    // TODO(perf): can use InternedType?
    type Tag = StructTag;
    type Txn = SignatureVerifiedTransaction;
    type Value = MonoValue;

    fn skip_output() -> Self {
        MonoTxnOutput::Skipped
    }

    fn resource_write_set(&self) -> HashMap<Self::Key, Self::Value> {
        // Staged into the map's arena at `execute` time. Every write is its own
        // resource key, including group members: the group blob is reassembled
        // only at materialization, from the transaction's own member ops. A
        // discard or skip has no effects and writes nothing.
        match self {
            MonoTxnOutput::Committed(c) => c.writes.clone(),
            MonoTxnOutput::Skipped => HashMap::new(),
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
        HashMap::new()
    }

    fn for_each_resource_key(
        &self,
        callback: &mut dyn FnMut(&Self::Key) -> Result<(), PanicError>,
    ) -> Result<(), PanicError> {
        for key in self.write_keys() {
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
        // MonoMove cannot publish modules - nothing to do.
        Ok(())
    }

    fn fee_statement(&self) -> FeeStatement {
        match self {
            MonoTxnOutput::Committed(c) => match &c.outcome {
                TxnOutcome::Executed { fee_statement, .. } => *fee_statement,
                TxnOutcome::Discarded(_) => FeeStatement::zero(),
            },
            MonoTxnOutput::Skipped => FeeStatement::zero(),
        }
    }

    fn has_new_epoch_event(&self) -> bool {
        // TODO(completeness): detect reconfiguration / new-epoch events so the
        // block halts. Own-slot resource txns this milestone never reconfigure.
        false
    }

    fn output_approx_size(&self) -> u64 {
        // TODO(metering): size the serialized writes for the block output limit.
        0
    }

    fn get_write_summary(&self) -> HashSet<InputOutputKey<Self::Key, Self::Tag>> {
        self.write_keys()
            .map(|key| InputOutputKey::Resource(key.clone()))
            .collect()
    }

    fn storage_keys_read(&self) -> impl Iterator<Item = &Self::Key> {
        // TODO(completeness): the sequential mono path captures no reads.
        std::iter::empty()
    }

    fn storage_keys_written(&self) -> impl Iterator<Item = &Self::Key> {
        self.write_keys()
    }

    fn check_materialization(&self, _materializer: &impl Materializer<Self::Txn>) -> bool {
        // TODO(security): can add extra checks here.
        true
    }
}

// ---------------------------------------------------------------------------
// MonoTransactionExecutor: the per-worker VM (SingleTransactionExecutor)
// ---------------------------------------------------------------------------

/// A per-worker MonoMove executor. Owns the worker's [`ExecutionGuard`] (its
/// arena shard), acquired from the block's [`GlobalContext`] at [`init`] and
/// held for the whole block.
///
/// TODO(correctness): Block-STM keeps worker 0's executor for post-commit
/// finalization/materialization, which may run on a different thread than the
/// one that acquired the guard. The `ExecutionGuard`'s arena mutex is
/// thread-affine (parking_lot), so real materialization must be reworked to
/// acquire/serialize on the owning worker thread. The stub does not touch the
/// arena during materialize, so this is inert for now.
pub struct MonoTransactionExecutor<'ctx> {
    guard: ExecutionGuard<'ctx>,
    /// The block's feature set, used by the driver's status mapping and (later)
    /// its gas configuration and pre-execution checks.
    features: Features,
    #[allow(dead_code)]
    worker_id: u32,
}

impl<'ctx> SingleTransactionExecutor<'ctx> for MonoTransactionExecutor<'ctx> {
    type AuxiliaryInfo = AuxiliaryInfo;
    type Input = MonoReads;
    type Key = InMemoryStorageKey;
    type Output = MonoTxnOutput;
    type Tag = StructTag;
    type Txn = SignatureVerifiedTransaction;
    type Value = MonoValue;

    fn init(
        environment: &AptosEnvironment,
        ctx: &'ctx GlobalContext,
        _state_view: &impl TStateView<Key = <Self::Txn as Transaction>::Key>,
        worker_id: u32,
        _async_runtime_checks_enabled: bool,
    ) -> Self {
        let guard = ctx
            .try_execution_context(worker_id as usize)
            .expect("worker arena shard must be free (one guard per worker)");
        Self {
            guard,
            features: environment.features().clone(),
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
        // The mono path runs only sequentially this milestone; reads are neither
        // captured nor validated (see the module docs and the dispatch gate).
        let ViewMode::Sequential { unsync_map, .. } = mode else {
            return Err(code_invariant_error(
                "MonoMove: parallel execution is unsupported this milestone",
            ));
        };

        // Only signed user transactions run here. Block metadata / state
        // checkpoints carry no payload and produce no MonoMove effects.
        let Some(signed_txn) = txn.try_as_signed_user_txn() else {
            return Err(code_invariant_error(
                "MonoMove: only signed user transactions are supported",
            ));
        };

        let data_provider = BlockSTMProvider::new(&self.guard, shared.base_view, unsync_map);
        let module_provider = StateViewModuleProvider::new(shared.base_view);
        let natives = production_natives(&self.guard);
        let usage = shared
            .base_view
            .get_usage()
            .map_err(|e| code_invariant_error(format!("MonoMove: state usage read failed: {e}")))?;

        let outcome = AptosTransactionExecutor::new(
            &self.guard,
            &natives,
            &module_provider,
            &data_provider,
            &self.features,
            usage,
        )
        .execute_user_transaction(signed_txn, auxiliary_info);

        // Stage the write set into the map's arena now: the session heap is
        // dropped when this output is materialized, so a write pointer into it
        // cannot back a later transaction's read. Serialize each write out of
        // the session heap and re-materialize it into the map's append-only
        // arena, whose pointers share the map's lifetime. Group members are
        // staged as their own resource keys and merged into the group blob by
        // the executor's `drain_write_set` at materialize time.
        //
        // A discard is a normal committed result (empty writes, discard status
        // at materialization), not a speculative failure, so return `Executed`.
        let writes = self.stage_writes(&data_provider, &outcome)?;

        Ok((
            ExecutionStatus::Executed {
                output: MonoTxnOutput::Committed(Box::new(CommittedMonoOutput { outcome, writes })),
                skips_rest: false,
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
        // Sequential only: `execute` and `materialize` share the worker thread,
        // so the guard held from `init` is valid here (foreign-thread parallel
        // materialization is M2, tracked at the executor's `TODO(correctness)`).
        let ViewMode::Sequential { unsync_map, .. } = mode else {
            return Err(PanicOr::CodeInvariantError(
                "MonoMove: parallel materialization is unsupported this milestone".to_string(),
            ));
        };

        let committed = match output {
            MonoTxnOutput::Committed(c) => *c,
            // A skipped output renders to an empty, kept-success transaction.
            MonoTxnOutput::Skipped => {
                return Ok(TransactionOutput::new(
                    WriteSet::default(),
                    vec![],
                    0,
                    TransactionStatus::Keep(AptosExecutionStatus::Success),
                    TransactionAuxiliaryData::None,
                ))
            },
        };

        // Materialization serializes each own-slot write via the guard's layout
        // and reassembles each touched group's blob from the transaction's own
        // member ops overlaid on the stored members. This relies on the
        // sequential loop's per-transaction interleaving: `group_members` reads
        // the base view only, so the reassembled blob is exactly the stored
        // group with this transaction's member ops applied.
        let data_provider = BlockSTMProvider::new(&self.guard, shared.base_view, unsync_map);

        // TODO(completeness): derive `TransactionAuxiliaryData` from the
        // execute-time `AuxiliaryInfo` rather than defaulting it.
        committed
            .outcome
            .materialize(
                &self.guard,
                &data_provider,
                &self.features,
                TransactionAuxiliaryData::default(),
            )
            .map_err(|e| {
                PanicOr::CodeInvariantError(format!("MonoMove: materialization failed: {e}"))
            })
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
                // to `StateKey::resource`, its own slot, not the enclosing
                // group slot. This key never reaches a committed write set
                // (materialization emits the group slot directly); it only feeds
                // the block epilogue's hot-state set, so the approximation is
                // inert unless hotness is tracked in the epilogue.
                // TODO(perf): use self.guard for the type-to-state-key conversion.
                let struct_tag = struct_tag_of(ty).ok_or_else(|| {
                    code_invariant_error("MonoMove: resource key type must be a nominal type")
                })?;
                // A too-deeply-nested resource type is a runtime limit reachable
                // with a sufficiently nested type, surfaced here as a panic error
                // because the epilogue key set cannot carry it further.
                StateKey::resource(&address, &struct_tag).map_err(|_| {
                    code_invariant_error(
                        "MonoMove: resource type nested too deeply for a state key",
                    )
                })
            },
            InMemoryStorageKey::TableItem { handle, key, .. } => {
                Ok(StateKey::table_item(&TableHandle(handle.address()), &key))
            },
        }
    }
}

impl MonoTransactionExecutor<'_> {
    /// Copies a committed transaction's writes out of its session heap into the
    /// map's append-only resource arena, returning the map-facing write set. The
    /// session heap is dropped at materialize, so the map cannot hold pointers
    /// into it; the arena's pointers share the map's lifetime instead. A discard
    /// has no effects and writes nothing.
    fn stage_writes<S: TStateView<Key = StateKey>>(
        &self,
        data_provider: &BlockSTMProvider<'_, '_, S>,
        outcome: &TxnOutcome,
    ) -> Result<HashMap<InMemoryStorageKey, MonoValue>, PanicError> {
        let TxnOutcome::Executed { effects, .. } = outcome else {
            return Ok(HashMap::new());
        };
        let mut writes = HashMap::new();
        for (key, class, _group) in effects.read_write_set.writes_unordered() {
            let value = match class {
                WriteClass::Creation(ptr) => {
                    let ptr = self.stage_value(data_provider, key, ptr)?;
                    MonoValue::Write {
                        ptr,
                        kind: WriteOpKind::Creation,
                    }
                },
                WriteClass::Modification(ptr) => {
                    let ptr = self.stage_value(data_provider, key, ptr)?;
                    MonoValue::Write {
                        ptr,
                        kind: WriteOpKind::Modification,
                    }
                },
                WriteClass::Deletion => MonoValue::Deletion,
            };
            writes.insert(key.clone(), value);
        }
        Ok(writes)
    }

    /// Serializes the value at `ptr` (in the session heap) and re-materializes
    /// it into the map's arena, returning the arena pointer.
    fn stage_value<S: TStateView<Key = StateKey>>(
        &self,
        data_provider: &BlockSTMProvider<'_, '_, S>,
        key: &InMemoryStorageKey,
        ptr: NonNull<u8>,
    ) -> Result<NonNull<u8>, PanicError> {
        // SAFETY: `ptr` addresses a live value in the transaction's frozen
        // session heap; no GC runs between execution and staging.
        let blob = unsafe { serialize(&self.guard, ptr.as_ptr(), key.value_ty()) }
            .map_err(|e| code_invariant_error(format!("MonoMove: failed to serialize a write: {e}")))?;
        data_provider
            .materialize_into_arena(key, &blob)
            .map_err(|e| code_invariant_error(format!("MonoMove: failed to stage a write: {e}")))
    }
}
