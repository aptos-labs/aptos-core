// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    executor::BlockExecutor,
    task::{ExecutionStatus, ExecutorTask, TransactionOutput},
    unit_tests::{AptosModuleCacheManagerGuard, DefaultTxnProvider, NoOpTransactionCommitHook},
};
use aptos_aggregator::{
    delayed_change::DelayedChange, delta_change_set::DeltaOp, resolver::TAggregatorV1View,
};
use aptos_mvhashmap::types::TxnIndex;
use aptos_types::{
    account_address::AccountAddress,
    block_executor::config::BlockExecutorConfig,
    contract_event::TransactionEvent,
    error::PanicError,
    executable::ModulePath,
    fee_statement::FeeStatement,
    state_store::{
        state_value::{StateValue, StateValueMetadata},
        MockStateView, TStateView,
    },
    transaction::BlockExecutableTransaction,
    vm_status::StatusCode,
    write_set::{TransactionWrite, WriteOp, WriteOpKind},
};
use aptos_vm_environment::environment::AptosEnvironment;
use aptos_vm_types::{
    module_write_set::ModuleWrite,
    resolver::{ResourceGroupSize, TExecutorView, TResourceGroupView},
};
use bytes::Bytes;
use move_core_types::{identifier::IdentStr, value::MoveTypeLayout};
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
use num_cpus;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::{
    collections::{BTreeMap, HashSet},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};
use test_case::test_case;

#[derive(Clone, Copy, Hash, Debug, PartialEq, PartialOrd, Ord, Eq)]
struct TestKey {
    key: usize,
}

#[derive(Clone)]
struct TestTransaction {
    write_keys: Vec<TestKey>,
    read_key: TestKey,
}

const STORAGE_VALUE: usize = usize::MAX;

#[derive(Clone, Debug, Eq, PartialEq)]
struct TestValue {
    txn_idx: usize,
}

static BYTES: Bytes = Bytes::from_static(b"");

impl TransactionWrite for TestValue {
    fn bytes(&self) -> Option<&Bytes> {
        Some(&BYTES)
    }

    fn from_state_value(_maybe_state_value: Option<StateValue>) -> Self {
        // Should only be used on storage values in the test.
        TestValue {
            txn_idx: STORAGE_VALUE,
        }
    }

    fn write_op_kind(&self) -> WriteOpKind {
        WriteOpKind::Creation
    }

    fn as_state_value(&self) -> Option<StateValue> {
        Some(StateValue::new_legacy(self.bytes().unwrap().clone()))
    }

    fn set_bytes(&mut self, _bytes: Bytes) {
        unimplemented!("Unused in the test")
    }
}

impl ModulePath for TestKey {
    fn is_module_path(&self) -> bool {
        false
    }

    fn from_address_and_module_name(_address: &AccountAddress, _module_name: &IdentStr) -> Self {
        unimplemented!("Unused in the test")
    }
}

#[derive(Clone, Debug)]
struct TestEvent();

impl TransactionEvent for TestEvent {
    fn get_event_data(&self) -> &[u8] {
        unimplemented!("Unused in the test")
    }
    fn set_event_data(&mut self, _event_data: Vec<u8>) {
        unimplemented!("Unused in the test")
    }
}

impl BlockExecutableTransaction for TestTransaction {
    type Event = TestEvent;
    type Key = TestKey;
    type Tag = ();
    type Value = TestValue;

    fn user_txn_bytes_len(&self) -> usize {
        1
    }
}

#[derive(Debug)]
struct TestOutput {
    writes: Vec<(TestKey, TestValue)>,
}

impl TransactionOutput for TestOutput {
    type Txn = TestTransaction;

    fn resource_write_set(&self) -> Vec<(TestKey, Arc<TestValue>, Option<Arc<MoveTypeLayout>>)> {
        self.writes
            .clone()
            .into_iter()
            .map(|(key, value)| (key, Arc::new(value), None))
            .collect()
    }

    fn module_write_set(&self) -> BTreeMap<TestKey, ModuleWrite<TestValue>> {
        BTreeMap::new()
    }

    fn aggregator_v1_write_set(&self) -> BTreeMap<TestKey, TestValue> {
        BTreeMap::new()
    }

    fn aggregator_v1_delta_set(&self) -> Vec<(TestKey, DeltaOp)> {
        Vec::new()
    }

    fn delayed_field_change_set(&self) -> BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>> {
        BTreeMap::new()
    }

    fn reads_needing_delayed_field_exchange(
        &self,
    ) -> Vec<(TestKey, StateValueMetadata, Arc<MoveTypeLayout>)> {
        Vec::new()
    }

    fn group_reads_needing_delayed_field_exchange(&self) -> Vec<(TestKey, StateValueMetadata)> {
        Vec::new()
    }

    fn get_events(&self) -> Vec<(TestEvent, Option<MoveTypeLayout>)> {
        Vec::new()
    }

    fn resource_group_write_set(
        &self,
    ) -> Vec<(
        TestKey,
        TestValue,
        ResourceGroupSize,
        BTreeMap<(), (TestValue, Option<Arc<MoveTypeLayout>>)>,
    )> {
        Vec::new()
    }

    fn resource_group_metadata_ops(&self) -> Vec<(TestKey, TestValue)> {
        Vec::new()
    }

    // Used due to output initialization in block executor, o.w. shouldn't occur in the test.
    fn skip_output() -> Self {
        TestOutput { writes: vec![] }
    }

