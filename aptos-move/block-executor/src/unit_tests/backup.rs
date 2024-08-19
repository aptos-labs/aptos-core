// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    executor::BlockExecutor,
    proptest_types::types::EmptyDataView,
    task::{ExecutionStatus, ExecutorTask, TransactionOutput},
    txn_commit_hook::TransactionCommitHook,
};
use aptos_aggregator::{
    delayed_change::DelayedChange, delta_change_set::DeltaOp, resolver::TAggregatorV1View,
};
use aptos_mvhashmap::types::{Incarnation, TxnIndex};
use aptos_types::{
    account_address::AccountAddress,
    block_executor::config::{BlockExecutorConfig, BlockSTMCommitterBackup},
    contract_event::TransactionEvent,
    delayed_fields::PanicError,
    executable::ModulePath,
    fee_statement::FeeStatement,
    state_store::{
        state_value::{StateValue, StateValueMetadata},
        TStateView,
    },
    transaction::BlockExecutableTransaction,
    vm_status::StatusCode,
    write_set::{TransactionWrite, WriteOp, WriteOpKind},
};
use aptos_vm_types::resolver::{TExecutorView, TResourceGroupView};
use bytes::Bytes;
use claims::assert_none;
use move_core_types::{identifier::IdentStr, value::MoveTypeLayout};
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
use std::{
    collections::{BTreeMap, HashSet},
    marker::PhantomData,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
use test_case::test_case;

const DEFAULT_STATUS: u64 = 0;

// The flags are values corresponding to 1-bit binary representations.
const TXN1_READ_FLAG: u64 = 1;
const TXN2_READ_FLAG: u64 = 2;
const TXN1_ABORTED_FLAG: u64 = 4;
const TXN2_BACKUP_STARTED_FLAG: u64 = 8;

#[derive(Clone)]
struct TestTransaction {
    id: u8,
}

#[derive(Clone, Copy, Hash, Debug, PartialEq, PartialOrd, Ord, Eq)]
enum TestKey {
    A,
    B,
    C,
    D,
    Module,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum TestValue {
    None, // Used to represent empty storage slots, i.e. 'deletion' write.
    SpeculativeWrongValue(Bytes),
    FinalCorrectValue(Bytes),
}

impl TestValue {
    fn speculative_wrong_value() -> Self {
        TestValue::SpeculativeWrongValue(vec![0u8].into()) // Contains '0'
    }

    fn final_correct_value() -> Self {
        TestValue::FinalCorrectValue(vec![1u8].into()) // Contains '1'
    }
}

impl TransactionWrite for TestValue {
    fn bytes(&self) -> Option<&Bytes> {
        match self {
            TestValue::None => None,
            TestValue::SpeculativeWrongValue(bytes) | TestValue::FinalCorrectValue(bytes) => {
                Some(bytes)
            },
        }
    }

    fn from_state_value(maybe_state_value: Option<StateValue>) -> Self {
        // Should only be used on storage values in the test.
        assert_none!(maybe_state_value);
        TestValue::None
    }

    fn write_op_kind(&self) -> WriteOpKind {
        WriteOpKind::Creation
    }

    fn as_state_value(&self) -> Option<StateValue> {
        match self {
            TestValue::None => None,
            TestValue::SpeculativeWrongValue(bytes) | TestValue::FinalCorrectValue(bytes) => {
                Some(StateValue::new_legacy(bytes.clone()))
            },
        }
    }

    fn set_bytes(&mut self, _bytes: Bytes) {
        unimplemented!("Unused in the test")
    }
}

impl ModulePath for TestKey {
    fn is_module_path(&self) -> bool {
        matches!(self, TestKey::Module)
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
    type Identifier = DelayedFieldID;
    // To satisfy the trait requirements.
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

    fn module_write_set(&self) -> BTreeMap<TestKey, TestValue> {
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

    fn get_write_summary(
        &self,
    ) -> HashSet<crate::types::InputOutputKey<TestKey, (), DelayedFieldID>> {
        HashSet::new()
    }
}

struct TestCommitHook {
    next_to_commit_idx: Arc<AtomicU64>,
}

impl TransactionCommitHook for TestCommitHook {
    type Output = TestOutput;

    fn on_transaction_committed(&self, txn_idx: TxnIndex, _output: &Self::Output) {
        self.next_to_commit_idx
            .fetch_max(txn_idx as u64 + 1, Ordering::Relaxed);
    }

    fn on_execution_aborted(&self, _txn_idx: TxnIndex) {
        // no-op
    }
}

#[derive(Default)]
struct TestExecutor {
    // When we test the optimization (currently for module reads), where execution halts
    // if the backup has already finished, we provided Some(next_idx_to_commit) via
    // the block executor's environment parameter (passed down to ExecutorTask's init).
    // In this case, block execution gets a TestCommitHook that updates the atomic index.
    maybe_next_idx_to_commit: Option<Arc<AtomicU64>>,
    synchronization_status: Arc<AtomicU64>,
}

// TODO: picture and description of the test case.

impl ExecutorTask for TestExecutor {
    type Environment = (Option<Arc<AtomicU64>>, Arc<AtomicU64>);
    type Error = usize;
    type Output = TestOutput;
    type Txn = TestTransaction;

    fn init(
        env: (Option<Arc<AtomicU64>>, Arc<AtomicU64>),
        _state_view: &impl TStateView<Key = TestKey>,
    ) -> Self {
        TestExecutor {
            maybe_next_idx_to_commit: env.0,
            synchronization_status: env.1,
        }
    }

    fn is_transaction_dynamic_change_set_capable(_txn: &Self::Txn) -> bool {
        true
    }

    fn execute_transaction(
        &self,
        view: &(impl TExecutorView<TestKey, (), MoveTypeLayout, DelayedFieldID, TestValue>
              + TResourceGroupView<GroupKey = TestKey, ResourceTag = (), Layout = MoveTypeLayout>),
        txn: &Self::Txn,
        txn_idx: TxnIndex,
        incarnation: Incarnation,
        is_backup: bool,
    ) -> ExecutionStatus<Self::Output, Self::Error> {
        let mut writes = vec![];
        match txn.id {
            0 => {
                assert_eq!(txn_idx, 0, "Algorithm for TXN 0");

                assert_eq!(incarnation, 0);
                assert!(!is_backup);

                let target_mask = TXN1_READ_FLAG + TXN2_READ_FLAG;
                loop {
                    let status = self.synchronization_status.load(Ordering::Relaxed);
                    if status & target_mask == target_mask {
                        // TXN1 and TXN2 have started and performed reads.
                        break;
                    }
                }

                writes.push((TestKey::A, TestValue::final_correct_value()));
            },
            1 => {
                assert_eq!(txn_idx, 1, "Algorithm for TXN 1");
                assert!(!is_backup);

                match incarnation {
                    0 => {
                        // Key A should contain no values at this point, as TXN0 waits
                        // for the read flag set below, before it writes.
                        assert_none!(view.get_resource_state_value(&TestKey::A, None).unwrap());

                        let prev_status = self
                            .synchronization_status
                            .fetch_xor(TXN1_READ_FLAG, Ordering::Relaxed);
                        // 2 workers: (0, 0) is waiting for (1,0) and (2,0). (1,0) starts first and
                        // only after (1,0) finishes (2,0) may start.
                        assert!(prev_status == DEFAULT_STATUS);

                        writes.push((TestKey::B, TestValue::speculative_wrong_value()));
                    },
                    1 => {
                        // Do not finish execution until TXN2 reads key C, guaranteeing the read
                        // may not observe the write by incarnation 1 that starts after.
                        while (self.synchronization_status.load(Ordering::Relaxed) & TXN2_READ_FLAG)
                            == 0
                        {}

                        // No write should be visible at B.
                        assert_none!(view.get_resource_state_value(&TestKey::B, None).unwrap());
                        // TODO: check A. and final correct value.

                        let prev_status = self
                            .synchronization_status
                            .fetch_xor(TXN1_ABORTED_FLAG, Ordering::Relaxed);
                        assert_eq!(
                            prev_status & TXN1_READ_FLAG,
                            TXN1_READ_FLAG,
                            "TXN1 prior incarnation must set the read flag"
                        );
                        assert_eq!(
                            prev_status & TXN2_READ_FLAG,
                            TXN2_READ_FLAG,
                            "TXN1 prior incarnation waits to finish for TXN2 read flag set"
                        );

                        writes.push((TestKey::B, TestValue::final_correct_value()));
                        writes.push((TestKey::C, TestValue::final_correct_value()));
                    },
                    _ => unreachable!("Incarnation for TXN 1 should be 0 or 1"),
                }
            },
            2 => {
                assert_eq!(txn_idx, 2, "Algorithm for TXN 2");

                // First execution of TXN2 and backup both have incarnation 0.
                assert_eq!(incarnation, 0);

                match is_backup {
                    false => {
                        // Key C should contain no values here, as TXN1 waits for the read flag
                        // set below, before it (starts incarnation that) writes to key C.
                        assert_none!(view.get_resource_state_value(&TestKey::C, None).unwrap());

                        let mut prev_status = self
                            .synchronization_status
                            .fetch_xor(TXN2_READ_FLAG, Ordering::Relaxed);
                        assert!(prev_status == TXN1_READ_FLAG);

                        writes.push((TestKey::D, TestValue::speculative_wrong_value()));

                        while prev_status & TXN1_ABORTED_FLAG == 0 {
                            prev_status = self.synchronization_status.load(Ordering::Relaxed);
                        }
                        assert_eq!(
                            prev_status & TXN1_READ_FLAG,
                            TXN1_READ_FLAG,
                            "TXN1 read flag is set before aborted flag"
                        );
                        // Waiting for TXN1 aborted flag to be set above guarantees that the
                        // read from key B below may not observe the write from TXN1 incarnation 0,
                        // but will instead observe a dependency (an Estimate in MVHashMap).
                        // Even if the dependency is resolved, the other worker (since there are 2)
                        // that commits TXN1 will start backup execution immediately after
                        // (before signaling the suspended Read here). finish_execution of the
                        // backup will do the signal, providing an SpeculativeAbort error.
                        //
                        match view.get_resource_state_value(&TestKey::B, None) {
                            Err(e) => {
                                assert_eq!(
                                    e.major_status(),
                                    StatusCode::SPECULATIVE_EXECUTION_ABORT_ERROR
                                );
                            },
                            Ok(maybe_state_value) => {
                                // There is a slim chance (as (2, 0) starts before (1, 1)), that
                                // Read happens after (1,1) has completed. In this case, it will
                                // read the value from (1,1).
                                assert!(
                                    maybe_state_value.unwrap().bytes().is_empty(),
                                    "Must be encoding of FinalCorrectValue2"
                                );
                            },
                        }

                        // Stay in 'Executing' state, allowing the backup execution to start
                        // after TXN1 is committed.
                        while prev_status & TXN2_BACKUP_STARTED_FLAG == 0 {
                            prev_status = self.synchronization_status.load(Ordering::Relaxed);
                        }
                        let target_mask = TXN1_READ_FLAG
                            + TXN2_READ_FLAG
                            + TXN1_ABORTED_FLAG
                            + TXN2_BACKUP_STARTED_FLAG;
                        assert_eq!(
                            prev_status & target_mask,
                            target_mask,
                            "Txn 2: all flags must be set",
                        );

                        if let Some(next_idx_to_commit) = &self.maybe_next_idx_to_commit {
                            while next_idx_to_commit.load(Ordering::Relaxed) < 3 {}

                            // At this point backup must have won, since 2 committed. A call
                            // to get module data should cause an error (to speed up returning).
                            match view.get_module_state_value(&TestKey::Module) {
                                Err(e) => {
                                    assert_eq!(
                                        e.major_status(),
                                        StatusCode::SPECULATIVE_EXECUTION_ABORT_ERROR
                                    );
                                },
                                Ok(_) => unreachable!("Must be speculative abort error"),
                            }
                        }
                    },
                    true => {
                        let prev_status = self
                            .synchronization_status
                            .fetch_xor(TXN2_BACKUP_STARTED_FLAG, Ordering::Relaxed);
                        let target_mask = TXN1_READ_FLAG + TXN2_READ_FLAG + TXN1_ABORTED_FLAG;
                        assert_eq!(
                            prev_status & target_mask,
                            target_mask,
                            "TXN2 backup: all flags must be set",
                        );

                        // Backup must read the correct value, written by TXN1 incarnation 1.
                        let c_state_value = view
                            .get_resource_state_value(&TestKey::C, None)
                            .unwrap()
                            .unwrap();
                        assert_eq!(
                            c_state_value.bytes()[0],
                            1,
                            "Must be the as_state_value() encoding of FinalCorrectValue"
                        );
                    },
                }
            },
            _ => {
                unreachable!("There should be only 3 txn ids");
            },
        }

        ExecutionStatus::Success(TestOutput { writes })
    }
}

// When with_commit_hook is true, Executor is provided with a hook to notify of committed
// transactions, allowing to test additional behavior. However, 'false' case is still interesting
// as it allows for more concurrent interleavings between the execution and its backup.
#[test_case(false)]
#[test_case(true)]
fn test_commit_backup(with_commit_hook: bool) {
    if num_cpus::get() < 2 {
        return;
    }
    let num_workers = 2;

    let next_to_commit_idx_local = Arc::new(AtomicU64::new(0));
    let maybe_commit_hook = with_commit_hook.then(|| TestCommitHook {
        next_to_commit_idx: next_to_commit_idx_local.clone(),
    });
    let transactions = Vec::from([
        TestTransaction { id: 0 },
        TestTransaction { id: 1 },
        TestTransaction { id: 2 },
    ]);

    let data_view = EmptyDataView::<TestKey> {
        phantom: PhantomData,
    };
    let executor_thread_pool = Arc::new(
        rayon::ThreadPoolBuilder::new()
            .num_threads(num_workers)
            .build()
            .unwrap(),
    );

    let mut config = BlockExecutorConfig::new_no_block_limit(num_workers);
    // Set a setting that allows commits to happen despite the test suspending
    // transaction execution by some (unpredictable) workers.
    config.local.block_stm_committer_backup = BlockSTMCommitterBackup::All;

    let synchronization_status = Arc::new(AtomicU64::new(DEFAULT_STATUS));

    let binding = BlockExecutor::<
        TestTransaction,
        TestExecutor,
        EmptyDataView<TestKey>,
        TestCommitHook,
    >::new(config, executor_thread_pool.clone(), maybe_commit_hook)
    .execute_transactions_parallel(
        &(
            with_commit_hook.then_some(next_to_commit_idx_local),
            synchronization_status,
        ),
        &transactions,
        &data_view,
    )
    .unwrap();
    let output = binding.get_transaction_outputs_forced();

    assert_eq!(output.len(), 3);
    assert_eq!(output[0].writes.len(), 1);
    assert_eq!(output[0].writes[0].0, TestKey::A);
    assert_eq!(output[0].writes[0].1, TestValue::final_correct_value());

    assert_eq!(output[1].writes.len(), 2);
    assert_eq!(output[1].writes[0].0, TestKey::B);
    assert_eq!(output[1].writes[0].1, TestValue::final_correct_value());
    assert_eq!(output[1].writes[1].0, TestKey::C);
    assert_eq!(output[1].writes[1].1, TestValue::final_correct_value());

    assert!(output[2].writes.is_empty());
}
