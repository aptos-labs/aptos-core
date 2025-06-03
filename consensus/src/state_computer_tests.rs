// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::MempoolError, pipeline::pipeline_phase::CountedRequest, state_computer::ExecutionProxy,
    state_replication::StateComputer, transaction_deduper::NoOpDeduper,
    transaction_shuffler::NoOpShuffler, txn_notifier::TxnNotifier,
};
use aptos_consensus_notifications::{ConsensusNotificationSender, Error};
use aptos_consensus_types::{block::Block, block_data::BlockData};
use aptos_crypto::HashValue;
use aptos_executor_types::{
    state_compute_result::StateComputeResult, BlockExecutorTrait, ExecutorResult,
};
use aptos_infallible::Mutex;
use aptos_transactions_filter::{
    transaction_filter::TransactionFilter, transaction_matcher::Filter,
};
use aptos_types::{
    block_executor::{config::BlockExecutorConfigFromOnchain, partitioner::ExecutableBlock},
    contract_event::ContractEvent,
    epoch_state::EpochState,
    ledger_info::LedgerInfoWithSignatures,
    transaction::{SignedTransaction, Transaction, TransactionStatus},
    validator_txn::ValidatorTransaction,
};
use std::{
    sync::{atomic::AtomicU64, Arc},
    time::Duration,
};
use tokio::{runtime::Handle, sync::Mutex as AsyncMutex};

struct DummyStateSyncNotifier {
    invocations: Mutex<Vec<(Vec<Transaction>, Vec<ContractEvent>)>>,
    tx: tokio::sync::mpsc::Sender<()>,
    rx: AsyncMutex<tokio::sync::mpsc::Receiver<()>>,
}

impl DummyStateSyncNotifier {
    fn new() -> Self {
        let (tx, rx) = tokio::sync::mpsc::channel(10);
        Self {
            invocations: Mutex::new(vec![]),
            tx,
            rx: AsyncMutex::new(rx),
        }
    }

    async fn wait_for_notification(&self) {
        self.rx.lock().await.recv().await;
    }
}

#[async_trait::async_trait]
impl ConsensusNotificationSender for DummyStateSyncNotifier {
    async fn notify_new_commit(
        &self,
        transactions: Vec<Transaction>,
        subscribable_events: Vec<ContractEvent>,
    ) -> Result<(), Error> {
        self.invocations
            .lock()
            .push((transactions, subscribable_events));
        self.tx.send(()).await.unwrap();
        Ok(())
    }

    async fn sync_for_duration(
        &self,
        _duration: Duration,
    ) -> Result<LedgerInfoWithSignatures, Error> {
        Err(Error::UnexpectedErrorEncountered(
            "sync_for_duration() is not supported by the DummyStateSyncNotifier!".into(),
        ))
    }

    async fn sync_to_target(&self, _target: LedgerInfoWithSignatures) -> Result<(), Error> {
        unreachable!()
    }
}

struct DummyTxnNotifier {}

#[async_trait::async_trait]
impl TxnNotifier for DummyTxnNotifier {
    async fn notify_failed_txn(
        &self,
        _txns: &[SignedTransaction],
        _statuses: &[TransactionStatus],
    ) -> anyhow::Result<(), MempoolError> {
        Ok(())
    }
}

struct DummyBlockExecutor {
    blocks_received: Mutex<Vec<ExecutableBlock>>,
}

impl DummyBlockExecutor {
    fn new() -> Self {
        Self {
            blocks_received: Mutex::new(vec![]),
        }
    }
}

impl BlockExecutorTrait for DummyBlockExecutor {
    fn committed_block_id(&self) -> HashValue {
        HashValue::zero()
    }

    fn reset(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn execute_and_update_state(
        &self,
        block: ExecutableBlock,
        _parent_block_id: HashValue,
        _onchain_config: BlockExecutorConfigFromOnchain,
    ) -> ExecutorResult<()> {
        self.blocks_received.lock().push(block);
        Ok(())
    }

    fn ledger_update(
        &self,
        _block_id: HashValue,
        _parent_block_id: HashValue,
    ) -> ExecutorResult<StateComputeResult> {
        let txns = self
            .blocks_received
            .lock()
            .last()
            .unwrap()
            .transactions
            .clone()
            .into_txns()
            .into_iter()
            .map(|t| t.into_inner())
            .collect();

        Ok(StateComputeResult::new_dummy_with_input_txns(txns))
    }

    fn pre_commit_block(&self, _block_id: HashValue) -> ExecutorResult<()> {
        Ok(())
    }

    fn commit_ledger(
        &self,
        _ledger_info_with_sigs: LedgerInfoWithSignatures,
    ) -> ExecutorResult<()> {
        Ok(())
    }

    fn finish(&self) {}
}

#[tokio::test]
#[cfg(test)]
async fn should_see_and_notify_validator_txns() {
    use crate::payload_manager::DirectMempoolPayloadManager;

    let executor = Arc::new(DummyBlockExecutor::new());

    let state_sync_notifier = Arc::new(DummyStateSyncNotifier::new());
    let execution_policy = ExecutionProxy::new(
        executor.clone(),
        Arc::new(DummyTxnNotifier {}),
        state_sync_notifier.clone(),
        &Handle::current(),
        TransactionFilter::new(Filter::empty()),
        true,
    );

    let validator_txn_0 = ValidatorTransaction::dummy(vec![0xFF; 99]);
    let validator_txn_1 = ValidatorTransaction::dummy(vec![0xFF; 999]);

    let block = Block::new_for_testing(
        HashValue::zero(),
        BlockData::dummy_with_validator_txns(vec![
            validator_txn_0.clone(),
            validator_txn_1.clone(),
        ]),
        None,
    );

    let epoch_state = EpochState::empty();

    execution_policy.new_epoch(
        &epoch_state,
        Arc::new(DirectMempoolPayloadManager::new()),
        Arc::new(NoOpShuffler {}),
        BlockExecutorConfigFromOnchain::new_no_block_limit(),
        Arc::new(NoOpDeduper {}),
        false,
        false,
    );

    // Ensure the dummy executor has received the txns.
    let _ = execution_policy
        .schedule_compute(&block, HashValue::zero(), None, None, dummy_guard())
        .await
        .await
        .unwrap();

    // Get the txns from the view of the dummy executor.
    let txns = executor.blocks_received.lock()[0]
        .transactions
        .clone()
        .into_txns();

    let supposed_validator_txn_0 = txns[1].expect_valid().try_as_validator_txn().unwrap();
    let supposed_validator_txn_1 = txns[2].expect_valid().try_as_validator_txn().unwrap();
    assert_eq!(&validator_txn_0, supposed_validator_txn_0);
    assert_eq!(&validator_txn_1, supposed_validator_txn_1);

    // Get all txns that state sync was notified with (when pre-commit finishes)
    state_sync_notifier.wait_for_notification().await;
    let (txns, _) = state_sync_notifier.invocations.lock()[0].clone();

    let supposed_validator_txn_0 = txns[1].try_as_validator_txn().unwrap();
    let supposed_validator_txn_1 = txns[2].try_as_validator_txn().unwrap();
    assert_eq!(&validator_txn_0, supposed_validator_txn_0);
    assert_eq!(&validator_txn_1, supposed_validator_txn_1);
}

fn dummy_guard() -> CountedRequest<()> {
    CountedRequest::new((), Arc::new(AtomicU64::new(0)))
}
