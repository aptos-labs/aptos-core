// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    code_cache_global_manager::AptosModuleCacheManagerGuard,
    combinatorial_tests::{
        mock_executor::MockEvent,
        types::{KeyType, ValueType},
    },
    executor::BlockExecutor,
    task::{
        AfterMaterializationOutput, BeforeMaterializationOutput, ExecutionStatus, ExecutorTask,
        TransactionOutput,
    },
    txn_commit_hook::NoOpTransactionCommitHook,
    txn_provider::default::DefaultTxnProvider,
    types::InputOutputKey,
};
use aptos_aggregator::{
    delayed_change::DelayedChange, delta_change_set::DeltaOp, resolver::TAggregatorV1View,
};
use aptos_mvhashmap::types::TxnIndex;
use aptos_types::{
    block_executor::{
        config::BlockExecutorConfig, transaction_slice_metadata::TransactionSliceMetadata,
    },
    error::PanicError,
    fee_statement::FeeStatement,
    state_store::{state_value::StateValueMetadata, MockStateView, TStateView},
    transaction::{
        AuxiliaryInfo, BlockExecutableTransaction, TransactionOutput as AptosTransactionOutput,
    },
    write_set::{WriteOp, WriteOpKind},
};
use aptos_vm_environment::environment::AptosEnvironment;
use aptos_vm_types::{
    module_and_script_storage::code_storage::AptosCodeStorage,
    module_write_set::ModuleWrite,
    resolver::{
        BlockSynchronizationKillSwitch, ResourceGroupSize, TExecutorView, TResourceGroupView,
    },
};
use bytes::Bytes;
use move_core_types::{value::MoveTypeLayout, vm_status::StatusCode};
use move_vm_runtime::execution_tracing::Trace;
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
use once_cell::sync::OnceCell;
use rayon::prelude::*;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};
use triomphe::Arc as TriompheArc;

const INVALIDATED_KEY: KeyType<u32> = KeyType(11);
const ESTIMATED_KEY: KeyType<u32> = KeyType(22);
const NUM_DEPENDENT_READ_TASKS: usize = 64;
const NUM_NESTED_WORK_RUN_ATTEMPTS: usize = 100_000;

#[derive(Debug)]
struct DependencyTestState {
    txn1_read_invalidated_key: AtomicBool,
    txn1_reexecution_started: AtomicBool,
    txn2_read_tasks_spawned: AtomicBool,
    txn1_reexecution_finished: AtomicBool,
    txn2_read_gate: Mutex<()>,
}

