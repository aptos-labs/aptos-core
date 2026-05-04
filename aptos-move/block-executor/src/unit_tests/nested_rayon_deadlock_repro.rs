// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::random_value;
use crate::{
    code_cache_global_manager::AptosModuleCacheManagerGuard,
    combinatorial_tests::{
        mock_executor::{MockEvent, MockOutput, MockOutputBuilder},
        types::{DeltaTestKind, KeyType, MockIncarnation, MockTransaction, ValueType},
    },
    executor::BlockExecutor,
    task::{ExecutionStatus, ExecutorTask},
    txn_commit_hook::NoOpTransactionCommitHook,
    txn_provider::default::DefaultTxnProvider,
};
use aptos_mvhashmap::types::TxnIndex;
use aptos_types::{
    block_executor::{
        config::BlockExecutorConfig, transaction_slice_metadata::TransactionSliceMetadata,
    },
    state_store::{MockStateView, TStateView},
    transaction::AuxiliaryInfo,
};
use aptos_vm_environment::environment::AptosEnvironment;
use aptos_vm_types::{
    module_and_script_storage::code_storage::AptosCodeStorage,
    resolver::{BlockSynchronizationKillSwitch, TExecutorView, TResourceGroupView},
};
use move_core_types::value::MoveTypeLayout;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Condvar, Mutex, OnceLock,
};

type DeadlockKey = KeyType<u32>;
type DeadlockTxn = MockTransaction<DeadlockKey, MockEvent>;
type DeadlockOutput = MockOutput<DeadlockKey, MockEvent>;

static NESTED_RAYON_DEADLOCK_STATE: OnceLock<Arc<NestedRayonDeadlockState>> = OnceLock::new();

struct NestedRayonDeadlockState {
    txn_1_first_read: (Mutex<bool>, Condvar),
    txn_1_reexecution_entered: AtomicBool,
}

impl NestedRayonDeadlockState {
    fn new() -> Self {
        Self {
            txn_1_first_read: (Mutex::new(false), Condvar::new()),
            txn_1_reexecution_entered: AtomicBool::new(false),
        }
    }

    fn wait_for_txn_1_first_read(&self) {
        let (lock, cvar) = &self.txn_1_first_read;
        let mut read_happened = lock.lock().unwrap();
        while !*read_happened {
            read_happened = cvar.wait(read_happened).unwrap();
        }
    }

    fn mark_txn_1_first_read(&self) {
        let (lock, cvar) = &self.txn_1_first_read;
        let mut read_happened = lock.lock().unwrap();
        *read_happened = true;
        cvar.notify_all();
    }
}

struct NestedRayonDeadlockTask;

impl ExecutorTask for NestedRayonDeadlockTask {
    type AuxiliaryInfo = AuxiliaryInfo;
    type Error = usize;
    type Output = DeadlockOutput;
    type Txn = DeadlockTxn;

    fn init(
        _environment: &AptosEnvironment,
        _state_view: &impl TStateView<Key = DeadlockKey>,
        _async_runtime_checks_enabled: bool,
    ) -> Self {
        Self
    }

