// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The block executor's view of a VM: how it executes and materializes a single
//! transaction, independent of the concrete VM. The legacy Move VM (its
//! `ExecutorTask`) is adapted to this trait via `LegacyAdapter`; other VMs
//! implement it directly.

use crate::{
    captured_reads::{LegacyReads, SnapshotModuleView, TxnInput},
    code_cache_global::GlobalModuleCache,
    counters::{self, TRACE_REPLAY_SECONDS},
    errors::ResourceGroupSerializationError,
    executor_utilities::{materialize_output, ParallelMaterializer, SequentialMaterializer},
    scheduler_wrapper::SchedulerWrapper,
    task::{ExecutorTask, RecordedOutput, TransactionOutput},
    view::{LatestView, ParallelState, SequentialState, ViewState},
};
use aptos_mvhashmap::{
    types::{Incarnation, MVDelayedFieldsError, TxnIndex},
    unsync_map::UnsyncMap,
    MVHashMap,
};
use aptos_types::{
    block_executor::value::ValueWithLayout,
    error::{PanicError, PanicOr},
    state_store::TStateView,
    transaction::{AuxiliaryInfoTrait, BlockExecutableTransaction as Transaction},
    vm::modules::AptosModuleExtension,
};
use aptos_logger::error;
use aptos_vm_environment::environment::AptosEnvironment;
use aptos_vm_logging::{alert, prelude::*};
use move_binary_format::CompiledModule;
use move_core_types::language_storage::ModuleId;
use move_vm_runtime::{Module, RuntimeEnvironment, TypeChecker};
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
use std::{cell::RefCell, sync::atomic::AtomicU32};

/// VM-neutral ingredients for building a per-transaction view/provider over the
/// multi-version data structures. Carries the raw pieces (not a built view), so
/// each VM constructs its own view: the legacy VM builds a `LatestView`, mono
/// builds its own resource provider.
pub enum LatestViewArgs<'a, I: TxnInput, S> {
    Parallel {
        base_view: &'a S,
        versioned_map: &'a MVHashMap<I::Key, I::Tag, I::Value, DelayedFieldID>,
        global_module_cache:
            &'a GlobalModuleCache<ModuleId, CompiledModule, Module, AptosModuleExtension>,
        runtime_environment: &'a RuntimeEnvironment,
        scheduler: SchedulerWrapper<'a>,
        start_shared_counter: u32,
        shared_counter: &'a AtomicU32,
        incarnation: Incarnation,
    },
    Sequential {
        base_view: &'a S,
        unsync_map: &'a UnsyncMap<I::Key, I::Tag, I::Value, DelayedFieldID>,
        global_module_cache:
            &'a GlobalModuleCache<ModuleId, CompiledModule, Module, AptosModuleExtension>,
        runtime_environment: &'a RuntimeEnvironment,
        start_counter: u32,
        counter: &'a RefCell<u32>,
    },
}

/// A VM as the block executor drives it for a single transaction.
///
/// `Input`/`Output` share the multi-version map's `Key`/`Tag`/`Value`; those are
/// the types the block executor stores and validates against, and each VM picks
/// its own.
pub trait SingleTransactionExecutor {
    type Txn: Transaction;
    type AuxiliaryInfo: AuxiliaryInfoTrait;
    type Input: TxnInput + 'static;
    type Output: TransactionOutput<
            Txn = Self::Txn,
            Key = <Self::Input as TxnInput>::Key,
            Tag = <Self::Input as TxnInput>::Tag,
            Value = <Self::Input as TxnInput>::Value,
        > + 'static;

    /// Create an instance of the executor for a block. The async-runtime-checks
    /// decision is made by the block executor (it depends on the block size) and
    /// passed in; VMs that do not defer runtime checks ignore it.
    fn init(
        environment: &AptosEnvironment,
        state_view: &impl TStateView<Key = <Self::Txn as Transaction>::Key>,
        async_runtime_checks_enabled: bool,
    ) -> Self;

    /// Execute a single transaction, returning its recorded outcome and read set.
    /// A code-invariant failure is reported as `Err(PanicError)` and not stored.
    fn execute<S: TStateView<Key = <Self::Txn as Transaction>::Key> + Sync>(
        &self,
        args: LatestViewArgs<'_, Self::Input, S>,
        txn: &Self::Txn,
        auxiliary_info: &Self::AuxiliaryInfo,
        txn_idx: TxnIndex,
    ) -> Result<(RecordedOutput<Self::Output>, Self::Input), PanicError>;

    /// Materialize a committed output in place, using the read set for delayed
    /// field exchange and (legacy) trace replay. Any trace replay is performed
    /// internally, so no trace is returned.
    fn materialize<S: TStateView<Key = <Self::Txn as Transaction>::Key> + Sync>(
        &self,
        output: &mut Self::Output,
        input: &Self::Input,
        args: LatestViewArgs<'_, Self::Input, S>,
        txn_idx: TxnIndex,
    ) -> Result<
        <Self::Output as TransactionOutput>::CommittedOutput,
        PanicOr<ResourceGroupSerializationError>,
    >;