impl Default for DependencyTestState {
    fn default() -> Self {
        Self {
            txn1_read_invalidated_key: AtomicBool::new(false),
            txn1_reexecution_started: AtomicBool::new(false),
            txn2_read_tasks_spawned: AtomicBool::new(false),
            txn1_reexecution_finished: AtomicBool::new(false),
            txn2_read_gate: Mutex::new(()),
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum DependencyTxnKind {
    Invalidator,
    ReexecutedWriter,
    DependentReader,
    StateCheckpoint,
}

#[derive(Clone, Debug)]
struct DependencyTxn {
    kind: DependencyTxnKind,
    state: Arc<DependencyTestState>,
    execution_count: Arc<AtomicUsize>,
}

impl DependencyTxn {
    fn new(kind: DependencyTxnKind, state: Arc<DependencyTestState>) -> Self {
        Self {
            kind,
            state,
            execution_count: Arc::new(AtomicUsize::new(0)),
        }
    }
}

impl BlockExecutableTransaction for DependencyTxn {
    type Event = MockEvent;
    type Key = KeyType<u32>;
    type Tag = u32;
    type Value = ValueType;

    fn user_txn_bytes_len(&self) -> usize {
        0
    }

    fn state_checkpoint(_block_id: aptos_crypto::HashValue) -> Self {
        Self::new(
            DependencyTxnKind::StateCheckpoint,
            Arc::new(DependencyTestState::default()),
        )
    }
}

#[derive(Debug)]
struct DependencyOutput {
    writes: Vec<(KeyType<u32>, ValueType)>,
    module_writes: BTreeMap<KeyType<u32>, ModuleWrite<ValueType>>,
    total_gas: u64,
    skipped: bool,
    committed_output: OnceCell<AptosTransactionOutput>,
}

impl DependencyOutput {
    fn success(writes: Vec<(KeyType<u32>, ValueType)>) -> Self {
        Self {
            writes,
            module_writes: BTreeMap::new(),
            total_gas: 1,
            skipped: false,
            committed_output: OnceCell::new(),
        }
    }

    fn skipped() -> Self {
        Self {
            writes: vec![],
            module_writes: BTreeMap::new(),
            total_gas: 0,
            skipped: true,
            committed_output: OnceCell::new(),
        }
    }
}

impl BeforeMaterializationOutput<DependencyTxn> for &DependencyOutput {
    fn resource_write_set(
        &self,
    ) -> HashMap<KeyType<u32>, (TriompheArc<ValueType>, Option<TriompheArc<MoveTypeLayout>>)> {
        self.writes
            .iter()
            .map(|(key, value)| (*key, (TriompheArc::new(value.clone()), None)))
            .collect()
    }

    fn module_write_set(&self) -> &BTreeMap<KeyType<u32>, ModuleWrite<ValueType>> {
        &self.module_writes
    }

    fn aggregator_v1_write_set(&self) -> BTreeMap<KeyType<u32>, ValueType> {
        BTreeMap::new()
    }

    fn aggregator_v1_delta_set(&self) -> BTreeMap<KeyType<u32>, DeltaOp> {
        BTreeMap::new()
    }

    fn delayed_field_change_set(&self) -> BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>> {
        BTreeMap::new()
    }

    fn reads_needing_delayed_field_exchange(
        &self,
    ) -> Vec<(
        KeyType<u32>,
        StateValueMetadata,
        TriompheArc<MoveTypeLayout>,
    )> {
        vec![]
    }

    fn group_reads_needing_delayed_field_exchange(
        &self,
    ) -> Vec<(KeyType<u32>, StateValueMetadata)> {
        vec![]
    }

    fn get_events(&self) -> Vec<(MockEvent, Option<MoveTypeLayout>)> {
        vec![]
    }

    fn resource_group_write_set(
        &self,
    ) -> HashMap<
        KeyType<u32>,
        (
            ValueType,
            ResourceGroupSize,
            BTreeMap<u32, (ValueType, Option<TriompheArc<MoveTypeLayout>>)>,
        ),
    > {
        HashMap::new()
    }

    fn for_each_resource_key_no_aggregator_v1(
        &self,
        callback: &mut dyn FnMut(&KeyType<u32>) -> Result<(), PanicError>,
    ) -> Result<(), PanicError> {
        for (key, _) in &self.writes {
            callback(key)?;
        }
        Ok(())
    }

    fn for_each_resource_group_key_and_tags(
        &self,
        _callback: &mut dyn FnMut(&KeyType<u32>, HashSet<&u32>) -> Result<(), PanicError>,
    ) -> Result<(), PanicError> {
        Ok(())
    }

    fn fee_statement(&self) -> FeeStatement {
        FeeStatement::new(self.total_gas, self.total_gas, 0, 0, 0)
    }

    fn has_new_epoch_event(&self) -> bool {
        false
    }

    fn output_approx_size(&self) -> u64 {
        0
    }

    fn get_write_summary(&self) -> HashSet<InputOutputKey<KeyType<u32>, u32>> {
        HashSet::new()
    }
}

impl AfterMaterializationOutput<DependencyTxn> for &DependencyOutput {
    fn fee_statement(&self) -> FeeStatement {
        FeeStatement::new(self.total_gas, self.total_gas, 0, 0, 0)
    }

    fn has_new_epoch_event(&self) -> bool {
        false
    }
}

impl TransactionOutput for DependencyOutput {
    type AfterMaterializationGuard<'a> = &'a Self;
    type BeforeMaterializationGuard<'a> = &'a Self;
    type Txn = DependencyTxn;

    fn committed_output(&self) -> &OnceCell<AptosTransactionOutput> {
        &self.committed_output
    }

    fn skip_output() -> Self {
        Self::skipped()
    }

    fn discard_output(_discard_code: StatusCode) -> Self {
        Self::skipped()
    }

    fn before_materialization<'a>(
        &'a self,
    ) -> Result<Self::BeforeMaterializationGuard<'a>, PanicError> {
        Ok(self)
    }

    fn after_materialization<'a>(
        &'a self,
    ) -> Result<Self::AfterMaterializationGuard<'a>, PanicError> {
        Ok(self)
    }

    fn is_materialized_and_success(&self) -> bool {
        !self.skipped
    }

    fn check_materialization(&self) -> Result<bool, PanicError> {
        Ok(!self.skipped)
    }

    fn incorporate_materialized_txn_output(
        &mut self,
        _aggregator_v1_writes: Vec<(KeyType<u32>, WriteOp)>,
        patched_resource_write_set: Vec<(KeyType<u32>, ValueType)>,
        _patched_events: Vec<MockEvent>,
    ) -> Result<Trace, PanicError> {
        self.writes.extend(patched_resource_write_set);
        Ok(Trace::empty())
    }

    fn set_txn_output_for_non_dynamic_change_set(&mut self) {}

    fn legacy_sequential_materialize_agg_v1(
        &mut self,
        _view: &impl TAggregatorV1View<Identifier = KeyType<u32>>,
    ) {
    }
}

