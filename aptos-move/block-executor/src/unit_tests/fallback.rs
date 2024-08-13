// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::task::{ExecutionStatus, ExecutorTask, TransactionOutput};
use aptos_aggregator::{
    delayed_change::DelayedChange, delta_change_set::DeltaOp, resolver::TAggregatorV1View,
};
use aptos_mvhashmap::types::{Incarnation, TxnIndex};
use aptos_types::{
    account_address::AccountAddress,
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
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

#[derive(Clone)]
struct TestTransaction {
    id: u8,
    status: Arc<AtomicU64>,
}

#[derive(Clone, Copy, Hash, Debug, PartialEq, PartialOrd, Ord, Eq)]
enum TestKey {
    A,
    B,
    C,
    D,
    Module,
}

#[derive(Clone, Debug)]
enum TestValue {
    None, // Used to represent empty storage slots, i.e. 'deletion' write.
    SpeculativeWrongValue,
    FinalCorrectValue,
}

impl TransactionWrite for TestValue {
    fn bytes(&self) -> Option<&Bytes> {
        // We do not call get_x_bytes() in the test.
        unimplemented!("Unused in the test")
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
            TestValue::SpeculativeWrongValue => Some(StateValue::new_legacy(vec![0u8].into())),
            TestValue::FinalCorrectValue => Some(StateValue::new_legacy(vec![1u8].into())),
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
    type Identifier = DelayedFieldID; // To satisfy the trait requirements.
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
        unimplemented!("Module writes unused in the test")
    }

    fn aggregator_v1_write_set(&self) -> BTreeMap<TestKey, TestValue> {
        unimplemented!("Aggregator V1 writes unused in the test")
    }

    fn aggregator_v1_delta_set(&self) -> Vec<(TestKey, DeltaOp)> {
        unimplemented!("Aggregator V1 deltas Unused in the test")
    }

    fn delayed_field_change_set(&self) -> BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>> {
        unimplemented!("Delayed fields unused in the test")
    }

    fn reads_needing_delayed_field_exchange(
        &self,
    ) -> Vec<(TestKey, StateValueMetadata, Arc<MoveTypeLayout>)> {
        unimplemented!("Delayed fields unused in the test")
    }

    fn group_reads_needing_delayed_field_exchange(&self) -> Vec<(TestKey, StateValueMetadata)> {
        unimplemented!("Delayed fields unused in the test")
    }

    fn get_events(&self) -> Vec<(TestEvent, Option<MoveTypeLayout>)> {
        unimplemented!("Events unused in the test")
    }

    fn resource_group_write_set(
        &self,
    ) -> Vec<(
        TestKey,
        TestValue,
        BTreeMap<(), (TestValue, Option<Arc<MoveTypeLayout>>)>,
    )> {
        unimplemented!("Groups unused in the test")
    }

    fn resource_group_metadata_ops(&self) -> Vec<(TestKey, TestValue)> {
        unimplemented!("Groups unused in the test")
    }

    fn skip_output() -> Self {
        unimplemented!("Skip output unused in the test")
    }

    fn discard_output(_discard_code: StatusCode) -> Self {
        unimplemented!("Discarded outputs unused in the test")
    }

    fn materialize_agg_v1(&self, view: &impl TAggregatorV1View<Identifier = TestKey>) {
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

#[derive(Default)]
struct TestExecutor {
    // Test the optimization (currently for module reads), where execution halts
    // if the fallback has already finished. Provided as the block executor environment.
    test_read_error: bool,
}

// TODO: picture and description of the test case.

impl ExecutorTask for TestExecutor {
    type Environment = bool;
    type Error = usize;
    type Output = TestOutput;
    type Txn = TestTransaction;

    fn init(env: bool, _state_view: &impl TStateView<Key = TestKey>) -> Self {
        TestExecutor {
            test_read_error: env,
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
        is_fallback: bool,
    ) -> ExecutionStatus<Self::Output, Self::Error> {
        let mut writes = vec![];
        match txn.id {
            0 => {
                assert_eq!(txn_idx, 0, "Algorithm for TXN 0");

                assert_eq!(incarnation, 0);
                assert_eq!(is_fallback, false);

                loop {
                    let status = txn.status.load(Ordering::Relaxed);
                    if status != DEFAULT_STATUS {
                        assert_gt!(
                            status & TXN1_READ_FLAG + status & TXN2_READ_FLAG,
                            0,
                            "TXN1 and TXN2 perform a read first, set flag",
                        );
                        break;
                    }
                }

                writes.push((TestKey::A, TestValue::FinalCorrectValue));
            },
            1 => {
                assert_eq!(txn_idx, 1, "Algorithm for TXN 1");
                assert_eq!(is_fallback, false);

                match incarnation {
                    0 => {
                        // Key A should contain no values at this point, as TXN0 waits
                        // for the read flag set below, before it writes.
                        assert_none!(view.get_resource_state_value(&TestKey::A, None).unwrap());

                        let mut prev_status =
                            txn.status.fetch_xor(TXN1_READ_FLAG, Ordering::Relaxed);
                        assert!(prev_status == DEFAULT_STATUS || prev_status == TXN2_READ_FLAG);

                        writes.push((TestKey::B, TestValue::SpeculativeWrongValue));

                        // Do not finish execution until TXN2 reads key C, guaranteeing the read
                        // may not observe the write by incarnation 1 that starts after.
                        while prev_status & TXN2_READ_FLAG == 0 {
                            prev_status = txn.status.load(Ordering::Relaxed);
                        }
                    },
                    1 => {
                        let mut prev_status =
                            txn.status.fetch_xor(TXN1_ABORTED_FLAG, Ordering::Relaxed);
                        assert_eq!(
                            prev_status & TXN1_READ_FLAG,
                            1,
                            "TXN1 prior incarnation must set the read flag"
                        );
                        assert_eq!(
                            prev_status & TXN2_READ_FLAG,
                            1,
                            "TXN1 prior incarnation waits to finish for TXN2 read flag set"
                        );

                        writes.push((TestKey::C, TestValue::FinalCorrectValue));
                    },
                    _ => unreachable!("Incarnation for TXN 1 should be 0 or 1"),
                }
            },
            2 => {
                assert_eq!(txn_idx, 2, "Algorithm for TXN 2");

                // First execution of TXN2 and fallback both have incarnation 0.
                assert_eq!(incarnation, 0);

                match fallback {
                    false => {
                        // Key C should contain no values here, as TXN1 waits for the read flag
                        // set below, before it (starts incarnation that) writes to key C.
                        assert_none!(view.get_resource_state_value(&TestKey::C, None).unwrap());

                        let mut prev_status =
                            txn.status.fetch_xor(TXN2_READ_FLAG, Ordering::Relaxed);
                        assert!(prev_status == DEFAULT_STATUS || prev_status == TXN1_READ_FLAG);

                        writes.push((TestKey::D, TestValue::SpeculativeWrongValue));

                        // Waiting for TXN1 aborted flag to be set guarantees that the following
                        // read from key B would not observe the write from TXN1 incarnation 0,
                        // instead potentially waiting for the corresponding estimate, and
                        // eventually observing no write to key B by TXN1 incarnation 1.
                        while prev_status & TXN1_ABORTED_FLAG == 0 {
                            prev_status = txn.status.load(Ordering::Relaxed);
                        }
                        assert_eq!(
                            prev_status & TXN1_READ_FLAG,
                            1,
                            "TXN1 read flag is set before aborted flag"
                        );
                        assert_eq!(view.get_resource_state_value(&TestKey::B, None).unwrap());

                        // Stay in 'Executing' state, allowing the fallback execution to start
                        // after TXN1 is committed.
                        while prev_status & TXN2_FALLBACK_STARTED_FLAG == 0 {
                            prev_status = txn.status.load(Ordering::Relaxed);
                        }
                        assert_eq!(
                            (prev_status & TXN1_READ_FLAG)
                                + (prev_status & TXN2_READ_FLAG)
                                + (prev_status & TXN1_ABORTED_FLAG)
                                + (prev_status & TXN2_FALLBACK_STARTED_FLAG),
                            4,
                            "All flags must be set",
                        );

                        if self.test_read_error {
                            // self.commit_hook
                            // TransactionCommitHook<Output = AptosTransactionOutput>,
                            // after 2 is committed, should
                            // TODO: halt error from fetch_module if we wait for fallback to finish.
                        }
                    },
                    true => {
                        let prev_status = txn
                            .status
                            .fetch_xor(TXN2_FALLBACK_STARTED_FLAG, Ordering::Relaxed);
                        assert_eq!(
                            (prev_status & TXN1_READ_FLAG)
                                + (prev_status & TXN2_READ_FLAG)
                                + (prev_status & TXN1_ABORTED_FLAG)
                            3,
                            "All flags must be set"
                        );

                        // Fallback must read the correct value, written by TXN1 incarnation 1.
                        let c_state_value = view
                            .get_resource_state_value(&TestKey::C, None)
                            .unwrap()
                            .unwrap();
                        assert_eq!(
                            state_value.bytes().as_bytes()[0],
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

const DEFAULT_STATUS: u64 = 0;

// The flags are values corresponding to 1-bit binary representations.
const TXN1_READ_FLAG: u64 = 1;
const TXN2_READ_FLAG: u64 = 2;
const TXN1_ABORTED_FLAG: u64 = 4;
const TXN2_FALLBACK_STARTED_FLAG: u64 = 8;

// TODO: only run test when there are at least two threads.