    fn discard_output(_discard_code: StatusCode) -> Self {
        unimplemented!("Discarded outputs unused in the test")
    }

    fn materialize_agg_v1(&self, _view: &impl TAggregatorV1View<Identifier = TestKey>) {
        unimplemented!("AggregatorV1 outputs unused in the test")
    }

    fn incorporate_materialized_txn_output(
        &self,
        aggregator_v1_writes: Vec<(TestKey, WriteOp)>,
        patched_resource_write_set: Vec<(TestKey, TestValue)>,
        patched_events: Vec<TestEvent>,
    ) -> Result<(), PanicError> {
        assert!(aggregator_v1_writes.is_empty());
        assert!(patched_resource_write_set.is_empty());
        assert!(patched_events.is_empty());
        Ok(())
    }

    fn set_txn_output_for_non_dynamic_change_set(&self) {}

    fn fee_statement(&self) -> FeeStatement {
        FeeStatement::new(2, 1, 1, 0, 0)
    }

    fn output_approx_size(&self) -> u64 {
        1
    }

    fn get_write_summary(&self) -> HashSet<crate::types::InputOutputKey<TestKey, ()>> {
        HashSet::new()
    }
}

struct MockTask {}

impl MockTask {
    pub fn new() -> Self {
        Self {}
    }
}

// TODO: Should be a member of TestExecutor, passed as a generic environment.
static MOCK_EXECUTION_TIME_US: AtomicU64 = AtomicU64::new(0);

impl ExecutorTask for MockTask {
    type Error = usize;
    type Output = TestOutput;
    type Txn = TestTransaction;

    fn init(_env: &AptosEnvironment, _state_view: &impl TStateView<Key = TestKey>) -> Self {
        Self::new()
    }

    fn is_transaction_dynamic_change_set_capable(_txn: &Self::Txn) -> bool {
        true
    }

    fn execute_transaction(
        &self,
        view: &(impl TExecutorView<TestKey, (), MoveTypeLayout, TestValue>
              + TResourceGroupView<GroupKey = TestKey, ResourceTag = (), Layout = MoveTypeLayout>),
        txn: &Self::Txn,
        txn_idx: TxnIndex,
    ) -> ExecutionStatus<Self::Output, Self::Error> {
        thread::sleep(Duration::from_micros(
            MOCK_EXECUTION_TIME_US.load(Ordering::Relaxed),
        ));

        let _ = view.get_resource_state_value(&txn.read_key, None);

        ExecutionStatus::Success(TestOutput {
            writes: txn
                .write_keys
                .iter()
                .map(|key| {
                    (
                        key.clone(),
                        TestValue {
                            txn_idx: txn_idx as usize,
                        },
                    )
                })
                .collect(),
        })
    }
}

#[test_case(1, 1000, 10, 50, 200)]
fn barrier_workload_v1(
    num_workers: u32,
    num_txns: u32,
    num_keys: usize,
    barrier_interval: u32,
    mock_execution_time_us: u64,
) {
    MOCK_EXECUTION_TIME_US.store(mock_execution_time_us, Ordering::Relaxed);
    let num_cpus = num_cpus::get();
    if num_workers as usize > num_cpus {
        println!("Number of specified workers {num_workers} > number of cpus {num_cpus}");
        // Ideally, we would want:
        // https://users.rust-lang.org/t/how-to-programatically-ignore-a-unit-test/64096/5
        return;
    }

    let mut times_us: Vec<usize> = vec![];
    for seed in 0..7 {
        let mut r = StdRng::seed_from_u64(seed);

        let transactions = (0..num_txns)
            .map(|idx| TestTransaction {
                write_keys: if idx % barrier_interval == 0 {
                    (0..num_keys)
                        .into_iter()
                        .map(|key| TestKey { key })
                        .collect()
                } else {
                    vec![TestKey {
                        key: r.gen_range(0, num_keys),
                    }]
                },
                read_key: TestKey {
                    key: r.gen_range(0, num_keys),
                },
            })
            .collect();
        let txn_provider = DefaultTxnProvider::new(transactions);

        let executor_thread_pool = Arc::new(
            rayon::ThreadPoolBuilder::new()
                .num_threads(num_workers as usize)
                .build()
                .unwrap(),
        );

        let mut guard = AptosModuleCacheManagerGuard::none();
        let state_view = MockStateView::empty();

        let start_time = Instant::now();

        let _ = BlockExecutor::<
            TestTransaction,
            MockTask,
            MockStateView<TestKey>,
            NoOpTransactionCommitHook<TestOutput, usize>,
            DefaultTxnProvider<TestTransaction>,
        >::new(
            BlockExecutorConfig::new_maybe_block_limit(num_cpus::get(), None),
            executor_thread_pool.clone(),
            None,
        )
        .execute_transactions_parallel(&txn_provider, &state_view, &mut guard)
        .unwrap();

        let execution_time = start_time.elapsed().as_micros();
        times_us.push(execution_time.try_into().unwrap());
    }

    // Times reported in order of measurements. We can ignore e.g. first two as warm-ups.
    println!(
        "Barrier workload V1 execution Summary:\n    num_workers {num_workers}\n    \
	     num_txns {num_txns}\n    num_keys {num_keys}\n    barrier_interval \
	     {barrier_interval}\n    execution time {mock_execution_time_us}\
	     \ntimes in microseconds: {:?}\n",
        times_us
    );
}