struct SendViewPtr<V: ?Sized>(*const V);

impl<V: ?Sized> Copy for SendViewPtr<V> {}

impl<V: ?Sized> Clone for SendViewPtr<V> {
    fn clone(&self) -> Self {
        *self
    }
}

// The nested par_iter below guarantees that all spawned read tasks finish before the borrowed view
// can be dropped, and txn2_read_gate serializes access to LatestView's RefCell-backed read capture.
unsafe impl<V: ?Sized> Send for SendViewPtr<V> {}
unsafe impl<V: ?Sized> Sync for SendViewPtr<V> {}

impl<V: ?Sized> SendViewPtr<V> {
    fn new(view: &V) -> Self {
        Self(view as *const V)
    }

    unsafe fn as_ref(&self) -> &V {
        unsafe { &*self.0 }
    }
}

struct NestedParallelDependencyTask;

impl ExecutorTask for NestedParallelDependencyTask {
    type AuxiliaryInfo = AuxiliaryInfo;
    type Error = usize;
    type Output = DependencyOutput;
    type Txn = DependencyTxn;

    fn init(
        _environment: &AptosEnvironment,
        _state_view: &impl TStateView<Key = KeyType<u32>>,
        _async_runtime_checks_enabled: bool,
    ) -> Self {
        Self
    }

    fn execute_transaction(
        &self,
        view: &(impl TExecutorView<KeyType<u32>, u32, MoveTypeLayout, ValueType>
              + TResourceGroupView<
            GroupKey = KeyType<u32>,
            ResourceTag = u32,
            Layout = MoveTypeLayout,
        > + AptosCodeStorage
              + BlockSynchronizationKillSwitch),
        txn: &Self::Txn,
        _auxiliary_info: &Self::AuxiliaryInfo,
        _txn_idx: TxnIndex,
    ) -> ExecutionStatus<Self::Output, Self::Error> {
        match txn.kind {
            DependencyTxnKind::Invalidator => {
                wait_until("txn1 reads the key that txn0 invalidates", || {
                    txn.state.txn1_read_invalidated_key.load(Ordering::Acquire)
                });
                ExecutionStatus::Success(DependencyOutput::success(vec![(
                    INVALIDATED_KEY,
                    test_value(1),
                )]))
            },
            DependencyTxnKind::ReexecutedWriter => {
                let execution = txn.execution_count.fetch_add(1, Ordering::AcqRel);
                if execution == 0 {
                    view.get_resource_bytes(&INVALIDATED_KEY, None)
                        .expect("read of invalidated key should not fail");
                    txn.state
                        .txn1_read_invalidated_key
                        .store(true, Ordering::Release);
                    ExecutionStatus::Success(DependencyOutput::success(vec![(
                        ESTIMATED_KEY,
                        test_value(2),
                    )]))
                } else {
                    txn.state
                        .txn1_reexecution_started
                        .store(true, Ordering::Release);
                    wait_until("txn2 spawns nested parallel dependency reads", || {
                        txn.state.txn2_read_tasks_spawned.load(Ordering::Acquire)
                    });

                    // If a BlockSTM worker is also registered with the nested parallel runtime,
                    // this yield can execute txn2's dependency read while txn1 is still marked as
                    // executing. The executor must not let that pattern park the worker on txn1's
                    // own estimated write.
                    for _ in 0..NUM_NESTED_WORK_RUN_ATTEMPTS {
                        if rayon::yield_now().is_none() {
                            break;
                        }
                        thread::yield_now();
                    }

                    txn.state
                        .txn1_reexecution_finished
                        .store(true, Ordering::Release);
                    ExecutionStatus::Success(DependencyOutput::success(vec![(
                        ESTIMATED_KEY,
                        test_value(3),
                    )]))
                }
            },
            DependencyTxnKind::DependentReader => {
                wait_until("txn1 starts re-executing the estimated write", || {
                    txn.state.txn1_reexecution_started.load(Ordering::Acquire)
                });

                let view = SendViewPtr::new(view);
                let state = Arc::clone(&txn.state);
                (0..NUM_DEPENDENT_READ_TASKS).into_par_iter().for_each(|_| {
                    state.txn2_read_tasks_spawned.store(true, Ordering::Release);
                    let _guard = state
                        .txn2_read_gate
                        .lock()
                        .expect("txn2 read gate poisoned");
                    let _ = unsafe { view.as_ref() }
                        .get_resource_bytes(&ESTIMATED_KEY, None)
                        .expect("dependent read should not fail");
                });

                ExecutionStatus::Success(DependencyOutput::success(vec![]))
            },
            DependencyTxnKind::StateCheckpoint => {
                ExecutionStatus::Success(DependencyOutput::success(vec![]))
            },
        }
    }

