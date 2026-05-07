// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Regression test for the BlockSTM rayon-pool deadlock.
//!
//! When BlockSTM workers ran on a `rayon::ThreadPool`, calling `par_iter` /
//! `rayon::scope` from inside `execute_transaction` landed sub-tasks on the
//! same pool. While a worker was waiting for its own sub-tasks - or yielding
//! via `rayon::yield_now()` - rayon could work-steal a sub-task spawned by
//! another BlockSTM worker onto this thread. If the stolen sub-task hit a
//! BlockSTM read dependency (Estimate flag from a previously-aborted txn)
//! and parked on the dependency `Condvar`, the parked thread could be the
//! worker for the txn the stolen task was waiting on -> deadlock.
//!
//! The fix moves BlockSTM workers onto a plain std-thread pool so they are
//! no longer registered with rayon, and nested rayon work runs on rayon's
//! global pool instead.
//!
//! The deadlock's natural probability is too low to catch reliably, so this
//! test orchestrates it with three transactions and shared atomic flags:
//!   txn 0 - Invalidator: writes K_INV after txn 1 has read it.
//!   txn 1 - Re-executed writer: first incarnation reads K_INV (storage None)
//!           and writes K_EST. Validation aborts it once txn 0 commits, which
//!           marks K_EST@1 as Estimate. The re-execution actively
//!           `rayon::yield_now()`s while txn 2 has a nested par_iter pending,
//!           making this worker steal one of those tasks.
//!   txn 2 - Dependent reader: spawns a nested `par_iter` whose tasks read
//!           K_EST. Each task hits the Estimate from txn 1 and calls
//!           `wait_for_dependency`, which parks the running thread.
//!
//! Without the fix, txn 1's worker steals one of txn 2's tasks while txn 1
//! is still in `Executing`, parks on its own estimate, and the executor
//! deadlocks. With the fix, the par_iter sub-tasks run on rayon's global
//! pool and txn 1's worker - a plain std thread - is not eligible to be
//! work-stolen, so txn 1 finishes normally.

