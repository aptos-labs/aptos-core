// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block_preparer::BlockPreparer,
    block_storage::tracing::{observe_block, BlockStage},
    counters,
    error::StateSyncError,
    execution_pipeline::{ExecutionPipeline, PreCommitHook},
    monitor,
    payload_manager::TPayloadManager,
    pipeline::pipeline_phase::CountedRequest,
    state_replication::{StateComputer, StateComputerCommitCallBackType},
    transaction_deduper::TransactionDeduper,
    transaction_filter::TransactionFilter,
    transaction_shuffler::TransactionShuffler,
    txn_notifier::TxnNotifier,
};
use anyhow::Result;
use aptos_consensus_notifications::ConsensusNotificationSender;
use aptos_consensus_types::{
    block::Block, common::Round, pipeline_execution_result::PipelineExecutionResult,
    pipelined_block::PipelinedBlock,
};
use aptos_crypto::HashValue;
use aptos_executor_types::{BlockExecutorTrait, ExecutorResult, StateComputeResult};
use aptos_infallible::RwLock;
use aptos_logger::prelude::*;
use aptos_metrics_core::IntGauge;
use aptos_types::{
    account_address::AccountAddress, block_executor::config::BlockExecutorConfigFromOnchain,
    block_metadata_ext::BlockMetadataExt, epoch_state::EpochState,
    ledger_info::LedgerInfoWithSignatures, randomness::Randomness, transaction::SignedTransaction,
};
use fail::fail_point;
use futures::{future::BoxFuture, SinkExt, StreamExt};
use std::{boxed::Box, sync::Arc, time::Instant};
use tokio::sync::Mutex as AsyncMutex;

pub type StateComputeResultFut = BoxFuture<'static, ExecutorResult<PipelineExecutionResult>>;

type NotificationType = BoxFuture<'static, ()>;

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

#[derive(Clone)]
struct MutableState {
    validators: Arc<[AccountAddress]>,
    payload_manager: Arc<dyn TPayloadManager>,
    transaction_shuffler: Arc<dyn TransactionShuffler>,
    block_executor_onchain_config: BlockExecutorConfigFromOnchain,
    transaction_deduper: Arc<dyn TransactionDeduper>,
    is_randomness_enabled: bool,
}

/// Basic communication with the Execution module;
/// implements StateComputer traits.
pub struct ExecutionProxy {
    executor: Arc<dyn BlockExecutorTrait>,
    txn_notifier: Arc<dyn TxnNotifier>,
    state_sync_notifier: Arc<dyn ConsensusNotificationSender>,
    pre_commit_notifier: aptos_channels::Sender<NotificationType>,
    commit_notifier: aptos_channels::Sender<NotificationType>,
    write_mutex: AsyncMutex<LogicalTime>,
    transaction_filter: Arc<TransactionFilter>,
    execution_pipeline: ExecutionPipeline,
    state: RwLock<Option<MutableState>>,
}

impl ExecutionProxy {
    pub fn new(
        executor: Arc<dyn BlockExecutorTrait>,
        txn_notifier: Arc<dyn TxnNotifier>,
        state_sync_notifier: Arc<dyn ConsensusNotificationSender>,
        handle: &tokio::runtime::Handle,
        txn_filter: TransactionFilter,
        enable_pre_commit: bool,
    ) -> Self {
        let pre_commit_notifier = Self::spawn_future_runner(
            handle,
            "pre-commit",
            &counters::PENDING_STATE_SYNC_NOTIFICATION,
        );
        let commit_notifier =
            Self::spawn_future_runner(handle, "commit", &counters::PENDING_COMMIT_NOTIFICATION);

        let execution_pipeline =
            ExecutionPipeline::spawn(executor.clone(), handle, enable_pre_commit);
        Self {
            executor,
            txn_notifier,
            state_sync_notifier,
            pre_commit_notifier,
            commit_notifier,
            write_mutex: AsyncMutex::new(LogicalTime::new(0, 0)),
            transaction_filter: Arc::new(txn_filter),
            execution_pipeline,
            state: RwLock::new(None),
        }
    }

    fn spawn_future_runner(
        handle: &tokio::runtime::Handle,
        name: &'static str,
        pending_notifications_gauge: &IntGauge,
    ) -> aptos_channels::Sender<NotificationType> {
        let (tx, mut rx) = aptos_channels::new::<NotificationType>(10, pending_notifications_gauge);
        let _join_handle = handle.spawn(async move {
            while let Some(fut) = rx.next().await {
                fut.await
            }
            info!(name = name, "Future runner stopped.")
        });
        tx
    }

