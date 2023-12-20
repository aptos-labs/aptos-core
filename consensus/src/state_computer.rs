// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block_preparer::BlockPreparer,
    block_storage::tracing::{observe_block, BlockStage},
    counters,
    error::StateSyncError,
    execution_pipeline::ExecutionPipeline,
    monitor,
    payload_manager::PayloadManager,
    state_replication::{StateComputer, StateComputerCommitCallBackType},
    transaction_deduper::TransactionDeduper,
    transaction_filter::TransactionFilter,
    transaction_shuffler::TransactionShuffler,
    txn_notifier::TxnNotifier,
};
use anyhow::Result;
use aptos_consensus_notifications::ConsensusNotificationSender;
use aptos_consensus_types::{block::Block, common::Round, executed_block::ExecutedBlock};
use aptos_crypto::HashValue;
use aptos_executor_types::{BlockExecutorTrait, ExecutorResult, StateComputeResult};
use aptos_infallible::Mutex;
use aptos_logger::prelude::*;
use aptos_types::{
    account_address::AccountAddress,
    block_executor::config::BlockExecutorConfigFromOnchain,
    contract_event::ContractEvent,
    epoch_state::EpochState,
    ledger_info::LedgerInfoWithSignatures,
    on_chain_config::OnChainExecutionConfig,
    transaction::{SignedTransaction, Transaction},
};
use fail::fail_point;
use futures::{future::BoxFuture, SinkExt, StreamExt};
use std::{boxed::Box, sync::Arc};
use tokio::sync::Mutex as AsyncMutex;

pub type StateComputeResultFut = BoxFuture<'static, ExecutorResult<PipelineExecutionResult>>;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct PipelineExecutionResult {
    pub input_txns: Vec<SignedTransaction>,
    pub result: StateComputeResult,
}

impl PipelineExecutionResult {
    pub fn new(input_txns: Vec<SignedTransaction>, result: StateComputeResult) -> Self {
        Self { input_txns, result }
    }

    pub fn new_dummy() -> Self {
        Self {
            input_txns: vec![],
            result: StateComputeResult::new_dummy(),
        }
    }
}

type NotificationType = (
    Box<dyn FnOnce() + Send + Sync>,
    Vec<Transaction>,
    Vec<ContractEvent>, // Subscribable events, e.g. NewEpochEvent, DKGStartEvent
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
    block_executor_onchain_config: Mutex<BlockExecutorConfigFromOnchain>,
    transaction_deduper: Mutex<Option<Arc<dyn TransactionDeduper>>>,
    transaction_filter: Arc<TransactionFilter>,
    execution_pipeline: ExecutionPipeline,
}

impl ExecutionProxy {
    pub fn new(
        executor: Arc<dyn BlockExecutorTrait>,
        txn_notifier: Arc<dyn TxnNotifier>,
        state_sync_notifier: Arc<dyn ConsensusNotificationSender>,
        handle: &tokio::runtime::Handle,
        txn_filter: TransactionFilter,
    ) -> Self {
        let (tx, mut rx) =
            aptos_channels::new::<NotificationType>(10, &counters::PENDING_STATE_SYNC_NOTIFICATION);
        let notifier = state_sync_notifier.clone();
        handle.spawn(async move {
            while let Some((callback, txns, subscribable_events)) = rx.next().await {
                if let Err(e) = monitor!(
                    "notify_state_sync",
                    notifier.notify_new_commit(txns, subscribable_events).await
                ) {
                    error!(error = ?e, "Failed to notify state synchronizer");
                }

                callback();
            }
        });
        let execution_pipeline = ExecutionPipeline::spawn(executor.clone(), handle);
        Self {
            executor,
            txn_notifier,
            state_sync_notifier,
            async_state_sync_notifier: tx,
            validators: Mutex::new(vec![]),
            write_mutex: AsyncMutex::new(LogicalTime::new(0, 0)),
            payload_manager: Mutex::new(None),
            transaction_shuffler: Mutex::new(None),
            block_executor_onchain_config: Mutex::new(
                OnChainExecutionConfig::default_if_missing().block_executor_onchain_config(),
            ),
            transaction_deduper: Mutex::new(None),
            transaction_filter: Arc::new(txn_filter),
            execution_pipeline,
        }
    }
}