    fn execute_transaction(
        &self,
        view: &(impl TExecutorView<DeadlockKey, u32, MoveTypeLayout, ValueType>
              + TResourceGroupView<GroupKey = DeadlockKey, ResourceTag = u32, Layout = MoveTypeLayout>
              + AptosCodeStorage
              + BlockSynchronizationKillSwitch),
        txn: &Self::Txn,
        _auxiliary_info: &Self::AuxiliaryInfo,
        txn_idx: TxnIndex,
    ) -> ExecutionStatus<Self::Output, Self::Error> {
        let state = NESTED_RAYON_DEADLOCK_STATE
            .get()
            .expect("deadlock repro state must be initialized");

        let (incarnation, behavior) = match txn {
            MockTransaction::Write {
                incarnation_counter,
                incarnation_behaviors,
                ..
            } => {
                let incarnation = incarnation_counter.fetch_add(1, Ordering::SeqCst);
                (
                    incarnation,
                    &incarnation_behaviors[incarnation % incarnation_behaviors.len()],
                )
            },
            other => unreachable!("unexpected transaction in deadlock repro: {:?}", other),
        };

        if txn_idx == 0 {
            state.wait_for_txn_1_first_read();
        }

        if txn_idx == 1 && incarnation > 0 {
            state
                .txn_1_reexecution_entered
                .store(true, Ordering::SeqCst);
            eprintln!(
                "txn 1 re-execution is yielding to rayon; the stolen worker should block on txn 1"
            );
            rayon::yield_now();
            eprintln!("unexpectedly returned from nested rayon yield");
        }

        let mut builder = MockOutputBuilder::from_mock_incarnation(behavior, DeltaTestKind::None);
        if let Err(status) = builder.add_resource_reads(view, &behavior.resource_reads, false) {
            return status;
        }

        if txn_idx == 1 && incarnation == 0 {
            state.mark_txn_1_first_read();
        }

        if let Err(status) =
            builder.add_resource_writes(view, &behavior.resource_writes, false, txn_idx)
        {
            return status;
        }

        ExecutionStatus::Success(builder.build())
    }

    fn is_transaction_dynamic_change_set_capable(_txn: &Self::Txn) -> bool {
        true
    }
}

#[test]
fn blockstm_v1_nested_rayon_dependency_deadlock_repro() {
    if num_cpus::get() < 3 {
        eprintln!("deadlock repro requires at least 3 configured BlockSTM workers");
        return;
    }

    let state = NESTED_RAYON_DEADLOCK_STATE
        .get_or_init(|| Arc::new(NestedRayonDeadlockState::new()))
        .clone();

    let conflict_key = KeyType(10);
    let dependency_key = KeyType(20);

    let txn_0_invalidates_txn_1 = MockTransaction::from_behavior(MockIncarnation::new(
        vec![],
        vec![(conflict_key, random_value(false), false)],
        vec![],
        vec![],
        1,
    ));

    let txn_1_writes_dependency = MockTransaction::from_behaviors(vec![
        MockIncarnation::new(
            vec![(conflict_key, false)],
            vec![(dependency_key, random_value(false), false)],
            vec![],
            vec![],
            1,
        ),
        MockIncarnation::new(
            vec![(conflict_key, false)],
            vec![(dependency_key, random_value(false), false)],
            vec![],
            vec![],
            1,
        ),
    ]);

    let txn_2_waits_on_txn_1_estimate = MockTransaction::from_behavior(MockIncarnation::new(
        vec![(dependency_key, false)],
        vec![],
        vec![],
        vec![],
        1,
    ));

    let noop =
        || MockTransaction::from_behavior(MockIncarnation::new(vec![], vec![], vec![], vec![], 1));
    let transactions = vec![
        txn_0_invalidates_txn_1,
        txn_1_writes_dependency,
        txn_2_waits_on_txn_1_estimate,
        noop(),
        noop(),
        noop(),
    ];

    let executor_thread_pool = Arc::new(
        rayon::ThreadPoolBuilder::new()
            .num_threads(2)
            .thread_name(|idx| format!("deadlock-repro-par-exec-{idx}"))
            .build()
            .unwrap(),
    );

    let block_executor = BlockExecutor::<
        DeadlockTxn,
        NestedRayonDeadlockTask,
        MockStateView<DeadlockKey>,
        NoOpTransactionCommitHook<usize>,
        DefaultTxnProvider<DeadlockTxn, AuxiliaryInfo>,
        AuxiliaryInfo,
    >::new(
        BlockExecutorConfig::new_no_block_limit(3),
        executor_thread_pool,
        None,
    );
    let txn_provider = DefaultTxnProvider::new_without_info(transactions);
    let data_view = MockStateView::empty();
    let mut guard = AptosModuleCacheManagerGuard::none();

    let _ = block_executor.execute_transactions_parallel(
        &txn_provider,
        &data_view,
        &TransactionSliceMetadata::unknown(),
        &mut guard,
    );

    assert!(
        state.txn_1_reexecution_entered.load(Ordering::SeqCst),
        "the reproducer did not reach txn 1 re-execution"
    );
}
