// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! MonoMove integration into Block-STM.
//!
//! This wires the MonoMove VM in as a [`SingleTransactionExecutor`] so a block
//! runs through the existing Block-STM loop (scheduler, multi-version map,
//! validation, commit) with `InMemoryStorageKey` / [`MonoValue`] as the map
//! key/value. The dispatch (in `aptos-vm`) selects it via the on-chain
//! `FeatureFlag::MONO_MOVE`.
//!
//! This is scaffolding: [`MonoTransactionExecutor::execute`] /
//! [`materialize`](MonoTransactionExecutor::materialize) are stubs that produce
//! an empty, successful output, so the mono path compiles and runs inert. The
//! real execution (resource provider, interpreter run, write/read extraction,
//! materialization via `mono_move_output::to_transaction_output`) is follow-up.

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
use mono_move_core::{storage::resource_provider::InMemoryStorageKey, struct_tag_of};
use mono_move_global_context::{ExecutionGuard, GlobalContext};
use mono_move_runtime::Heap;
use move_binary_format::CompiledModule;
use move_core_types::language_storage::{ModuleId, StructTag};
use move_vm_runtime::Module;
use move_vm_types::{code::SyncModuleCache, delayed_values::delayed_field_id::DelayedFieldID};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    ptr::NonNull,
    sync::Arc,
};

/// A value written by a MonoMove transaction, stored in the multi-version map.
#[derive(Clone)]
pub enum MonoValue {
    Write {
        ptr: NonNull<u8>,
        kind: WriteOpKind,
        /// A frozen transaction heap where the pointer is allocated.
        heap: Arc<Heap>,
    },
    Deletion,
}

// SAFETY: The pointer addresses a value living entirely inside `heap`, which is
// frozen (execution / base-value deserialization completed before publish) and
// never mutated or GC'd afterward. The `Arc<Heap>` carried alongside keeps that
// heap alive, so sharing the value read-only across worker threads is sound.
unsafe impl Send for MonoValue {}
unsafe impl Sync for MonoValue {}

impl SpeculativeValue for MonoValue {
    fn eq_value(&self, _other: &Self) -> bool {
        // False here only forces re-validation, so safe to use.
        false
    }

    fn eq_metadata(&self, _other: &Self) -> bool {
        false
    }

    fn bytes_len(&self) -> Option<usize> {
        // Serialized size is unknown until materialization.
        None
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
    type Tag = StructTag;
    type Value = MonoValue;

    fn validate_data_reads(
        &self,
        _data_map: &VersionedData<Self::Key, Self::Value>,
        _idx_to_validate: TxnIndex,
    ) -> bool {
        true
    }

    fn validate_group_reads(
        &self,
        _group_map: &VersionedGroupData<Self::Key, Self::Tag, Self::Value>,
        _idx_to_validate: TxnIndex,
    ) -> bool {
        true
    }

    fn validate_delayed_field_reads(
        &self,
        _delayed_fields: &dyn TVersionedDelayedFieldView<DelayedFieldID>,
        _idx_to_validate: TxnIndex,
    ) -> Result<bool, PanicError> {
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
        // Module publishing is unsupported on the mono path for now.
        true
    }

    fn record_delayed_field_application_failure(&mut self) {}

    fn incarnation(&self) -> Option<Incarnation> {
        None
    }

    fn is_incorrect_use(&self) -> bool {
        false
    }

    fn get_read_summary(&self) -> HashSet<InputOutputKey<Self::Key, Self::Tag>> {
        HashSet::new()
    }
}

/// A MonoMove transaction's output as seen by Block-STM. Stub: carries nothing
/// (empty write set, no events). The real impl holds the frozen `SessionEffects`
/// (writes as `MonoValue`, events) and materializes via
/// `mono_move_output::to_transaction_output`.
#[derive(Debug, Default)]
pub struct MonoTxnOutput;

impl TxnOutput for MonoTxnOutput {
    type CommittedOutput = TransactionOutput;
    type Key = InMemoryStorageKey;
    type Tag = StructTag;
    type Txn = SignatureVerifiedTransaction;
    type Value = MonoValue;

    fn skip_output() -> Self {
        MonoTxnOutput
    }

    fn resource_write_set(&self) -> HashMap<Self::Key, Self::Value> {
        HashMap::new()
    }

    fn delayed_field_change_set(&self) -> BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>> {
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
        _callback: &mut dyn FnMut(&Self::Key) -> Result<(), PanicError>,
    ) -> Result<(), PanicError> {
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
        Ok(())
    }

    fn fee_statement(&self) -> FeeStatement {
        FeeStatement::zero()
    }

    fn has_new_epoch_event(&self) -> bool {
        false
    }

    fn output_approx_size(&self) -> u64 {
        0
    }

    fn get_write_summary(&self) -> HashSet<InputOutputKey<Self::Key, Self::Tag>> {
        HashSet::new()
    }

    fn storage_keys_read(&self) -> impl Iterator<Item = &Self::Key> {
        std::iter::empty()
    }

    fn storage_keys_written(&self) -> impl Iterator<Item = &Self::Key> {
        std::iter::empty()
    }

    fn check_materialization(&self, _materializer: &impl Materializer<Self::Txn>) -> bool {
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
        _environment: &AptosEnvironment,
        ctx: &'ctx GlobalContext,
        _state_view: &impl TStateView<Key = <Self::Txn as Transaction>::Key>,
        worker_id: u32,
        _async_runtime_checks_enabled: bool,
    ) -> Self {
        let guard = ctx
            .try_execution_context(worker_id as usize)
            .expect("worker arena shard must be free (one guard per worker)");
        Self { guard, worker_id }
    }

    fn execute<S: TStateView<Key = <Self::Txn as Transaction>::Key> + Sync>(
        &self,
        _shared: SharedViewArgs<'_, S>,
        _mode: ViewMode<'_, Self::Input>,
        _txn: &Self::Txn,
        _auxiliary_info: &Self::AuxiliaryInfo,
        _txn_idx: TxnIndex,
    ) -> Result<(ExecutionStatus<Self::Output>, Self::Input), PanicError> {
        // TODO(completeness): build the block-stm resource provider, run the
        // entry function on a fresh heap via `self.guard`, and split the frozen
        // `SessionEffects` into `MonoTxnOutput` (writes) + `MonoReads` (versions).
        Ok((
            ExecutionStatus::Executed {
                output: MonoTxnOutput,
                skips_rest: false,
            },
            MonoReads,
        ))
    }

    fn materialize<S: TStateView<Key = <Self::Txn as Transaction>::Key> + Sync>(
        &self,
        _output: Self::Output,
        _input: &Self::Input,
        _shared: SharedViewArgs<'_, S>,
        _mode: ViewMode<'_, Self::Input>,
        _txn_idx: TxnIndex,
    ) -> Result<
        <Self::Output as TxnOutput>::CommittedOutput,
        PanicOr<ResourceGroupSerializationError>,
    > {
        // TODO(completeness): serialize the `MonoValue` writes via `self.guard`
        // (a `LayoutProvider`) and reuse `mono_move_output::to_transaction_output`.
        Ok(TransactionOutput::new(
            WriteSet::default(),
            vec![],
            0,
            TransactionStatus::Keep(AptosExecutionStatus::Success),
            TransactionAuxiliaryData::None,
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
        match key {
            InMemoryStorageKey::Resource { address, ty } => {
                // TODO: use self.guard for type to state key conversion.
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
