// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    consensus_observer::publisher::consensus_publisher::ConsensusPublisher,
    counters,
    error::StateSyncError,
    network::{
        IncomingCommitRequest, IncomingRandGenRequest, IncomingSecretShareRequest, NetworkSender,
    },
    network_interface::{ConsensusMsg, ConsensusNetworkClient},
    payload_manager::TPayloadManager,
    pipeline::{
        commit_pipeline::{CommitPipeline, OrderedBlocks, ResetAck, ResetRequest, ResetSignal},
        commit_signer::CommitSignerProvider,
        errors::Error,
        pipeline_builder::PipelineBuilder,
    },
    rand::{
        rand_gen::{
            rand_manager::RandManager,
            storage::interface::RandStorage,
            types::{AugmentedData, RandConfig, Share},
        },
        secret_sharing::{
            secret_share_manager::SecretShareManager, storage::SecretShareStorage,
            verifier::SecretShareVerifier,
        },
    },
    state_computer::ExecutionProxy,
    state_replication::StateComputer,
    transaction_deduper::create_transaction_deduper,
    transaction_shuffler::create_transaction_shuffler,
};
use anyhow::{anyhow, Result};
use aptos_bounded_executor::BoundedExecutor;
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_config::config::{ConsensusConfig, ConsensusObserverConfig};
use aptos_consensus_types::{
    common::{Author, Round},
    pipelined_block::{OrderedBlocksForObserver, PipelinedBlock},
    wrapped_ledger_info::WrappedLedgerInfo,
};
use aptos_crypto::bls12381::PrivateKey;
use aptos_executor_types::ExecutorResult;
use aptos_infallible::RwLock;
use aptos_logger::prelude::*;
use aptos_network::{application::interface::NetworkClient, protocols::network::Event};
use aptos_types::{
    epoch_state::EpochState,
    ledger_info::LedgerInfoWithSignatures,
    on_chain_config::{
        Features, OnChainChunkyDKGConfig, OnChainConsensusConfig, OnChainExecutionConfig,
        OnChainRandomnessConfig,
    },
    validator_signer::ValidatorSigner,
};
use fail::fail_point;
use futures::{
    channel::{mpsc::UnboundedSender, oneshot},
    SinkExt,
};
use futures_channel::mpsc::unbounded;
use move_core_types::account_address::AccountAddress;
use std::{sync::Arc, time::Duration};

#[allow(clippy::too_many_arguments)]
#[async_trait::async_trait]
pub trait TExecutionClient: Send + Sync {
    /// Initialize the execution phase for a new epoch.
    async fn start_epoch(
        &self,
        maybe_consensus_key: Arc<PrivateKey>,
        epoch_state: Arc<EpochState>,
        commit_signer_provider: Arc<dyn CommitSignerProvider>,
        payload_manager: Arc<dyn TPayloadManager>,
        onchain_consensus_config: &OnChainConsensusConfig,
        onchain_execution_config: &OnChainExecutionConfig,
        onchain_randomness_config: &OnChainRandomnessConfig,
        onchain_chunky_dkg_config: &OnChainChunkyDKGConfig,
        features: &Features,
        rand_config: Option<RandConfig>,
        secret_share_verifier: Option<Arc<SecretShareVerifier>>,
        rand_msg_rx: aptos_channel::Receiver<AccountAddress, IncomingRandGenRequest>,
        secret_sharing_msg_rx: aptos_channel::Receiver<AccountAddress, IncomingSecretShareRequest>,
        highest_committed_round: Round,
    );

    /// This is needed for some DAG tests. Clean this up as a TODO.
    fn get_execution_channel(&self) -> Option<UnboundedSender<OrderedBlocks>>;

    /// Send ordered blocks to the real execution phase through the channel.
    async fn finalize_order(
        &self,
        blocks: Vec<Arc<PipelinedBlock>>,
        ordered_proof: WrappedLedgerInfo,
    ) -> ExecutorResult<()>;

    fn send_commit_msg(
        &self,
        peer_id: AccountAddress,
        commit_msg: IncomingCommitRequest,
    ) -> Result<()>;