    fn pre_commit_hook(
        &self,
        block: &Block,
        metadata: BlockMetadataExt,
        payload_manager: Arc<dyn TPayloadManager>,
    ) -> PreCommitHook {
        let mut pre_commit_notifier = self.pre_commit_notifier.clone();
        let state_sync_notifier = self.state_sync_notifier.clone();
        let payload = block.payload().cloned();
        let timestamp = block.timestamp_usecs();
        let validator_txns = block.validator_txns().cloned().unwrap_or_default();
        let block_id = block.id();
        Box::new(
            move |user_txns: &[SignedTransaction], state_compute_result: &StateComputeResult| {
                let input_txns = Block::combine_to_input_transactions(
                    validator_txns,
                    user_txns.to_vec(),
                    metadata,
                );
                let txns = state_compute_result.transactions_to_commit(input_txns, block_id);
                let subscribable_events = state_compute_result.subscribable_events().to_vec();
                Box::pin(async move {
                    pre_commit_notifier
                        .send(Box::pin(async move {
                            if let Err(e) = monitor!(
                                "notify_state_sync",
                                state_sync_notifier
                                    .notify_new_commit(txns, subscribable_events)
                                    .await
                            ) {
                                error!(error = ?e, "Failed to notify state synchronizer");
                            }

                            let payload_vec = payload.into_iter().collect();
                            payload_manager.notify_commit(timestamp, payload_vec);
                        }))
                        .await
                        .expect("Failed to send pre-commit notification");
                })
            },
        )
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
        randomness: Option<Randomness>,
        lifetime_guard: CountedRequest<()>,
    ) -> StateComputeResultFut {
        let block_id = block.id();
        debug!(
            block = %block,
            parent_id = parent_block_id,
            "Executing block",
        );
        let MutableState {
            validators,
            payload_manager,
            transaction_shuffler,
            block_executor_onchain_config,
            transaction_deduper,
            is_randomness_enabled,
        } = self
            .state
            .read()
            .as_ref()
            .cloned()
            .expect("must be set within an epoch");

        let txn_notifier = self.txn_notifier.clone();
        let transaction_generator = BlockPreparer::new(
            payload_manager.clone(),
            self.transaction_filter.clone(),
            transaction_deduper.clone(),
            transaction_shuffler.clone(),
        );

        let block_executor_onchain_config = block_executor_onchain_config.clone();

        let timestamp = block.timestamp_usecs();
        let metadata = if is_randomness_enabled {
            block.new_metadata_with_randomness(&validators, randomness)
        } else {
            block.new_block_metadata(&validators).into()
        };

        let pipeline_entry_time = Instant::now();
        let fut = self
            .execution_pipeline
            .queue(
                block.clone(),
                metadata.clone(),
                parent_block_id,
                transaction_generator,
                block_executor_onchain_config,
                self.pre_commit_hook(block, metadata, payload_manager),
                lifetime_guard,
            )
            .await;
        observe_block(timestamp, BlockStage::EXECUTION_PIPELINE_INSERTED);
        counters::PIPELINE_ENTRY_TO_INSERTED_TIME.observe_duration(pipeline_entry_time.elapsed());
        let pipeline_inserted_timestamp = Instant::now();

        Box::pin(async move {
            let pipeline_execution_result = fut.await?;
            debug!(
                block_id = block_id,
                "Got state compute result, post processing."
            );
            let user_txns = &pipeline_execution_result.input_txns;
            let result = &pipeline_execution_result.result;

            observe_block(timestamp, BlockStage::EXECUTED);
            counters::PIPELINE_INSERTION_TO_EXECUTED_TIME
                .observe_duration(pipeline_inserted_timestamp.elapsed());

            let compute_status = result.compute_status_for_input_txns();
            // the length of compute_status is user_txns.len() + num_vtxns + 1 due to having blockmetadata
            if user_txns.len() >= compute_status.len() {
                // reconfiguration suffix blocks don't have any transactions
                // otherwise, this is an error
                if !compute_status.is_empty() {
                    error!(
                        "Expected compute_status length and actual compute_status length mismatch! user_txns len: {}, compute_status len: {}, has_reconfiguration: {}",
                        user_txns.len(),
                        compute_status.len(),
                        result.has_reconfiguration(),
                    );
                }
            } else {
                let user_txn_status = &compute_status[compute_status.len() - user_txns.len()..];

                // notify mempool about failed transaction
                if let Err(e) = txn_notifier
                    .notify_failed_txn(user_txns, user_txn_status)
                    .await
                {
                    error!(
                        error = ?e, "Failed to notify mempool of rejected txns",
                    );
                }
            }

            Ok(pipeline_execution_result)
        })
    }

