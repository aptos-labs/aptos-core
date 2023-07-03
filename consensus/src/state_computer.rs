// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block_storage::tracing::{observe_block, BlockStage},
    counters,
    error::StateSyncError,
    monitor,
    payload_manager::PayloadManager,
    state_replication::{StateComputer, StateComputerCommitCallBackType},
    transaction_deduper::TransactionDeduper,
    transaction_shuffler::TransactionShuffler,
    txn_notifier::TxnNotifier,
};
use anyhow::Result;
use aptos_consensus_notifications::ConsensusNotificationSender;
use aptos_consensus_types::{block::Block, common::Round, executed_block::ExecutedBlock};
use aptos_crypto::HashValue;
use aptos_executor_types::{BlockExecutorTrait, Error as ExecutionError, StateComputeResult};
use aptos_infallible::Mutex;
use aptos_logger::prelude::*;
use aptos_types::{
    account_address::AccountAddress, contract_event::ContractEvent, epoch_state::EpochState,
    ledger_info::LedgerInfoWithSignatures, transaction::Transaction,
};
use fail::fail_point;
use futures::{SinkExt, StreamExt};
use std::{boxed::Box, sync::Arc};
use tokio::sync::Mutex as AsyncMutex;

type NotificationType = (
    Box<dyn FnOnce() + Send + Sync>,
    Vec<Transaction>,
    Vec<ContractEvent>,
);

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Hash)]
struct LogicalTime {
    epoch: u64,
    round: Round,
}

impl LogicalTime {
    pub fn new(epoch: u64, round: Round) -> Self {
        Self { epoch, round }
    }
}

/// Basic communication with the Execution module;
/// implements StateComputer traits.
pub struct ExecutionProxy {
    executor: Arc<dyn BlockExecutorTrait>,
    txn_notifier: Arc<dyn TxnNotifier>,
    state_sync_notifier: Arc<dyn ConsensusNotificationSender>,
    async_state_sync_notifier: aptos_channels::Sender<NotificationType>,
    validators: Mutex<Vec<AccountAddress>>,
    write_mutex: AsyncMutex<LogicalTime>,
    payload_manager: Mutex<Option<Arc<PayloadManager>>>,
    transaction_shuffler: Mutex<Option<Arc<dyn TransactionShuffler>>>,
    maybe_block_gas_limit: Mutex<Option<u64>>,
    transaction_deduper: Mutex<Option<Arc<dyn TransactionDeduper>>>,
}

impl ExecutionProxy {
    pub fn new(
        executor: Arc<dyn BlockExecutorTrait>,
        txn_notifier: Arc<dyn TxnNotifier>,
        state_sync_notifier: Arc<dyn ConsensusNotificationSender>,
        handle: &tokio::runtime::Handle,
    ) -> Self {
        let (tx, mut rx) =
            aptos_channels::new::<NotificationType>(10, &counters::PENDING_STATE_SYNC_NOTIFICATION);
        let notifier = state_sync_notifier.clone();
        handle.spawn(async move {
            while let Some((callback, txns, reconfig_events)) = rx.next().await {
                if let Err(e) = monitor!(
                    "notify_state_sync",
                    notifier.notify_new_commit(txns, reconfig_events).await
                ) {
                    error!(error = ?e, "Failed to notify state synchronizer");
                }

                callback();
            }
        });
        Self {
            executor,
            txn_notifier,
            state_sync_notifier,
            async_state_sync_notifier: tx,
            validators: Mutex::new(vec![]),
            write_mutex: AsyncMutex::new(LogicalTime::new(0, 0)),
            payload_manager: Mutex::new(None),
            transaction_shuffler: Mutex::new(None),
            maybe_block_gas_limit: Mutex::new(None),
            transaction_deduper: Mutex::new(None),
        }
    }
}