    /// Synchronizes for the specified duration and returns the latest synced
    /// ledger info. Note: it is possible that state sync may run longer than
    /// the specified duration (e.g., if the node is very far behind).
    async fn sync_for_duration(
        &self,
        duration: Duration,
    ) -> Result<LedgerInfoWithSignatures, StateSyncError>;

    /// Synchronize to a commit that is not present locally.
    async fn sync_to_target(&self, target: LedgerInfoWithSignatures) -> Result<(), StateSyncError>;

    /// Resets the internal state of the rand and commit pipelines.
    async fn reset(&self, target: &LedgerInfoWithSignatures) -> Result<()>;

    /// Shutdown the current processor at the end of the epoch.
    async fn end_epoch(&self);

    /// Returns a pipeline builder for the current epoch.
    fn pipeline_builder(&self, signer: Arc<ValidatorSigner>) -> PipelineBuilder;
}

struct PipelineHandle {
    pub execute_tx: Option<UnboundedSender<OrderedBlocks>>,
    pub rand_input_tx: Option<UnboundedSender<OrderedBlocks>>,
    pub secret_input_tx: Option<UnboundedSender<OrderedBlocks>>,
    pub commit_tx:
        Option<aptos_channel::Sender<AccountAddress, (AccountAddress, IncomingCommitRequest)>>,
    pub reset_tx_to_commit_pipeline: Option<UnboundedSender<ResetRequest>>,
    pub reset_tx_to_rand_manager: Option<UnboundedSender<ResetRequest>>,
    pub reset_tx_to_secret_share_manager: Option<UnboundedSender<ResetRequest>>,
}

impl PipelineHandle {
    pub fn new() -> Self {
        Self {
            execute_tx: None,
            rand_input_tx: None,
            secret_input_tx: None,
            commit_tx: None,
            reset_tx_to_commit_pipeline: None,
            reset_tx_to_rand_manager: None,
            reset_tx_to_secret_share_manager: None,
        }
    }

    pub fn init(
        &mut self,
        execute_tx: UnboundedSender<OrderedBlocks>,
        commit_tx: aptos_channel::Sender<AccountAddress, (AccountAddress, IncomingCommitRequest)>,
        reset_tx_to_commit_pipeline: UnboundedSender<ResetRequest>,
        reset_tx_to_rand_manager: Option<UnboundedSender<ResetRequest>>,
        maybe_reset_tx_to_secret_share_manager: Option<UnboundedSender<ResetRequest>>,
    ) {
        self.execute_tx = Some(execute_tx);
        self.commit_tx = Some(commit_tx);
        self.reset_tx_to_commit_pipeline = Some(reset_tx_to_commit_pipeline);
        self.reset_tx_to_rand_manager = reset_tx_to_rand_manager;
        self.reset_tx_to_secret_share_manager = maybe_reset_tx_to_secret_share_manager;
    }

    pub fn reset(
        &mut self,
    ) -> (
        Option<UnboundedSender<ResetRequest>>,
        Option<UnboundedSender<ResetRequest>>,
        Option<UnboundedSender<ResetRequest>>,
    ) {
        let reset_tx_to_rand_manager = self.reset_tx_to_rand_manager.take();
        let reset_tx_to_commit_pipeline = self.reset_tx_to_commit_pipeline.take();
        let reset_tx_to_secret_share_manager = self.reset_tx_to_secret_share_manager.take();
        self.execute_tx = None;
        self.rand_input_tx = None;
        self.secret_input_tx = None;
        self.commit_tx = None;
        (
            reset_tx_to_rand_manager,
            reset_tx_to_commit_pipeline,
            reset_tx_to_secret_share_manager,
        )
    }
}

pub struct ExecutionProxyClient {
    consensus_config: ConsensusConfig,
    execution_proxy: Arc<ExecutionProxy>,
    author: Author,
    self_sender: aptos_channels::UnboundedSender<Event<ConsensusMsg>>,
    network_sender: ConsensusNetworkClient<NetworkClient<ConsensusMsg>>,
    bounded_executor: BoundedExecutor,
    // channels to commit pipeline
    handle: Arc<RwLock<PipelineHandle>>,
    rand_storage: Arc<dyn RandStorage<AugmentedData>>,
    secret_share_storage: Arc<dyn SecretShareStorage>,
    consensus_observer_config: ConsensusObserverConfig,
    consensus_publisher: Option<Arc<ConsensusPublisher>>,
}

