// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    counters,
    error::StateSyncError,
    network::{IncomingCommitRequest, NetworkSender},
    network_interface::{ConsensusMsg, ConsensusNetworkClient},
    payload_manager::PayloadManager,
    pipeline::{
        buffer_manager::{OrderedBlocks, ResetAck, ResetRequest, ResetSignal},
        decoupled_execution_utils::prepare_phases_and_buffer_manager,
        errors::Error,
        signing_phase::CommitSignerProvider,
    },
    state_computer::ExecutionProxy,
    state_replication::{StateComputer, StateComputerCommitCallBackType},
    transaction_deduper::create_transaction_deduper,
    transaction_shuffler::create_transaction_shuffler,
};
use anyhow::Result;
use aptos_bounded_executor::BoundedExecutor;
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_consensus_types::{common::Author, pipelined_block::PipelinedBlock};
use aptos_executor_types::ExecutorResult;
use aptos_infallible::RwLock;
use aptos_logger::prelude::*;
use aptos_network2::{application::interface::NetworkClient, protocols::network::Event};
use aptos_types::{
    epoch_state::EpochState,
    ledger_info::LedgerInfoWithSignatures,
    on_chain_config::{FeatureFlag, Features, OnChainExecutionConfig},
};
use fail::fail_point;
use futures::{
    channel::{mpsc::UnboundedSender, oneshot},
    SinkExt,
};
use futures_channel::mpsc::unbounded;
use move_core_types::account_address::AccountAddress;
use std::sync::Arc;

#[async_trait::async_trait]
pub trait TExecutionClient: Send + Sync {
    /// Initialize the execution phase for a new epoch.
    async fn start_epoch(
        &self,
        epoch_state: Arc<EpochState>,
        commit_signer_provider: Arc<dyn CommitSignerProvider>,
        payload_manager: Arc<PayloadManager>,
        onchain_execution_config: &OnChainExecutionConfig,
        features: &Features,
    );

    /// This is needed for some DAG tests. Clean this up as a TODO.
    fn get_execution_channel(&self) -> Option<UnboundedSender<OrderedBlocks>>;

    /// Send ordered blocks to the real execution phase through the channel.
    async fn finalize_order(
        &self,
        blocks: &[Arc<PipelinedBlock>],
        ordered_proof: LedgerInfoWithSignatures,
        callback: StateComputerCommitCallBackType,
    ) -> ExecutorResult<()>;

    fn send_commit_msg(
        &self,
        peer_id: AccountAddress,
        commit_msg: IncomingCommitRequest,
    ) -> Result<()>;

    /// Synchronize to a commit that not present locally.
    async fn sync_to(&self, target: LedgerInfoWithSignatures) -> Result<(), StateSyncError>;

    /// Shutdown the current processor at the end of the epoch.
    async fn end_epoch(&self);
}

struct BufferManagerHandle {
    pub execute_tx: Option<UnboundedSender<OrderedBlocks>>,
    pub commit_tx: Option<aptos_channel::Sender<AccountAddress, IncomingCommitRequest>>,
    pub reset_tx: Option<UnboundedSender<ResetRequest>>,
}

impl BufferManagerHandle {
    pub fn new() -> Self {
        Self {
            execute_tx: None,
            commit_tx: None,
            reset_tx: None,
        }
    }

    pub fn init(
        &mut self,
        execute_tx: UnboundedSender<OrderedBlocks>,
        commit_tx: aptos_channel::Sender<AccountAddress, IncomingCommitRequest>,
        reset_tx: UnboundedSender<ResetRequest>,
    ) {
        self.execute_tx = Some(execute_tx);
        self.commit_tx = Some(commit_tx);
        self.reset_tx = Some(reset_tx);
    }

    pub fn reset(&mut self) -> Option<UnboundedSender<ResetRequest>> {
        let reset_tx = self.reset_tx.take();
        self.execute_tx = None;
        self.commit_tx = None;
        self.reset_tx = None;
        reset_tx
    }
}

pub struct ExecutionProxyClient {
    execution_proxy: Arc<ExecutionProxy>,
    author: Author,
    self_sender: aptos_channels::Sender<Event<ConsensusMsg>>,
    network_sender: ConsensusNetworkClient<NetworkClient<ConsensusMsg>>,
    bounded_executor: BoundedExecutor,
    // channels to buffer manager
    handle: Arc<RwLock<BufferManagerHandle>>,
}

impl ExecutionProxyClient {
    pub fn new(
        execution_proxy: Arc<ExecutionProxy>,
        author: Author,
        self_sender: aptos_channels::Sender<Event<ConsensusMsg>>,
        network_sender: ConsensusNetworkClient<NetworkClient<ConsensusMsg>>,
        bounded_executor: BoundedExecutor,
    ) -> Self {
        Self {
            execution_proxy,
            author,
            self_sender,
            network_sender,
            bounded_executor,
            handle: Arc::new(RwLock::new(BufferManagerHandle::new())),
        }
    }

