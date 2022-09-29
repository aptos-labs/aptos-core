// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block_storage::{BlockRetriever, BlockStore},
    counters,
    error::error_kind,
    monitor,
    network::NetworkSender,
    persistent_liveness_storage::{PersistentLivenessStorage, RecoveryData},
    round_manager::VerifiedEvent,
    state_replication::StateComputer,
};
use anyhow::{anyhow, ensure, Context, Result};
use aptos_logger::prelude::*;
use aptos_types::{block_info::Round, epoch_state::EpochState};
use channel::aptos_channel;
use consensus_types::{
    common::Author, proposal_msg::ProposalMsg, sync_info::SyncInfo, vote_msg::VoteMsg,
};
use futures::{FutureExt, StreamExt};
use futures_channel::oneshot;
use std::{mem::Discriminant, process, sync::Arc};

/// If the node can't recover corresponding blocks from local storage, RecoveryManager is responsible
/// for processing the events carrying sync info and use the info to retrieve blocks from peers
pub struct RecoveryManager {
    epoch_state: EpochState,
    network: NetworkSender,
    storage: Arc<dyn PersistentLivenessStorage>,
    state_computer: Arc<dyn StateComputer>,
    last_committed_round: Round,
}

impl RecoveryManager {
    pub fn new(
        epoch_state: EpochState,
        network: NetworkSender,
        storage: Arc<dyn PersistentLivenessStorage>,
        state_computer: Arc<dyn StateComputer>,
        last_committed_round: Round,
    ) -> Self {
        RecoveryManager {
            epoch_state,
            network,
            storage,
            state_computer,
            last_committed_round,
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
        );
        let recovery_data = BlockStore::fast_forward_sync(
            sync_info.highest_ordered_cert(),
            sync_info.highest_commit_cert(),
            &mut retriever,
            self.storage.clone(),
            self.state_computer.clone(),
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
                            error!(error = ?e, kind = error_kind(&e));
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