impl ExecutionProxyClient {
    pub fn new(
        consensus_config: ConsensusConfig,
        execution_proxy: Arc<ExecutionProxy>,
        author: Author,
        self_sender: aptos_channels::UnboundedSender<Event<ConsensusMsg>>,
        network_sender: ConsensusNetworkClient<NetworkClient<ConsensusMsg>>,
        bounded_executor: BoundedExecutor,
        rand_storage: Arc<dyn RandStorage<AugmentedData>>,
        secret_share_storage: Arc<dyn SecretShareStorage>,
        consensus_observer_config: ConsensusObserverConfig,
        consensus_publisher: Option<Arc<ConsensusPublisher>>,
    ) -> Self {
        Self {
            consensus_config,
            execution_proxy,
            author,
            self_sender,
            network_sender,
            bounded_executor,
            handle: Arc::new(RwLock::new(PipelineHandle::new())),
            rand_storage,
            secret_share_storage,
            consensus_observer_config,
            consensus_publisher,
        }
    }

    fn make_rand_manager(
        &self,
        epoch_state: &Arc<EpochState>,
        rand_msg_rx: aptos_channel::Receiver<AccountAddress, IncomingRandGenRequest>,
        highest_committed_round: u64,
        network_sender: &Arc<NetworkSender>,
        rand_config: RandConfig,
        consensus_sk: Arc<PrivateKey>,
    ) -> (
        UnboundedSender<OrderedBlocks>,
        UnboundedSender<ResetRequest>,
    ) {
        let (ordered_block_tx, ordered_block_rx) = unbounded::<OrderedBlocks>();

        let (reset_tx_to_rand_manager, reset_rand_manager_rx) = unbounded::<ResetRequest>();

        let signer = Arc::new(ValidatorSigner::new(self.author, consensus_sk));

        let rand_manager = RandManager::<Share, AugmentedData>::new(
            self.author,
            epoch_state.clone(),
            signer,
            rand_config,
            network_sender.clone(),
            self.rand_storage.clone(),
            self.bounded_executor.clone(),
            &self.consensus_config.rand_rb_config,
        );

        tokio::spawn(rand_manager.start(
            ordered_block_rx,
            rand_msg_rx,
            reset_rand_manager_rx,
            self.bounded_executor.clone(),
            highest_committed_round,
        ));

        (ordered_block_tx, reset_tx_to_rand_manager)
    }

    fn make_secret_sharing_manager(
        &self,
        epoch_state: &Arc<EpochState>,
        verifier: Arc<SecretShareVerifier>,
        secret_sharing_msg_rx: aptos_channel::Receiver<AccountAddress, IncomingSecretShareRequest>,
        highest_committed_round: u64,
        network_sender: &Arc<NetworkSender>,
    ) -> (
        UnboundedSender<OrderedBlocks>,
        UnboundedSender<ResetRequest>,
    ) {
        let (ordered_block_tx, ordered_block_rx) = unbounded::<OrderedBlocks>();

        let (reset_tx_to_secret_share_manager, reset_secret_share_manager_rx) =
            unbounded::<ResetRequest>();

        let state_sync_round_gap = self.consensus_config.max_commit_gap.max(
            self.consensus_config
                .vote_back_pressure_limit
                .saturating_mul(2),
        );
        let retention_rounds = state_sync_round_gap.saturating_mul(2);

        let secret_share_manager = SecretShareManager::new(
            self.author,
            epoch_state.clone(),
            verifier,
            network_sender.clone(),
            self.secret_share_storage.clone(),
            highest_committed_round,
            retention_rounds,
            self.bounded_executor.clone(),
            &self.consensus_config.secret_share_rb_config,
            self.consensus_config.secret_share_request_delay_ms,
        );

        tokio::spawn(secret_share_manager.start(
            ordered_block_rx,
            secret_sharing_msg_rx,
            reset_secret_share_manager_rx,
            self.bounded_executor.clone(),
            highest_committed_round,
        ));

        (ordered_block_tx, reset_tx_to_secret_share_manager)
    }