// TODO: filter duplicated transaction before executing
#[async_trait::async_trait]
impl StateComputer for ExecutionProxy {
    async fn compute(
        &self,
        // The block to be executed.
        block: &Block,
        // The parent block id.
        parent_block_id: HashValue,
    ) -> Result<StateComputeResult, ExecutionError> {
        fail_point!("consensus::compute", |_| {
            Err(ExecutionError::InternalError {
                error: "Injected error in compute".into(),
            })
        });
        let block_id = block.id();
        debug!(
            block = %block,
            parent_id = parent_block_id,
            "Executing block",
        );

        let payload_manager = self.payload_manager.lock().as_ref().unwrap().clone();
        let txn_deduper = self.transaction_deduper.lock().as_ref().unwrap().clone();
        let txn_shuffler = self.transaction_shuffler.lock().as_ref().unwrap().clone();
        let txns = payload_manager.get_transactions(block).await?;

        let deduped_txns = txn_deduper.dedup(txns);
        let shuffled_txns = txn_shuffler.shuffle(deduped_txns);

        let block_gas_limit = *self.maybe_block_gas_limit.lock();

        // TODO: figure out error handling for the prologue txn
        let executor = self.executor.clone();

        let transactions_to_execute = block.transactions_to_execute(
            &self.validators.lock(),
            shuffled_txns.clone(),
            block_gas_limit,
        );

        let compute_result = monitor!(
            "execute_block",
            tokio::task::spawn_blocking(move || {
                executor.execute_block(
                    (block_id, transactions_to_execute).into(),
                    parent_block_id,
                    block_gas_limit,
                )
            })
            .await
        )
        .expect("spawn_blocking failed")?;

        observe_block(block.timestamp_usecs(), BlockStage::EXECUTED);

        // notify mempool about failed transaction
        if let Err(e) = self
            .txn_notifier
            .notify_failed_txn(shuffled_txns, &compute_result)
            .await
        {
            error!(
                error = ?e, "Failed to notify mempool of rejected txns",
            );
        }
        Ok(compute_result)
    }

    /// Send a successful commit. A future is fulfilled when the state is finalized.
    async fn commit(
        &self,
        blocks: &[Arc<ExecutedBlock>],
        finality_proof: LedgerInfoWithSignatures,
        callback: StateComputerCommitCallBackType,
    ) -> Result<(), ExecutionError> {
        let mut latest_logical_time = self.write_mutex.lock().await;

        let mut block_ids = Vec::new();
        let mut txns = Vec::new();
        let mut reconfig_events = Vec::new();
        let mut payloads = Vec::new();
        let logical_time = LogicalTime::new(
            finality_proof.ledger_info().epoch(),
            finality_proof.ledger_info().round(),
        );
        let block_timestamp = finality_proof.commit_info().timestamp_usecs();

        let payload_manager = self.payload_manager.lock().as_ref().unwrap().clone();
        let txn_deduper = self.transaction_deduper.lock().as_ref().unwrap().clone();
        let txn_shuffler = self.transaction_shuffler.lock().as_ref().unwrap().clone();

        let block_gas_limit = *self.maybe_block_gas_limit.lock();

        for block in blocks {
            block_ids.push(block.id());

            if let Some(payload) = block.block().payload() {
                payloads.push(payload.clone());
            }

            let signed_txns = payload_manager.get_transactions(block.block()).await?;
            let deduped_txns = txn_deduper.dedup(signed_txns);
            let shuffled_txns = txn_shuffler.shuffle(deduped_txns);

            txns.extend(block.transactions_to_commit(
                &self.validators.lock(),
                shuffled_txns,
                block_gas_limit,
            ));
            reconfig_events.extend(block.reconfig_event());
        }

        let executor = self.executor.clone();
        let proof = finality_proof.clone();
        monitor!(
            "commit_block",
            tokio::task::spawn_blocking(move || {
                executor
                    .commit_blocks_ext(block_ids, proof, false)
                    .expect("Failed to commit blocks");
            })
            .await
        )
        .expect("spawn_blocking failed");

        let blocks = blocks.to_vec();
        let wrapped_callback = move || {
            callback(&blocks, finality_proof);
        };
        self.async_state_sync_notifier
            .clone()
            .send((Box::new(wrapped_callback), txns, reconfig_events))
            .await
            .expect("Failed to send async state sync notification");

        *latest_logical_time = logical_time;
        payload_manager
            .notify_commit(block_timestamp, payloads)
            .await;
        Ok(())
    }