    /// Values the executor may pre-populate into the map before execution.
    fn pre_write_values(
        _txn: &Self::Txn,
    ) -> Vec<(<Self::Input as TxnInput>::Key, <Self::Input as TxnInput>::Value)> {
        vec![]
    }
}

impl<'a, I: TxnInput, S> LatestViewArgs<'a, I, S> {
    fn runtime_environment(&self) -> &'a RuntimeEnvironment {
        match self {
            LatestViewArgs::Parallel {
                runtime_environment,
                ..
            }
            | LatestViewArgs::Sequential {
                runtime_environment,
                ..
            } => runtime_environment,
        }
    }

    pub(crate) fn global_module_cache(
        &self,
    ) -> &'a GlobalModuleCache<ModuleId, CompiledModule, Module, AptosModuleExtension> {
        match self {
            LatestViewArgs::Parallel {
                global_module_cache,
                ..
            }
            | LatestViewArgs::Sequential {
                global_module_cache,
                ..
            } => global_module_cache,
        }
    }
}

/// Builds the legacy `LatestView` from the ingredients: a `ParallelState` for
/// the parallel mode, a `SequentialState` for the sequential mode.
fn build_latest_view<'a, T: Transaction, S: TStateView<Key = T::Key>>(
    args: LatestViewArgs<'a, LegacyReads<T>, S>,
    txn_idx: TxnIndex,
) -> LatestView<'a, T, S> {
    match args {
        LatestViewArgs::Parallel {
            base_view,
            versioned_map,
            global_module_cache,
            runtime_environment,
            scheduler,
            start_shared_counter,
            shared_counter,
            incarnation,
        } => LatestView::new(
            base_view,
            global_module_cache,
            runtime_environment,
            ViewState::Sync(ParallelState::new(
                versioned_map,
                scheduler,
                start_shared_counter,
                shared_counter,
                incarnation,
            )),
            txn_idx,
        ),
        LatestViewArgs::Sequential {
            base_view,
            unsync_map,
            global_module_cache,
            runtime_environment,
            start_counter,
            counter,
        } => LatestView::new(
            base_view,
            global_module_cache,
            runtime_environment,
            ViewState::Unsync(SequentialState::new(unsync_map, start_counter, counter)),
            txn_idx,
        ),
    }
}

/// Adapts any legacy `ExecutorTask` to `SingleTransactionExecutor`: it builds the
/// `LatestView` from the ingredients, runs the task, and extracts the read set.
/// Keeps `LatestView`/`CapturedReads` internal to the block executor.
pub struct LegacyAdapter<E> {
    inner: E,
    async_runtime_checks_enabled: bool,
}

impl<E: ExecutorTask> SingleTransactionExecutor for LegacyAdapter<E> {
    type AuxiliaryInfo = E::AuxiliaryInfo;
    type Input = LegacyReads<E::Txn>;
    type Output = E::Output;
    type Txn = E::Txn;

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
        args: LatestViewArgs<'_, Self::Input, S>,
        txn: &E::Txn,
        auxiliary_info: &E::AuxiliaryInfo,
        txn_idx: TxnIndex,
    ) -> Result<(RecordedOutput<E::Output>, Self::Input), PanicError> {
        let is_parallel = matches!(args, LatestViewArgs::Parallel { .. });
        let view = build_latest_view(args, txn_idx);
        let recorded = self
            .inner
            .execute_transaction(&view, txn, auxiliary_info, txn_idx)?;

        let reads = if is_parallel {
            let mut reads = view.take_parallel_reads();
            if recorded.is_speculative_failure() {
                // BlockSTMv1 relies on the read set carrying the speculative
                // failure so that its own validation fails and it re-executes.
                reads.capture_delayed_field_read_error(&PanicOr::Or(
                    MVDelayedFieldsError::DeltaApplicationFailure,
                ));
            }
            LegacyReads::Parallel(reads)
        } else {
            LegacyReads::Sequential(view.take_sequential_reads())
        };
        Ok((recorded, reads))
    }

    fn materialize<S: TStateView<Key = <E::Txn as Transaction>::Key> + Sync>(
        &self,
        output: &mut E::Output,
        input: &Self::Input,
        args: LatestViewArgs<'_, Self::Input, S>,
        txn_idx: TxnIndex,
    ) -> Result<
        <E::Output as TransactionOutput>::CommittedOutput,
        PanicOr<ResourceGroupSerializationError>,
    > {
        let runtime_environment = args.runtime_environment();
        let view = build_latest_view(args, txn_idx);
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

    fn pre_write_values(
        txn: &E::Txn,
    ) -> Vec<(
        <E::Txn as Transaction>::Key,
        ValueWithLayout<<E::Txn as Transaction>::Value>,
    )> {
        E::pre_write_values(txn)
    }
}