    #[allow(clippy::too_many_arguments)]
    fn spawn_decoupled_execution(
        &self,
        consensus_sk: Arc<PrivateKey>,
        _commit_signer_provider: Arc<dyn CommitSignerProvider>,
        epoch_state: Arc<EpochState>,
        rand_config: Option<RandConfig>,
        secret_share_verifier: Option<Arc<SecretShareVerifier>>,
        onchain_consensus_config: &OnChainConsensusConfig,
        rand_msg_rx: aptos_channel::Receiver<AccountAddress, IncomingRandGenRequest>,
        secret_sharing_msg_rx: aptos_channel::Receiver<AccountAddress, IncomingSecretShareRequest>,
        highest_committed_round: Round,
        consensus_observer_config: ConsensusObserverConfig,
        consensus_publisher: Option<Arc<ConsensusPublisher>>,
        network_sender: Arc<NetworkSender>,
    ) {
        let (reset_commit_pipeline_tx, reset_commit_pipeline_rx) = unbounded::<ResetRequest>();

        let (commit_msg_tx, commit_msg_rx) =
            aptos_channel::new::<AccountAddress, (AccountAddress, IncomingCommitRequest)>(
                QueueStyle::FIFO,
                100,
                Some(&counters::BUFFER_MANAGER_MSGS),
            );

        let (execution_ready_block_tx, execution_ready_block_rx) = unbounded();
        let (rand_input_tx, rand_reset_tx) = if let Some(config) = rand_config {
            let (input, reset) = self.make_rand_manager(
                &epoch_state,
                rand_msg_rx,
                highest_committed_round,
                &network_sender,
                config,
                consensus_sk,
            );
            (Some(input), Some(reset))
        } else {
            (None, None)
        };
        let (secret_input_tx, secret_reset_tx) = if let Some(verifier) = secret_share_verifier {
            let (input, reset) = self.make_secret_sharing_manager(
                &epoch_state,
                verifier,
                secret_sharing_msg_rx,
                highest_committed_round,
                &network_sender,
            );
            (Some(input), Some(reset))
        } else {
            (None, None)
        };
        {
            let mut handle = self.handle.write();
            handle.init(
                execution_ready_block_tx,
                commit_msg_tx,
                reset_commit_pipeline_tx,
                rand_reset_tx,
                secret_reset_tx,
            );
            handle.rand_input_tx = rand_input_tx;
            handle.secret_input_tx = secret_input_tx;
        }

        let commit_pipeline = CommitPipeline::new(
            self.author,
            network_sender,
            commit_msg_rx,
            execution_ready_block_rx,
            reset_commit_pipeline_rx,
            epoch_state,
            self.bounded_executor.clone(),
            onchain_consensus_config.order_vote_enabled(),
            highest_committed_round,
            consensus_observer_config,
            consensus_publisher,
            self.consensus_config
                .max_pending_rounds_in_commit_vote_cache,
        );
        tokio::spawn(commit_pipeline.start());
    }
}