#[async_trait::async_trait]
impl StateComputer for ExecutionProxy {
    async fn schedule_compute(
        &self,
        // The block to be executed.
        block: &Block,
        // The parent block id.
        parent_block_id: HashValue,
    ) -> StateComputeResultFut {
        let block_id = block.id();
        debug!(
            block = %block,
            parent_id = parent_block_id,
            "Executing block",
        );

        let txn_notifier = self.txn_notifier.clone();
        let transaction_generator = BlockPreparer::new(
            self.payload_manager.lock().as_ref().unwrap().clone(),
            self.transaction_filter.clone(),
            self.transaction_deduper.lock().as_ref().unwrap().clone(),
            self.transaction_shuffler.lock().as_ref().unwrap().clone(),
        );

        let block_executor_onchain_config = self.block_executor_onchain_config.lock().clone();

        let timestamp = block.timestamp_usecs();
        let metadata = block.new_block_metadata(&self.validators.lock());
        let fut = self
            .execution_pipeline
            .queue(
                block.clone(),
                metadata,
                parent_block_id,
                transaction_generator,
                block_executor_onchain_config,
            )
            .await;

        Box::pin(async move {
            debug!(
                block_id = block_id,
                "Got state compute result, post processing."
            );
            let pipeline_execution_result = fut.await?;
            let input_txns = pipeline_execution_result.input_txns.clone();
            let result = &pipeline_execution_result.result;

            observe_block(timestamp, BlockStage::EXECUTED);

            // notify mempool about failed transaction
            if let Err(e) = txn_notifier.notify_failed_txn(input_txns, result).await {
                error!(
                    error = ?e, "Failed to notify mempool of rejected txns",
                );
            }
            Ok(pipeline_execution_result)
        })
    }

    /// Send a successful commit. A future is fulfilled when the state is finalized.
    async fn commit(
        &self,
        blocks: &[Arc<ExecutedBlock>],
        finality_proof: LedgerInfoWithSignatures,
        callback: StateComputerCommitCallBackType,
    ) -> ExecutorResult<()> {
        let mut latest_logical_time = self.write_mutex.lock().await;
        let mut block_ids = Vec::new();
        let mut txns = Vec::new();
        let mut subscribable_txn_events = Vec::new();
        let mut payloads = Vec::new();
        let logical_time = LogicalTime::new(
            finality_proof.ledger_info().epoch(),
            finality_proof.ledger_info().round(),
        );
        let block_timestamp = finality_proof.commit_info().timestamp_usecs();
        let payload_manager = self.payload_manager.lock().as_ref().unwrap().clone();

        for block in blocks {
            block_ids.push(block.id());

            if let Some(payload) = block.block().payload() {
                payloads.push(payload.clone());
            }

            let input_txns = block.input_transactions().clone();
            txns.extend(block.transactions_to_commit(
                &self.validators.lock(),
                block.validator_txns().cloned().unwrap_or_default(),
                input_txns,
            ));
            subscribable_txn_events.extend(block.subscribable_events());
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
            .send((Box::new(wrapped_callback), txns, subscribable_txn_events))
            .await
            .expect("Failed to send async state sync notification");

        *latest_logical_time = logical_time;
        payload_manager.notify_commit(block_timestamp, payloads);
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
            payload_manager.notify_commit(block_timestamp, Vec::new());
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
        block_executor_onchain_config: BlockExecutorConfigFromOnchain,
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
        *self.block_executor_onchain_config.lock() = block_executor_onchain_config;
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
    use aptos_config::config::transaction_filter_type::Filter;
    use aptos_consensus_notifications::Error;
    use aptos_executor_types::state_checkpoint_output::StateCheckpointOutput;
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
            _block: ExecutableBlock,
            _parent_block_id: HashValue,
            _onchain_config: BlockExecutorConfigFromOnchain,
        ) -> ExecutorResult<StateComputeResult> {
            Ok(StateComputeResult::new_dummy())
        }

        fn execute_and_state_checkpoint(
            &self,
            _block: ExecutableBlock,
            _parent_block_id: HashValue,
            _onchain_config: BlockExecutorConfigFromOnchain,
        ) -> ExecutorResult<StateCheckpointOutput> {
            todo!()
        }

        fn ledger_update(
            &self,
            _block_id: HashValue,
            _parent_block_id: HashValue,
            _state_checkpoint_output: StateCheckpointOutput,
        ) -> ExecutorResult<StateComputeResult> {
            todo!()
        }

        fn commit_blocks_ext(
            &self,
            _block_ids: Vec<HashValue>,
            ledger_info_with_sigs: LedgerInfoWithSignatures,
            _save_state_snapshots: bool,
        ) -> ExecutorResult<()> {
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
            _subscribable_events: Vec<ContractEvent>,
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
        TransactionFilter::new(Filter::empty()),
    );

    executor.new_epoch(
        &EpochState::empty(),
        Arc::new(PayloadManager::DirectMempool),
        create_transaction_shuffler(TransactionShufflerType::NoShuffling),
        BlockExecutorConfigFromOnchain::new_no_block_limit(),
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