    /// Synchronize to a commit that not present locally.
    async fn sync_to(&self, target: LedgerInfoWithSignatures) -> Result<(), StateSyncError> {
        let mut latest_logical_time = self.write_mutex.lock().await;
        let logical_time =
            LogicalTime::new(target.ledger_info().epoch(), target.ledger_info().round());
        let block_timestamp = target.commit_info().timestamp_usecs();

        // Before the state synchronization, we have to call finish() to free the in-memory SMT
        // held by BlockExecutor to prevent memory leak.
        self.executor.finish();

        // The pipeline phase already committed beyond the target block timestamp, just return.
        if *latest_logical_time >= logical_time {
            warn!(
                "State sync target {:?} is lower than already committed logical time {:?}",
                logical_time, *latest_logical_time
            );
            return Ok(());
        }

        // This is to update QuorumStore with the latest known commit in the system,
        // so it can set batches expiration accordingly.
        // Might be none if called in the recovery path, or between epoch stop and start.
        let maybe_payload_manager = self.payload_manager.lock().as_ref().cloned();
        if let Some(payload_manager) = maybe_payload_manager {
            payload_manager
                .notify_commit(block_timestamp, Vec::new())
                .await;
        }

        fail_point!("consensus::sync_to", |_| {
            Err(anyhow::anyhow!("Injected error in sync_to").into())
        });
        // Here to start to do state synchronization where ChunkExecutor inside will
        // process chunks and commit to Storage. However, after block execution and
        // commitments, the sync state of ChunkExecutor may be not up to date so
        // it is required to reset the cache of ChunkExecutor in State Sync
        // when requested to sync.
        let res = monitor!(
            "sync_to",
            self.state_sync_notifier.sync_to_target(target).await
        );
        *latest_logical_time = logical_time;

        // Similarly, after the state synchronization, we have to reset the cache
        // of BlockExecutor to guarantee the latest committed state is up to date.
        self.executor.reset()?;

        res.map_err(|error| {
            let anyhow_error: anyhow::Error = error.into();
            anyhow_error.into()
        })
    }

    fn new_epoch(
        &self,
        epoch_state: &EpochState,
        payload_manager: Arc<PayloadManager>,
        transaction_shuffler: Arc<dyn TransactionShuffler>,
        block_gas_limit: Option<u64>,
        transaction_deduper: Arc<dyn TransactionDeduper>,
    ) {
        *self.validators.lock() = epoch_state
            .verifier
            .get_ordered_account_addresses_iter()
            .collect();
        self.payload_manager.lock().replace(payload_manager);
        self.transaction_shuffler
            .lock()
            .replace(transaction_shuffler);
        *self.maybe_block_gas_limit.lock() = block_gas_limit;
        self.transaction_deduper.lock().replace(transaction_deduper);
    }

    // Clears the epoch-specific state. Only a sync_to call is expected before calling new_epoch
    // on the next epoch.
    fn end_epoch(&self) {
        *self.validators.lock() = vec![];
        self.payload_manager.lock().take();
    }
}