#[async_trait::async_trait]
impl TExecutionClient for ExecutionProxyClient {
    async fn start_epoch(
        &self,
        maybe_consensus_key: Arc<PrivateKey>,
        epoch_state: Arc<EpochState>,
        commit_signer_provider: Arc<dyn CommitSignerProvider>,
        payload_manager: Arc<dyn TPayloadManager>,
        onchain_consensus_config: &OnChainConsensusConfig,
        onchain_execution_config: &OnChainExecutionConfig,
        onchain_randomness_config: &OnChainRandomnessConfig,
        onchain_chunky_dkg_config: &OnChainChunkyDKGConfig,
        features: &Features,
        rand_config: Option<RandConfig>,
        secret_share_verifier: Option<Arc<SecretShareVerifier>>,
        rand_msg_rx: aptos_channel::Receiver<AccountAddress, IncomingRandGenRequest>,
        secret_sharing_msg_rx: aptos_channel::Receiver<AccountAddress, IncomingSecretShareRequest>,
        highest_committed_round: Round,
    ) {
        let network_sender = Arc::new(NetworkSender::new(
            self.author,
            self.network_sender.clone(),
            self.self_sender.clone(),
            epoch_state.verifier.clone(),
        ));
        let maybe_rand_msg_tx = self.spawn_decoupled_execution(
            maybe_consensus_key,
            commit_signer_provider,
            epoch_state.clone(),
            rand_config,
            secret_share_verifier.clone(),
            onchain_consensus_config,
            rand_msg_rx,
            secret_sharing_msg_rx,
            highest_committed_round,
            self.consensus_observer_config,
            self.consensus_publisher.clone(),
            network_sender.clone(),
        );

        let transaction_shuffler =
            create_transaction_shuffler(onchain_execution_config.transaction_shuffler_type());
        let block_executor_onchain_config: aptos_types::block_executor::config::BlockExecutorConfigFromOnchain =
            onchain_execution_config.block_executor_onchain_config(features);
        let transaction_deduper =
            create_transaction_deduper(onchain_execution_config.transaction_deduper_type());
        let randomness_enabled = onchain_consensus_config.is_vtxn_enabled()
            && onchain_randomness_config.randomness_enabled();
        let decryption_enabled = onchain_consensus_config.is_vtxn_enabled()
            && onchain_chunky_dkg_config.chunky_dkg_enabled();

        // TODO(decryption+windowing): the dense decryption-round chain is seeded
        // in `build_root` from the commit-root state, but with execution
        // windowing the synthetic root is the older `window_root`, and the
        // committed window blocks get re-pipelined on recovery. Whether they
        // re-increment the chain (overshooting the round) or pass through as
        // no-key (self-consistent) depends on what feeds `secret_shared_key_rx`
        // for re-inserted committed blocks — which is untested. Until that path
        // is verified, refuse to run the two together rather than risk a
        // mid-epoch round divergence across validators.
        assert!(
            !(decryption_enabled && onchain_consensus_config.window_size().is_some()),
            "execution windowing (window_size) is not yet supported with decryption enabled; \
             see TODO in execution_client::start_epoch"
        );

        let aux_version = onchain_execution_config.persisted_auxiliary_info_version();

        self.execution_proxy.new_epoch(
            &epoch_state,
            payload_manager,
            transaction_shuffler,
            block_executor_onchain_config,
            transaction_deduper,
            randomness_enabled,
            decryption_enabled,
            onchain_consensus_config.clone(),
            aux_version,
            network_sender,
            secret_share_verifier.as_ref().map(|v| v.config().clone()),
            self.consensus_publisher.clone(),
            self.consensus_observer_config.enable_v2_message_sending,
        );

        maybe_rand_msg_tx
    }

    fn get_execution_channel(&self) -> Option<UnboundedSender<OrderedBlocks>> {
        self.handle.read().execute_tx.clone()
    }

    async fn finalize_order(
        &self,
        blocks: Vec<Arc<PipelinedBlock>>,
        ordered_proof: WrappedLedgerInfo,
    ) -> ExecutorResult<()> {
        assert!(!blocks.is_empty());
        let (execute_tx, rand_input_tx, secret_input_tx) = {
            let handle = self.handle.read();
            (
                handle.execute_tx.clone(),
                handle.rand_input_tx.clone(),
                handle.secret_input_tx.clone(),
            )
        };
        let Some(mut execute_tx) = execute_tx else {
            return Ok(());
        };

        let ordered_blocks_for_observer = OrderedBlocksForObserver::new(&blocks);
        for block in &blocks {
            block.set_insertion_time();
            if let Some(tx) = block.pipeline_tx().lock().as_mut() {
                tx.order_proof_tx
                    .take()
                    .map(|tx| tx.send(ordered_proof.clone()));
                tx.ordered_blocks_for_observer_tx
                    .take()
                    .map(|tx| tx.send(ordered_blocks_for_observer.clone()));
                if rand_input_tx.is_none() {
                    if let Some(sender) = tx.rand_tx.take() {
                        let _ = sender.send(None);
                    }
                }
                if secret_input_tx.is_none() {
                    if let Some(sender) = tx.secret_shared_key_tx.take() {
                        let _ = sender.send(None);
                    }
                }
            }
        }

        let ordered = OrderedBlocks {
            ordered_blocks: blocks,
            ordered_proof: ordered_proof.ledger_info().clone(),
        };
        for mut sender in [rand_input_tx, secret_input_tx].into_iter().flatten() {
            if sender.send(ordered.clone()).await.is_err() {
                debug!("Share service stopped during ordering");
            }
        }
        if execute_tx.send(ordered).await.is_err() {
            debug!("Commit service stopped during ordering");
        }
        Ok(())
    }

