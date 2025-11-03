// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block_storage::{pending_blocks::PendingBlocks, BlockRetriever, BlockStore},
    counters,
    error::error_kind,
    monitor,
    network::NetworkSender,
    payload_manager::TPayloadManager,
    persistent_liveness_storage::{PersistentLivenessStorage, RecoveryData},
    pipeline::execution_client::TExecutionClient,
    round_manager::VerifiedEvent,
};
use anyhow::{anyhow, ensure, Context, Result};
use aptos_channels::aptos_channel;
use aptos_consensus_types::{
    common::Author, proposal_msg::ProposalMsg, sync_info::SyncInfo, vote_msg::VoteMsg,
};
use aptos_infallible::Mutex;
use aptos_logger::prelude::*;
use aptos_types::{block_info::Round, epoch_state::EpochState};
use futures::{FutureExt, StreamExt};
use futures_channel::oneshot;
use std::{mem::Discriminant, process, sync::Arc};

/// If the node can't recover corresponding blocks from local storage, RecoveryManager is responsible
/// for processing the events carrying sync info and use the info to retrieve blocks from peers
pub struct RecoveryManager {
    epoch_state: Arc<EpochState>,
    network: Arc<NetworkSender>,
    storage: Arc<dyn PersistentLivenessStorage>,
    execution_client: Arc<dyn TExecutionClient>,
    last_committed_round: Round,
    max_blocks_to_request: u64,
    payload_manager: Arc<dyn TPayloadManager>,
    order_vote_enabled: bool,
    window_size: Option<u64>,
    pending_blocks: Arc<Mutex<PendingBlocks>>,
}

impl RecoveryManager {
    pub fn new(
        epoch_state: Arc<EpochState>,
        network: Arc<NetworkSender>,
        storage: Arc<dyn PersistentLivenessStorage>,
        execution_client: Arc<dyn TExecutionClient>,
        last_committed_round: Round,
        max_blocks_to_request: u64,
        payload_manager: Arc<dyn TPayloadManager>,
        order_vote_enabled: bool,
        window_size: Option<u64>,
        pending_blocks: Arc<Mutex<PendingBlocks>>,
    ) -> Self {
        RecoveryManager {
            epoch_state,
            network,
            storage,
            execution_client,
            last_committed_round,
            max_blocks_to_request,
            payload_manager,
            order_vote_enabled,
            window_size,
            pending_blocks,
        }
    }

    pub async fn process_proposal_msg(
        &mut self,
        proposal_msg: ProposalMsg,
    ) -> Result<RecoveryData> {
        let author = proposal_msg.proposer();
        let sync_info = proposal_msg.sync_info();
        self.sync_up(sync_info, author).await
    }

    pub async fn process_vote_msg(&mut self, vote_msg: VoteMsg) -> Result<RecoveryData> {
        let author = vote_msg.vote().author();
        let sync_info = vote_msg.sync_info();
        self.sync_up(sync_info, author).await
    }

    pub async fn sync_up(&mut self, sync_info: &SyncInfo, peer: Author) -> Result<RecoveryData> {
        sync_info.verify(&self.epoch_state.verifier)?;
        ensure!(
            sync_info.highest_round() > self.last_committed_round,
            "[RecoveryManager] Received sync info has lower round number than committed block"
        );
        ensure!(
            sync_info.epoch() == self.epoch_state.epoch,
            "[RecoveryManager] Received sync info is in different epoch than committed block"
        );
        let mut retriever = BlockRetriever::new(
            self.network.clone(),
            peer,
            self.epoch_state
                .verifier
                .get_ordered_account_addresses_iter()
                .collect(),
            self.max_blocks_to_request,
            self.pending_blocks.clone(),
        );
        let recovery_data = BlockStore::fast_forward_sync(
            sync_info.highest_quorum_cert(),
            sync_info.highest_commit_cert(),
            &mut retriever,
            self.storage.clone(),
            self.execution_client.clone(),
            self.payload_manager.clone(),
            self.order_vote_enabled,
            self.window_size,
            None,
        )
        .await?;

        Ok(recovery_data)
    }

    pub async fn start(
        mut self,
        mut event_rx: aptos_channel::Receiver<
            (Author, Discriminant<VerifiedEvent>),
            (Author, VerifiedEvent),
        >,
        close_rx: oneshot::Receiver<oneshot::Sender<()>>,
    ) {
        info!(epoch = self.epoch_state.epoch, "RecoveryManager started");
        let mut close_rx = close_rx.into_stream();
        loop {
            futures::select! {
                (peer_id, event) = event_rx.select_next_some() => {
                    let result = match event {
                        VerifiedEvent::ProposalMsg(proposal_msg) => {
                            monitor!(
                                "process_recovery",
                                self.process_proposal_msg(*proposal_msg).await
                            )
                        }
                        VerifiedEvent::VoteMsg(vote_msg) => {
                            monitor!("process_recovery", self.process_vote_msg(*vote_msg).await)
                        }
                        VerifiedEvent::UnverifiedSyncInfo(sync_info) => {
                            monitor!(
                                "process_recovery",
                                self.sync_up(&sync_info, peer_id).await
                            )
                        }
                        unexpected_event => Err(anyhow!("Unexpected event: {:?}", unexpected_event)),
                    }
                    .with_context(|| format!("from peer {}", peer_id));

                    match result {
                        Ok(_) => {
                            info!("Recovery finishes for epoch {}, RecoveryManager stopped. Please restart the node", self.epoch_state.epoch);
                            process::exit(0);
                        },
                        Err(e) => {
                            counters::ERROR_COUNT.inc();
                            warn!(error = ?e, kind = error_kind(&e));
                        }
                    }
                }
                close_req = close_rx.select_next_some() => {
                    if let Ok(ack_sender) = close_req {
                        ack_sender.send(()).expect("[RecoveryManager] Fail to ack shutdown");
                    }
                    break;
                }
            }
        }
        info!(epoch = self.epoch_state.epoch, "RecoveryManager stopped");
    }
}
