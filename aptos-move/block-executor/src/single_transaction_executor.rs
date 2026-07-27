// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! How the block executor drives a VM for a single transaction, independent of
//! the concrete VM. The legacy Move VM (its `ExecutorTask`) is adapted via
//! `LegacyAdapter`; other VMs implement `SingleTransactionExecutor` directly.

use crate::{
    captured_reads::{LegacyReads, SnapshotModuleView, TxnInput},
    code_cache_global::GlobalModuleCache,
    counters::{self, TRACE_REPLAY_SECONDS},
    errors::ResourceGroupSerializationError,
    executor_utilities::{materialize_output, ParallelMaterializer, SequentialMaterializer},
    scheduler_wrapper::SchedulerWrapper,
    task::{ExecutionStatus, ExecutorTask, TxnOutput},
    view::{LatestView, ParallelState, SequentialState, ViewState},
};
use aptos_logger::error;
use aptos_mvhashmap::{
    types::{Incarnation, TxnIndex},
    unsync_map::UnsyncMap,
    MVHashMap,
};
use aptos_types::{
    block_executor::value::{SpeculativeValue, ValueWithLayout},
    error::{PanicError, PanicOr},
    state_store::TStateView,
    transaction::{AuxiliaryInfoTrait, BlockExecutableTransaction as Transaction},
    vm::modules::AptosModuleExtension,
};
use aptos_vm_environment::environment::AptosEnvironment;
use aptos_vm_logging::{alert, prelude::*};
use move_binary_format::CompiledModule;
use move_core_types::language_storage::ModuleId;
use move_vm_runtime::{Module, RuntimeEnvironment, TypeChecker};
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
use serde::Serialize;
use std::{cell::RefCell, fmt::Debug, hash::Hash, sync::atomic::AtomicU32};

/// Mode-invariant ingredients for a per-transaction view. The legacy code cache
/// is isolated here; a VM with its own code caching ignores it.
pub struct SharedViewArgs<'a, S> {
    pub base_view: &'a S,
    pub runtime_environment: &'a RuntimeEnvironment,
    pub global_module_cache:
        &'a GlobalModuleCache<ModuleId, CompiledModule, Module, AptosModuleExtension>,
}

/// The multi-version substrate plus the delayed-field id allocator (and, for
/// parallel, the scheduling state) needed to build a per-transaction view.
pub enum ViewMode<'a, I: TxnInput> {
    Parallel {
        versioned_map: &'a MVHashMap<I::Key, I::Tag, I::Value, DelayedFieldID>,
        scheduler: SchedulerWrapper<'a>,
        start_shared_counter: u32,
        shared_counter: &'a AtomicU32,
        incarnation: Incarnation,
    },
    Sequential {
        unsync_map: &'a UnsyncMap<I::Key, I::Tag, I::Value, DelayedFieldID>,
        start_counter: u32,
        counter: &'a RefCell<u32>,
    },
}

/// A VM used by the block executor to run a single transaction.
pub trait SingleTransactionExecutor {
    type Txn: Transaction;
    type AuxiliaryInfo: AuxiliaryInfoTrait;
    type Key: Ord + Send + Sync + Clone + Hash + Debug + 'static;
    type Tag: Ord + Send + Sync + Clone + Hash + Debug + Serialize + 'static;
    type Value: SpeculativeValue + 'static;

    type Input: TxnInput<Key = Self::Key, Tag = Self::Tag, Value = Self::Value> + 'static;
    type Output: TxnOutput<Txn = Self::Txn, Key = Self::Key, Tag = Self::Tag, Value = Self::Value>
        + 'static;

    /// The async-runtime-checks decision is made by the block executor and passed
    /// in; VMs that do not defer runtime checks ignore it.
    fn init(
        environment: &AptosEnvironment,
        state_view: &impl TStateView<Key = <Self::Txn as Transaction>::Key>,
        async_runtime_checks_enabled: bool,
    ) -> Self;

    /// Execute a single transaction, returning its recorded outcome and read set.
    fn execute<S: TStateView<Key = <Self::Txn as Transaction>::Key> + Sync>(
        &self,
        shared: SharedViewArgs<'_, S>,
        mode: ViewMode<'_, Self::Input>,
        txn: &Self::Txn,
        auxiliary_info: &Self::AuxiliaryInfo,
        txn_idx: TxnIndex,
    ) -> Result<(ExecutionStatus<Self::Output>, Self::Input), PanicError>;

    /// Materialize a committed output, using the read set for delayed field
    /// exchange and (legacy) trace replay. Trace replay is performed internally.
    fn materialize<S: TStateView<Key = <Self::Txn as Transaction>::Key> + Sync>(
        &self,
        output: Self::Output,
        input: &Self::Input,
        shared: SharedViewArgs<'_, S>,
        mode: ViewMode<'_, Self::Input>,
        txn_idx: TxnIndex,
    ) -> Result<
        <Self::Output as TxnOutput>::CommittedOutput,
        PanicOr<ResourceGroupSerializationError>,
    >;

    /// Whether this output can be materialized (bcs-serialized to its storage
    /// representation). Only invoked during the sequential resource-group
    /// serialization fallback, to discard an output that would not serialize.
    fn check_materialization<S: TStateView<Key = <Self::Txn as Transaction>::Key> + Sync>(
        &self,
        output: &Self::Output,
        shared: SharedViewArgs<'_, S>,
        mode: ViewMode<'_, Self::Input>,
        txn_idx: TxnIndex,
    ) -> bool;

    /// Values the executor may pre-populate into the map before execution.
    fn pre_write_values(_txn: &Self::Txn) -> Vec<(Self::Key, Self::Value)> {
        vec![]
    }
}