#[tokio::test]
async fn test_commit_sync_race() {
    use crate::{
        error::MempoolError, transaction_deduper::create_transaction_deduper,
        transaction_shuffler::create_transaction_shuffler,
    };
    use aptos_consensus_notifications::Error;
    use aptos_types::{
        aggregate_signature::AggregateSignature,
        block_executor::partitioner::ExecutableBlock,
        block_info::BlockInfo,
        ledger_info::LedgerInfo,
        on_chain_config::{TransactionDeduperType, TransactionShufflerType},
        transaction::SignedTransaction,
    };

    struct RecordedCommit {
        time: Mutex<LogicalTime>,
    }

    impl BlockExecutorTrait for RecordedCommit {
        fn committed_block_id(&self) -> HashValue {
            HashValue::zero()
        }

        fn reset(&self) -> Result<()> {
            Ok(())
        }

        fn execute_block(
            &self,
            _block: ExecutableBlock<Transaction>,
            _parent_block_id: HashValue,
            _maybe_block_gas_limit: Option<u64>,
        ) -> Result<StateComputeResult, ExecutionError> {
            Ok(StateComputeResult::new_dummy())
        }

        fn commit_blocks_ext(
            &self,
            _block_ids: Vec<HashValue>,
            ledger_info_with_sigs: LedgerInfoWithSignatures,
            _save_state_snapshots: bool,
        ) -> Result<(), ExecutionError> {
            *self.time.lock() = LogicalTime::new(
                ledger_info_with_sigs.ledger_info().epoch(),
                ledger_info_with_sigs.ledger_info().round(),
            );
            Ok(())
        }

        fn finish(&self) {}
    }

    #[async_trait::async_trait]
    impl TxnNotifier for RecordedCommit {
        async fn notify_failed_txn(
            &self,
            _txns: Vec<SignedTransaction>,
            _compute_results: &StateComputeResult,
        ) -> Result<(), MempoolError> {
            Ok(())
        }
    }

    #[async_trait::async_trait]
    impl ConsensusNotificationSender for RecordedCommit {
        async fn notify_new_commit(
            &self,
            _transactions: Vec<Transaction>,
            _reconfiguration_events: Vec<ContractEvent>,
        ) -> std::result::Result<(), Error> {
            Ok(())
        }

        async fn sync_to_target(
            &self,
            target: LedgerInfoWithSignatures,
        ) -> std::result::Result<(), Error> {
            let logical_time =
                LogicalTime::new(target.ledger_info().epoch(), target.ledger_info().round());
            if logical_time <= *self.time.lock() {
                return Err(Error::NotificationError(
                    "Decreasing logical time".to_string(),
                ));
            }
            *self.time.lock() = logical_time;
            Ok(())
        }
    }

    let callback = Box::new(move |_a: &[Arc<ExecutedBlock>], _b: LedgerInfoWithSignatures| {});
    let recorded_commit = Arc::new(RecordedCommit {
        time: Mutex::new(LogicalTime::new(0, 0)),
    });
    let generate_li = |epoch, round| {
        LedgerInfoWithSignatures::new(
            LedgerInfo::new(
                BlockInfo::random_with_epoch(epoch, round),
                HashValue::zero(),
            ),
            AggregateSignature::empty(),
        )
    };
    let executor = ExecutionProxy::new(
        recorded_commit.clone(),
        recorded_commit.clone(),
        recorded_commit.clone(),
        &tokio::runtime::Handle::current(),
    );
    executor.new_epoch(
        &EpochState::empty(),
        Arc::new(PayloadManager::DirectMempool),
        create_transaction_shuffler(TransactionShufflerType::NoShuffling),
        None,
        create_transaction_deduper(TransactionDeduperType::NoDedup),
    );
    executor
        .commit(&[], generate_li(1, 1), callback.clone())
        .await
        .unwrap();
    executor
        .commit(&[], generate_li(1, 10), callback)
        .await
        .unwrap();
    assert!(executor.sync_to(generate_li(1, 8)).await.is_ok());
    assert_eq!(*recorded_commit.time.lock(), LogicalTime::new(1, 10));
    assert!(executor.sync_to(generate_li(2, 8)).await.is_ok());
    assert_eq!(*recorded_commit.time.lock(), LogicalTime::new(2, 8));
}