    fn spawn_decoupled_execution(
        &self,
        commit_signer_provider: Arc<dyn CommitSignerProvider>,
        epoch_state: Arc<EpochState>,
    ) {
        let network_sender = NetworkSender::new(
            self.author,
            self.network_sender.clone(),
            self.self_sender.clone(),
            epoch_state.verifier.clone(),
        );

        let (block_tx, block_rx) = unbounded::<OrderedBlocks>();
        let (reset_tx, reset_rx) = unbounded::<ResetRequest>();

        let (commit_msg_tx, commit_msg_rx) =
            aptos_channel::new::<AccountAddress, IncomingCommitRequest>(
                QueueStyle::FIFO,
                100,
                Some(&counters::BUFFER_MANAGER_MSGS),
            );

        self.handle.write().init(block_tx, commit_msg_tx, reset_tx);

        let (
            execution_schedule_phase,
            execution_wait_phase,
            signing_phase,
            persisting_phase,
            buffer_manager,
        ) = prepare_phases_and_buffer_manager(
            self.author,
            self.execution_proxy.clone(),
            commit_signer_provider,
            network_sender,
            commit_msg_rx,
            self.execution_proxy.clone(),
            block_rx,
            reset_rx,
            epoch_state,
            self.bounded_executor.clone(),
        );

        tokio::spawn(execution_schedule_phase.start());
        tokio::spawn(execution_wait_phase.start());
        tokio::spawn(signing_phase.start());
        tokio::spawn(persisting_phase.start());
        tokio::spawn(buffer_manager.start());
    }
}

#[async_trait::async_trait]
impl TExecutionClient for ExecutionProxyClient {
    async fn start_epoch(
        &self,
        epoch_state: Arc<EpochState>,
        commit_signer_provider: Arc<dyn CommitSignerProvider>,
        payload_manager: Arc<PayloadManager>,
        onchain_execution_config: &OnChainExecutionConfig,
        features: &Features,
    ) {
        self.spawn_decoupled_execution(commit_signer_provider, epoch_state.clone());

        let transaction_shuffler =
            create_transaction_shuffler(onchain_execution_config.transaction_shuffler_type());
        let block_executor_onchain_config =
            onchain_execution_config.block_executor_onchain_config();
        let transaction_deduper =
            create_transaction_deduper(onchain_execution_config.transaction_deduper_type());
        self.execution_proxy.new_epoch(
            &epoch_state,
            payload_manager,
            transaction_shuffler,
            block_executor_onchain_config,
            transaction_deduper,
            features.is_enabled(FeatureFlag::RECONFIGURE_WITH_DKG),
        );
    }

    fn get_execution_channel(&self) -> Option<UnboundedSender<OrderedBlocks>> {
        self.handle.read().execute_tx.clone()
    }

    async fn finalize_order(
        &self,
        blocks: &[Arc<PipelinedBlock>],
        ordered_proof: LedgerInfoWithSignatures,
        callback: StateComputerCommitCallBackType,
    ) -> ExecutorResult<()> {
        assert!(!blocks.is_empty());
        let execute_tx = self.handle.read().execute_tx.clone();

        if execute_tx.is_none() {
            debug!("Failed to send to buffer manager, maybe epoch ends");
            return Ok(());
        }

        for block in blocks {
            block.set_insertion_time();
        }

        if execute_tx
            .unwrap()
            .send(OrderedBlocks {
                ordered_blocks: blocks
                    .iter()
                    .map(|b| (**b).clone())
                    .collect::<Vec<PipelinedBlock>>(),
                ordered_proof,
                callback,
            })
            .await
            .is_err()
        {
            debug!("Failed to send to buffer manager, maybe epoch ends");
        }
        Ok(())
    }

    fn send_commit_msg(
        &self,
        peer_id: AccountAddress,
        commit_msg: IncomingCommitRequest,
    ) -> Result<()> {
        if let Some(tx) = &self.handle.read().commit_tx {
            tx.push(peer_id, commit_msg)
        } else {
            counters::EPOCH_MANAGER_ISSUES_DETAILS
                .with_label_values(&["buffer_manager_not_started"])
                .inc();
            warn!("Buffer manager not started");
            Ok(())
        }
    }

    async fn sync_to(&self, target: LedgerInfoWithSignatures) -> Result<(), StateSyncError> {
        fail_point!("consensus::sync_to", |_| {
            Err(anyhow::anyhow!("Injected error in sync_to").into())
        });

        let reset_tx = self.handle.read().reset_tx.clone();

        if let Some(mut reset_tx) = reset_tx {
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

        // TODO: handle the sync error, should re-push the ordered blocks to buffer manager
        // when it's reset but sync fails.
        self.execution_proxy.sync_to(target).await?;
        Ok(())
    }

    async fn end_epoch(&self) {
        let reset_tx = self.handle.write().reset();
        if let Some(mut tx) = reset_tx {
            let (ack_tx, ack_rx) = oneshot::channel();
            tx.send(ResetRequest {
                tx: ack_tx,
                signal: ResetSignal::Stop,
            })
            .await
            .expect("[EpochManager] Fail to drop buffer manager");
            ack_rx
                .await
                .expect("[EpochManager] Fail to drop buffer manager");
        }
        self.execution_proxy.end_epoch();
    }
}

pub struct DummyExecutionClient;

#[async_trait::async_trait]
impl TExecutionClient for DummyExecutionClient {
    async fn start_epoch(
        &self,
        _epoch_state: Arc<EpochState>,
        _commit_signer_provider: Arc<dyn CommitSignerProvider>,
        _payload_manager: Arc<PayloadManager>,
        _onchain_execution_config: &OnChainExecutionConfig,
        _features: &Features,
    ) {
    }

    fn get_execution_channel(&self) -> Option<UnboundedSender<OrderedBlocks>> {
        None
    }

    async fn finalize_order(
        &self,
        _: &[Arc<PipelinedBlock>],
        _: LedgerInfoWithSignatures,
        _: StateComputerCommitCallBackType,
    ) -> ExecutorResult<()> {
        Ok(())
    }

    fn send_commit_msg(&self, _: AccountAddress, _: IncomingCommitRequest) -> Result<()> {
        Ok(())
    }

    async fn sync_to(&self, _: LedgerInfoWithSignatures) -> Result<(), StateSyncError> {
        Ok(())
    }

    async fn end_epoch(&self) {}
}