use crate::{
    code_cache_global_manager::AptosModuleCacheManagerGuard,
    combinatorial_tests::{
        mock_executor::{MockEvent, MockOutput, MockTask},
        types::{KeyType, MockIncarnation, MockTransaction, ValueType},
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
use std::{
    marker::PhantomData,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc, Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

const K_INV: KeyType<u32> = KeyType(11);
const K_EST: KeyType<u32> = KeyType(22);
const NESTED_TASKS: usize = 64;
const YIELD_BUDGET: usize = 100_000;

#[derive(Default)]
struct Coord {
    txn1_first_done: AtomicBool,
    txn1_reexec_started: AtomicBool,
    txn2_par_iter_started: AtomicBool,
    txn1_reexec_count: AtomicUsize,
    /// Serialises the par_iter sub-tasks' access to LatestView, whose
    /// `captured_reads` is a `RefCell` and would panic on concurrent
    /// `borrow_mut`s.
    view_gate: Mutex<()>,
}

impl Coord {
    fn wait(&self, label: &str, ready: impl Fn() -> bool) {
        let started = Instant::now();
        while !ready() {
            assert!(
                started.elapsed() < Duration::from_secs(5),
                "timed out waiting for {label}"
            );
            thread::yield_now();
        }
    }
}

/// `Send + Sync` bridge for the `&impl TExecutorView` view passed into
/// `execute_transaction`. The trait does not bound the view as `Sync`, but
/// the runtime view (`LatestView`) is, and it outlives the rayon scope below.
struct ForceSync<T: ?Sized>(*const T);
unsafe impl<T: ?Sized> Send for ForceSync<T> {}
unsafe impl<T: ?Sized> Sync for ForceSync<T> {}
impl<T: ?Sized> ForceSync<T> {
    fn new(r: &T) -> Self {
        Self(r as *const T)
    }
    fn get(&self) -> &T {
        unsafe { &*self.0 }
    }
}

/// Shared coordination for the current test invocation. Set before invoking
/// the executor and consumed by `DeadlockTask::execute_transaction`.
static COORD: once_cell::sync::Lazy<std::sync::Mutex<Option<Arc<Coord>>>> =
    once_cell::sync::Lazy::new(|| std::sync::Mutex::new(None));

fn coord() -> Arc<Coord> {
    Arc::clone(
        COORD
            .lock()
            .unwrap()
            .as_ref()
            .expect("coord must be installed"),
    )
}

struct DeadlockTask {
    inner: MockTask<KeyType<u32>, MockEvent>,
    _phantom: PhantomData<()>,
}

impl ExecutorTask for DeadlockTask {
    type AuxiliaryInfo = AuxiliaryInfo;
    type Error = usize;
    type Output = MockOutput<KeyType<u32>, MockEvent>;
    type Txn = MockTransaction<KeyType<u32>, MockEvent>;

    fn init(
        env: &AptosEnvironment,
        sv: &impl TStateView<Key = KeyType<u32>>,
        async_checks: bool,
    ) -> Self {
        Self {
            inner: MockTask::init(env, sv, async_checks),
            _phantom: PhantomData,
        }
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
        aux: &Self::AuxiliaryInfo,
        txn_idx: TxnIndex,
    ) -> ExecutionStatus<Self::Output, Self::Error> {
        let c = coord();
        match txn_idx {
            // Invalidator: must commit *after* txn1 reads K_INV, otherwise
            // txn1's read will already see this write and won't be aborted.
            0 => {
                c.wait("txn1 first read", || {
                    c.txn1_first_done.load(Ordering::Acquire)
                });
                self.inner.execute_transaction(view, txn, aux, txn_idx)
            },
            // Re-executed writer: on first run, read K_INV (storage None) and
            // write K_EST; validation will abort us, marking K_EST@1 as
            // Estimate. On re-execution, advertise that the estimate is in
            // place, wait for txn 2 to spawn its nested reads, then
            // `rayon::yield_now()` to actively work-steal so this worker
            // picks up one of those reads while we are still Executing.
            1 => {
                let n = c.txn1_reexec_count.fetch_add(1, Ordering::AcqRel);
                if n == 0 {
                    let result = self.inner.execute_transaction(view, txn, aux, txn_idx);
                    c.txn1_first_done.store(true, Ordering::Release);
                    result
                } else {
                    c.txn1_reexec_started.store(true, Ordering::Release);
                    c.wait("txn2 par_iter started", || {
                        c.txn2_par_iter_started.load(Ordering::Acquire)
                    });
                    for _ in 0..YIELD_BUDGET {
                        if rayon::yield_now().is_none() {
                            break;
                        }
                        thread::yield_now();
                    }
                    self.inner.execute_transaction(view, txn, aux, txn_idx)
                }
            },
            // Dependent reader: nested par_iter of reads on K_EST. Each task
            // sees the Estimate from txn1 and parks on the dependency
            // Condvar via `wait_for_dependency`. Without the fix, one of
            // these tasks is the one that work-steals onto txn1's rayon
            // worker.
            2 => {
                c.wait("txn1 re-exec started", || {
                    c.txn1_reexec_started.load(Ordering::Acquire)
                });
                let view_sync = ForceSync::new(view);
                let c_for_par = Arc::clone(&c);
                // Use explicit s.spawn so each read is its own rayon task in
                // the spawning worker's deque - that's what gives the other
                // (txn1) worker something to actually work-steal during its
                // `rayon::yield_now()` loop. `par_iter()` would adaptively
                // collapse this into a couple of macro-chunks that the
                // spawning worker runs sequentially inside its closure.
                rayon::scope(|s| {
                    for _ in 0..NESTED_TASKS {
                        let view_sync = &view_sync;
                        let c_for_par = &c_for_par;
                        s.spawn(move |_| {
                            c_for_par
                                .txn2_par_iter_started
                                .store(true, Ordering::Release);
                            let _guard = c_for_par.view_gate.lock().unwrap();
                            let _ = view_sync.get().get_resource_bytes(&K_EST, None);
                        });
                    }
                });
                self.inner.execute_transaction(view, txn, aux, txn_idx)
            },
            _ => self.inner.execute_transaction(view, txn, aux, txn_idx),
        }
    }

    fn is_transaction_dynamic_change_set_capable(t: &Self::Txn) -> bool {
        MockTask::<KeyType<u32>, MockEvent>::is_transaction_dynamic_change_set_capable(t)
    }
}

fn val(byte: u8) -> ValueType {
    ValueType::from_value(vec![byte; 8], true)
}

#[test]
fn rayon_pool_par_iter_no_deadlock() {
    let coord = Arc::new(Coord::default());
    *COORD.lock().unwrap() = Some(Arc::clone(&coord));

    // txn 0 writes K_INV (after waiting for txn1's first read).
    let txn0 = MockTransaction::from_behavior(MockIncarnation::new(
        vec![],
        vec![(K_INV, val(0), false)],
        vec![],
        vec![],
        1,
    ));
    // txn 1 reads K_INV, writes K_EST. Aborts on first incarnation when
    // txn 0 commits, marking K_EST@1 as an Estimate.
    let txn1 = MockTransaction::from_behavior(MockIncarnation::new(
        vec![(K_INV, false)],
        vec![(K_EST, val(1), false)],
        vec![],
        vec![],
        1,
    ));
    // txn 2 reads K_EST inside a nested par_iter (driven by DeadlockTask).
    let txn2 = MockTransaction::from_behavior(MockIncarnation::new(
        vec![],
        vec![],
        vec![],
        vec![],
        1,
    ));

    let txn_provider = DefaultTxnProvider::new_without_info(vec![txn0, txn1, txn2]);
    let data_view = MockStateView::empty();
    let executor_thread_pool = Arc::new(
        rayon::ThreadPoolBuilder::new()
            .num_threads(2)
            .build()
            .unwrap(),
    );

    let (tx, rx) = mpsc::channel();
    let coord_for_thread = Arc::clone(&coord);
    thread::Builder::new()
        .name("rayon-deadlock".into())
        .spawn(move || {
            let _ = coord_for_thread; // keep coord alive for the run
            let block_executor = BlockExecutor::<
                MockTransaction<KeyType<u32>, MockEvent>,
                DeadlockTask,
                MockStateView<KeyType<u32>>,
                NoOpTransactionCommitHook<usize>,
                DefaultTxnProvider<MockTransaction<KeyType<u32>, MockEvent>, AuxiliaryInfo>,
                AuxiliaryInfo,
            >::new(
                BlockExecutorConfig::new_no_block_limit(2),
                executor_thread_pool,
                None,
            );
            let mut guard = AptosModuleCacheManagerGuard::none();
            let _ = block_executor.execute_transactions_parallel(
                &txn_provider,
                &data_view,
                &TransactionSliceMetadata::unknown(),
                &mut guard,
            );
            let _ = tx.send(());
        })
        .unwrap();

    let result = rx.recv_timeout(Duration::from_secs(60));
    *COORD.lock().unwrap() = None;
    if result.is_err() {
        panic!(
            "BlockExecutor with nested par_iter on the same rayon pool deadlocked - \
             the regression for ad7011f7719c is back: BlockSTM workers must not be \
             rayon workers, otherwise nested par_iter sub-tasks (or rayon yields) \
             can park a BlockSTM worker that is needed to make progress on the txn \
             the stolen task is waiting on. Coord state at timeout: txn1 incarnations \
             {}, txn1_first_done {}, txn1_reexec_started {}, txn2_par_iter_started {}.",
            coord.txn1_reexec_count.load(Ordering::Acquire),
            coord.txn1_first_done.load(Ordering::Acquire),
            coord.txn1_reexec_started.load(Ordering::Acquire),
            coord.txn2_par_iter_started.load(Ordering::Acquire),
        );
    }
    assert!(
        coord.txn1_reexec_count.load(Ordering::Acquire) >= 2,
        "txn1 must be aborted and re-executed for the test to be meaningful, \
         got incarnations: {}",
        coord.txn1_reexec_count.load(Ordering::Acquire),
    );
}