    fn is_transaction_dynamic_change_set_capable(_txn: &Self::Txn) -> bool {
        true
    }
}

fn wait_until(name: &str, condition: impl Fn() -> bool) {
    let start = Instant::now();
    while !condition() {
        assert!(
            start.elapsed() < Duration::from_secs(10),
            "timed out waiting for {name}"
        );
        thread::yield_now();
    }
}

fn test_value(byte: u8) -> ValueType {
    ValueType::new(
        Some(Bytes::from(vec![byte; 16])),
        StateValueMetadata::none(),
        WriteOpKind::Creation,
    )
}

#[test]
fn nested_parallel_dependency_reads_do_not_deadlock_blockstm_workers() {
    // Regression coverage for nested parallel work that observes BlockSTM dependencies:
    // 1. txn0 waits until txn1 has read INVALIDATED_KEY, then writes it.
    // 2. txn1's first incarnation writes ESTIMATED_KEY, then aborts because txn0 invalidates
    //    its read. Its ESTIMATED_KEY write is now an estimate.
    // 3. txn1 re-executes and gives nested parallel work a chance to run.
    // 4. txn2 enters a nested par_iter whose tasks read ESTIMATED_KEY and park on txn1's
    //    estimate. The executor must not let txn1's worker park on its own completion.
    let state = Arc::new(DependencyTestState::default());
    let transactions = vec![
        DependencyTxn::new(DependencyTxnKind::Invalidator, Arc::clone(&state)),
        DependencyTxn::new(DependencyTxnKind::ReexecutedWriter, Arc::clone(&state)),
        DependencyTxn::new(DependencyTxnKind::DependentReader, Arc::clone(&state)),
    ];
    let txn1_execution_count = Arc::clone(&transactions[1].execution_count);
    let txn_provider = DefaultTxnProvider::new_without_info(transactions);
    let data_view = MockStateView::empty();
    let block_executor = BlockExecutor::<
        DependencyTxn,
        NestedParallelDependencyTask,
        MockStateView<KeyType<u32>>,
        NoOpTransactionCommitHook<usize>,
        DefaultTxnProvider<DependencyTxn, AuxiliaryInfo>,
        AuxiliaryInfo,
    >::new(BlockExecutorConfig::new_no_block_limit(2), None);
    let mut guard = AptosModuleCacheManagerGuard::none();

    let output = block_executor
        .execute_transactions_parallel(
            &txn_provider,
            &data_view,
            &TransactionSliceMetadata::unknown(),
            &mut guard,
        )
        .expect("parallel execution should succeed")
        .into_inner()
        .0;

    assert_eq!(output.len(), 3);
    assert!(
        txn1_execution_count.load(Ordering::Acquire) >= 2,
        "txn1 must be aborted and re-executed to create an estimated write"
    );
    assert!(
        state.txn1_reexecution_finished.load(Ordering::Acquire),
        "txn1 should finish re-execution after nested dependency reads are resolved"
    );
}