    /// Send a successful commit. A future is fulfilled when the state is finalized.
    async fn commit(
        &self,
        blocks: &[Arc<PipelinedBlock>],
        finality_proof: LedgerInfoWithSignatures,
        callback: StateComputerCommitCallBackType,
    ) -> ExecutorResult<()> {
        let mut latest_logical_time = self.write_mutex.lock().await;
        let logical_time = LogicalTime::new(
            finality_proof.ledger_info().epoch(),
            finality_proof.ledger_info().round(),
        );

        // wait until all blocks are committed
        for block in blocks {
            block.take_pre_commit_fut().await?
        }

        let executor = self.executor.clone();
        let proof = finality_proof.clone();
        monitor!(
            "commit_block",
            tokio::task::spawn_blocking(move || {
                executor
                    .commit_ledger(proof)
                    .expect("Failed to commit blocks");
            })
            .await
        )
        .expect("spawn_blocking failed");

        let blocks = blocks.to_vec();
        let callback_fut = Box::pin(async move {
            callback(&blocks, finality_proof);
        });

        self.commit_notifier
            .clone()
            .send(callback_fut)
            .await
            .expect("Failed to send commit notification");

        *latest_logical_time = logical_time;
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
        if let Some(inner) = self.state.read().as_ref() {
            inner
                .payload_manager
                .notify_commit(block_timestamp, Vec::new());
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
        payload_manager: Arc<dyn TPayloadManager>,
        transaction_shuffler: Arc<dyn TransactionShuffler>,
        block_executor_onchain_config: BlockExecutorConfigFromOnchain,
        transaction_deduper: Arc<dyn TransactionDeduper>,
        randomness_enabled: bool,
    ) {
        *self.state.write() = Some(MutableState {
            validators: epoch_state
                .verifier
                .get_ordered_account_addresses_iter()
                .collect::<Vec<_>>()
                .into(),
            payload_manager,
            transaction_shuffler,
            block_executor_onchain_config,
            transaction_deduper,
            is_randomness_enabled: randomness_enabled,
        });
    }

    // Clears the epoch-specific state. Only a sync_to call is expected before calling new_epoch
    // on the next epoch.
    fn end_epoch(&self) {
        self.state.write().take();
    }
}

#[tokio::test]
async fn test_commit_sync_race() {
    use crate::{
        error::MempoolError, payload_manager::DirectMempoolPayloadManager,
        transaction_deduper::create_transaction_deduper,
        transaction_shuffler::create_transaction_shuffler,
    };
    use aptos_config::config::transaction_filter_type::Filter;
    use aptos_consensus_notifications::Error;
    use aptos_executor_types::{
        state_checkpoint_output::StateCheckpointOutput, StateComputeResult,
    };
    use aptos_infallible::Mutex;
    use aptos_types::{
        aggregate_signature::AggregateSignature,
        block_executor::partitioner::ExecutableBlock,
        block_info::BlockInfo,
        contract_event::ContractEvent,
        ledger_info::LedgerInfo,
        on_chain_config::{TransactionDeduperType, TransactionShufflerType},
        transaction::{SignedTransaction, Transaction, TransactionStatus},
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

        fn pre_commit_block(
            &self,
            _block_id: HashValue,
            _parent_block_id: HashValue,
        ) -> ExecutorResult<()> {
            todo!()
        }

        fn commit_ledger(
            &self,
            ledger_info_with_sigs: LedgerInfoWithSignatures,
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
            _txns: &[SignedTransaction],
            _compute_results: &[TransactionStatus],
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

        async fn sync_for_duration(
            &self,
            _duration: std::time::Duration,
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

    let callback = Box::new(move |_a: &[Arc<PipelinedBlock>], _b: LedgerInfoWithSignatures| {});
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
        true,
    );

    executor.new_epoch(
        &EpochState::empty(),
        Arc::new(DirectMempoolPayloadManager {}),
        create_transaction_shuffler(TransactionShufflerType::NoShuffling),
        BlockExecutorConfigFromOnchain::new_no_block_limit(),
        create_transaction_deduper(TransactionDeduperType::NoDedup),
        false,
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