/// Builds the legacy `LatestView` from the view ingredients.
fn build_latest_view<'a, T: Transaction, S: TStateView<Key = T::Key>>(
    shared: SharedViewArgs<'a, S>,
    mode: ViewMode<'a, LegacyReads<T>>,
    txn_idx: TxnIndex,
) -> LatestView<'a, T, S> {
    let view_state = match mode {
        ViewMode::Parallel {
            versioned_map,
            scheduler,
            start_shared_counter,
            shared_counter,
            incarnation,
        } => ViewState::Sync(ParallelState::new(
            versioned_map,
            scheduler,
            start_shared_counter,
            shared_counter,
            incarnation,
        )),
        ViewMode::Sequential {
            unsync_map,
            start_counter,
            counter,
        } => ViewState::Unsync(SequentialState::new(unsync_map, start_counter, counter)),
    };
    LatestView::new(
        shared.base_view,
        shared.global_module_cache,
        shared.runtime_environment,
        view_state,
        txn_idx,
    )
}

/// Adapts any legacy [`ExecutorTask`] to [`SingleTransactionExecutor`].
pub struct LegacyTransactionExecutor<E> {
    inner: E,
    async_runtime_checks_enabled: bool,
}

impl<E: ExecutorTask> SingleTransactionExecutor for LegacyTransactionExecutor<E> {
    type AuxiliaryInfo = E::AuxiliaryInfo;
    type Input = LegacyReads<E::Txn>;
    // Legacy: the in-memory key/tag are the storage key/tag; the map value wraps
    // the storage write with its speculative layout.
    type Key = <E::Txn as Transaction>::Key;
    type Output = E::Output;
    type Tag = <E::Txn as Transaction>::Tag;
    type Txn = E::Txn;
    type Value = ValueWithLayout<<E::Txn as Transaction>::Value>;

    fn init(
        environment: &AptosEnvironment,
        state_view: &impl TStateView<Key = <E::Txn as Transaction>::Key>,
        async_runtime_checks_enabled: bool,
    ) -> Self {
        Self {
            inner: E::init(environment, state_view, async_runtime_checks_enabled),
            async_runtime_checks_enabled,
        }
    }

    fn execute<S: TStateView<Key = <E::Txn as Transaction>::Key> + Sync>(
        &self,
        shared: SharedViewArgs<'_, S>,
        mode: ViewMode<'_, Self::Input>,
        txn: &E::Txn,
        auxiliary_info: &E::AuxiliaryInfo,
        txn_idx: TxnIndex,
    ) -> Result<(ExecutionStatus<E::Output>, Self::Input), PanicError> {
        let is_parallel = matches!(mode, ViewMode::Parallel { .. });
        let view = build_latest_view(shared, mode, txn_idx);
        let status = self
            .inner
            .execute_transaction(&view, txn, auxiliary_info, txn_idx)?;

        let reads = if is_parallel {
            LegacyReads::Parallel(view.take_parallel_reads())
        } else {
            LegacyReads::Sequential(view.take_sequential_reads())
        };
        Ok((status, reads))
    }

    fn materialize<S: TStateView<Key = <E::Txn as Transaction>::Key> + Sync>(
        &self,
        output: E::Output,
        input: &Self::Input,
        shared: SharedViewArgs<'_, S>,
        mode: ViewMode<'_, Self::Input>,
        txn_idx: TxnIndex,
    ) -> Result<<E::Output as TxnOutput>::CommittedOutput, PanicOr<ResourceGroupSerializationError>>
    {
        let runtime_environment = shared.runtime_environment;
        let view = build_latest_view(shared, mode, txn_idx);
        let (committed_output, trace) = match input {
            LegacyReads::Parallel(reads) => {
                let materializer = ParallelMaterializer::new(&view, reads);
                materialize_output(output, &materializer)?
            },
            LegacyReads::Sequential(_) => {
                let materializer = SequentialMaterializer::new(&view);
                materialize_output(output, &materializer)?
            },
        };

        // Trace replay is legacy-only and needs the captured reads; only the
        // parallel path collects traces.
        if let LegacyReads::Parallel(reads) = input {
            if self.async_runtime_checks_enabled && !trace.is_empty() {
                let snapshot_view = SnapshotModuleView::new(reads, runtime_environment);
                let result = {
                    counters::update_txn_trace_counters(&trace);
                    let _timer = TRACE_REPLAY_SECONDS.start_timer();
                    TypeChecker::new(&snapshot_view).replay(&trace)
                };
                if let Err(err) = result {
                    alert!(
                        "Runtime type check failed during replay of transaction {}: {:?}",
                        txn_idx,
                        err
                    );
                    return Err(PanicOr::CodeInvariantError(format!(
                        "Sequential fallback on type check failure for transaction {}: {:?}",
                        txn_idx, err
                    )));
                }
            }
        }
        Ok(committed_output)
    }

    fn check_materialization<S: TStateView<Key = <E::Txn as Transaction>::Key> + Sync>(
        &self,
        output: &E::Output,
        shared: SharedViewArgs<'_, S>,
        mode: ViewMode<'_, Self::Input>,
        txn_idx: TxnIndex,
    ) -> bool {
        let view = build_latest_view(shared, mode, txn_idx);
        output.check_materialization(&SequentialMaterializer::new(&view))
    }

    fn pre_write_values(
        txn: &E::Txn,
    ) -> Vec<(
        <E::Txn as Transaction>::Key,
        ValueWithLayout<<E::Txn as Transaction>::Value>,
    )> {
        E::pre_write_values(txn)
    }
}
