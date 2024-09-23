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
use aptos_executor_types::{BlockExecutorTrait, ExecutorResult};
use aptos_infallible::RwLock;
use aptos_logger::prelude::*;
use aptos_types::{
    account_address::AccountAddress, block_executor::config::BlockExecutorConfigFromOnchain,
    contract_event::ContractEvent, epoch_state::EpochState, ledger_info::LedgerInfoWithSignatures,
    randomness::Randomness, transaction::Transaction,
};
use fail::fail_point;
use futures::{future::BoxFuture, SinkExt, StreamExt};
use std::{boxed::Box, sync::Arc, time::Instant};
use tokio::sync::Mutex as AsyncMutex;

pub type StateComputeResultFut = BoxFuture<'static, ExecutorResult<PipelineExecutionResult>>;

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
    async_state_sync_notifier: aptos_channels::Sender<NotificationType>,
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
        let execution_pipeline =
            ExecutionPipeline::spawn(executor.clone(), handle, enable_pre_commit);
        Self {
            executor,
            txn_notifier,
            state_sync_notifier,
            async_state_sync_notifier: tx,
            write_mutex: AsyncMutex::new(LogicalTime::new(0, 0)),
            transaction_filter: Arc::new(txn_filter),
            execution_pipeline,
            state: RwLock::new(None),
        }
    }

    fn transactions_to_commit(
        &self,
        executed_block: &PipelinedBlock,
        validators: &[AccountAddress],
        randomness_enabled: bool,
    ) -> Vec<Transaction> {
        // reconfiguration suffix don't execute
        if executed_block.is_reconfiguration_suffix() {
            return vec![];
        }

        let user_txns = executed_block.input_transactions().clone();
        let validator_txns = executed_block.validator_txns().cloned().unwrap_or_default();
        let metadata = if randomness_enabled {
            executed_block
                .block()
                .new_metadata_with_randomness(validators, executed_block.randomness().cloned())
        } else {
            executed_block.block().new_block_metadata(validators).into()
        };

        let input_txns = Block::combine_to_input_transactions(validator_txns, user_txns, metadata);

        // Adds StateCheckpoint/BlockEpilogue transaction if needed.
        executed_block
            .compute_result()
            .transactions_to_commit(input_txns, executed_block.id())
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
                metadata,
                parent_block_id,
                transaction_generator,
                block_executor_onchain_config,
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
        let mut txns = Vec::new();
        let mut subscribable_txn_events = Vec::new();
        let mut payloads = Vec::new();
        let logical_time = LogicalTime::new(
            finality_proof.ledger_info().epoch(),
            finality_proof.ledger_info().round(),
        );
        let block_timestamp = finality_proof.commit_info().timestamp_usecs();

        let MutableState {
            payload_manager,
            validators,
            is_randomness_enabled,
            ..
        } = self
            .state
            .read()
            .as_ref()
            .cloned()
            .expect("must be set within an epoch");
        let mut pre_commit_futs = Vec::with_capacity(blocks.len());
        for block in blocks {
            if let Some(payload) = block.block().payload() {
                payloads.push(payload.clone());
            }

            txns.extend(self.transactions_to_commit(block, &validators, is_randomness_enabled));
            subscribable_txn_events.extend(block.subscribable_events());
            pre_commit_futs.push(block.take_pre_commit_fut());
        }

        // wait until all blocks are committed
        for pre_commit_fut in pre_commit_futs {
            pre_commit_fut.await?
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
        ledger_info::LedgerInfo,
        on_chain_config::{TransactionDeduperType, TransactionShufflerType},
        transaction::{SignedTransaction, TransactionStatus},
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