    fn send_commit_msg(
        &self,
        peer_id: AccountAddress,
        commit_msg: IncomingCommitRequest,
    ) -> Result<()> {
        if let Some(tx) = &self.handle.read().commit_tx {
            tx.push(peer_id, (peer_id, commit_msg))
        } else {
            counters::EPOCH_MANAGER_ISSUES_DETAILS
                .with_label_values(&["commit_pipeline_not_started"])
                .inc();
            warn!("Buffer manager not started");
            Ok(())
        }
    }

    async fn sync_for_duration(
        &self,
        duration: Duration,
    ) -> Result<LedgerInfoWithSignatures, StateSyncError> {
        fail_point!("consensus::sync_for_duration", |_| {
            Err(anyhow::anyhow!("Injected error in sync_for_duration").into())
        });

        // Sync for the specified duration
        let result = self.execution_proxy.sync_for_duration(duration).await;

        // Reset the rand and commit pipelines to the new synced round
        if let Ok(latest_synced_ledger_info) = &result {
            self.reset(latest_synced_ledger_info).await?;
        }

        result
    }

    async fn sync_to_target(&self, target: LedgerInfoWithSignatures) -> Result<(), StateSyncError> {
        fail_point!("consensus::sync_to_target", |_| {
            Err(anyhow::anyhow!("Injected error in sync_to_target").into())
        });

        // Reset the rand and commit pipelines to the target round
        self.reset(&target).await?;

        // TODO: handle the state sync error (e.g., re-push the ordered
        // blocks to the commit pipeline when it's reset but sync fails).
        self.execution_proxy.sync_to_target(target).await
    }

    async fn reset(&self, target: &LedgerInfoWithSignatures) -> Result<()> {
        let (
            reset_tx_to_rand_manager,
            reset_tx_to_secret_share_manager,
            reset_tx_to_commit_pipeline,
        ) = {
            let handle = self.handle.read();
            (
                handle.reset_tx_to_rand_manager.clone(),
                handle.reset_tx_to_secret_share_manager.clone(),
                handle.reset_tx_to_commit_pipeline.clone(),
            )
        };

        if let Some(mut reset_tx) = reset_tx_to_rand_manager {
            let (ack_tx, ack_rx) = oneshot::channel::<ResetAck>();
            reset_tx
                .send(ResetRequest {
                    tx: ack_tx,
                    signal: ResetSignal::TargetRound(target.commit_info().round()),
                })
                .await
                .map_err(|_| Error::RandResetDropped)?;
            ack_rx.await.map_err(|_| Error::RandResetDropped)?;
        }

        if let Some(mut reset_tx) = reset_tx_to_secret_share_manager {
            let (ack_tx, ack_rx) = oneshot::channel::<ResetAck>();
            reset_tx
                .send(ResetRequest {
                    tx: ack_tx,
                    signal: ResetSignal::TargetRound(target.commit_info().round()),
                })
                .await
                .map_err(|_| Error::RandResetDropped)?;
            ack_rx.await.map_err(|_| Error::RandResetDropped)?;
        }

        if let Some(mut reset_tx) = reset_tx_to_commit_pipeline {
            // reset execution phase and commit phase
            let (tx, rx) = oneshot::channel::<ResetAck>();
            reset_tx
                .send(ResetRequest {
                    tx,
                    signal: ResetSignal::TargetRound(target.commit_info().round()),
                })
                .await
                .map_err(|_| Error::ResetDropped)?;
            rx.await.map_err(|_| Error::ResetDropped)?;
        }

        Ok(())
    }

    async fn end_epoch(&self) {
        let (
            reset_tx_to_rand_manager,
            reset_tx_to_commit_pipeline,
            reset_tx_to_secret_share_manager,
        ) = {
            let mut handle = self.handle.write();
            handle.reset()
        };

        if let Some(mut tx) = reset_tx_to_rand_manager {
            let (ack_tx, ack_rx) = oneshot::channel();
            tx.send(ResetRequest {
                tx: ack_tx,
                signal: ResetSignal::Stop,
            })
            .await
            .expect("[EpochManager] Fail to drop rand manager");
            ack_rx
                .await
                .expect("[EpochManager] Fail to drop rand manager");
        }

        if let Some(mut tx) = reset_tx_to_secret_share_manager {
            let (ack_tx, ack_rx) = oneshot::channel();
            tx.send(ResetRequest {
                tx: ack_tx,
                signal: ResetSignal::Stop,
            })
            .await
            .expect("[EpochManager] Fail to drop secret share manager");
            ack_rx
                .await
                .expect("[EpochManager] Fail to drop secret share manager");
        }

        if let Some(mut tx) = reset_tx_to_commit_pipeline {
            let (ack_tx, ack_rx) = oneshot::channel();
            tx.send(ResetRequest {
                tx: ack_tx,
                signal: ResetSignal::Stop,
            })
            .await
            .expect("[EpochManager] Fail to drop commit pipeline");
            ack_rx
                .await
                .expect("[EpochManager] Fail to drop commit pipeline");
        }
        self.execution_proxy.end_epoch();
    }

    fn pipeline_builder(&self, signer: Arc<ValidatorSigner>) -> PipelineBuilder {
        self.execution_proxy.pipeline_builder(signer)
    }
}

pub struct DummyExecutionClient;

#[async_trait::async_trait]
impl TExecutionClient for DummyExecutionClient {
    async fn start_epoch(
        &self,
        _maybe_consensus_key: Arc<PrivateKey>,
        _epoch_state: Arc<EpochState>,
        _commit_signer_provider: Arc<dyn CommitSignerProvider>,
        _payload_manager: Arc<dyn TPayloadManager>,
        _onchain_consensus_config: &OnChainConsensusConfig,
        _onchain_execution_config: &OnChainExecutionConfig,
        _onchain_randomness_config: &OnChainRandomnessConfig,
        _onchain_chunky_dkg_config: &OnChainChunkyDKGConfig,
        _features: &Features,
        _rand_config: Option<RandConfig>,
        _secret_share_verifier: Option<Arc<SecretShareVerifier>>,
        _rand_msg_rx: aptos_channel::Receiver<AccountAddress, IncomingRandGenRequest>,
        _secret_sharing_msg_rx: aptos_channel::Receiver<AccountAddress, IncomingSecretShareRequest>,
        _highest_committed_round: Round,
    ) {
    }

    fn get_execution_channel(&self) -> Option<UnboundedSender<OrderedBlocks>> {
        None
    }

    async fn finalize_order(
        &self,
        _: Vec<Arc<PipelinedBlock>>,
        _: WrappedLedgerInfo,
    ) -> ExecutorResult<()> {
        Ok(())
    }

    fn send_commit_msg(&self, _: AccountAddress, _: IncomingCommitRequest) -> Result<()> {
        Ok(())
    }

    async fn sync_for_duration(
        &self,
        _: Duration,
    ) -> Result<LedgerInfoWithSignatures, StateSyncError> {
        Err(StateSyncError::from(anyhow!(
            "sync_for_duration() is not supported by the DummyExecutionClient!"
        )))
    }

    async fn sync_to_target(&self, _: LedgerInfoWithSignatures) -> Result<(), StateSyncError> {
        Ok(())
    }

    async fn reset(&self, _: &LedgerInfoWithSignatures) -> Result<()> {
        Ok(())
    }

    async fn end_epoch(&self) {}

    fn pipeline_builder(&self, _signer: Arc<ValidatorSigner>) -> PipelineBuilder {
        todo!()
    }
}
