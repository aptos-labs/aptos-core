// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block_storage::tracing::{observe_block, BlockStage},
    monitor,
    payload_manager::PayloadManager,
    round_manager::{UnverifiedEvent, VerifiedEvent},
};
use anyhow::{bail, Context};
use aptos_channels::aptos_channel;
use aptos_consensus_types::common::Author;
use aptos_logger::{error, warn, SecurityEvent};
use aptos_types::epoch_state::EpochState;
use move_core_types::account_address::AccountAddress;
use std::{
    hash::Hash,
    mem::{discriminant, Discriminant},
    sync::Arc,
};
use tokio::runtime::Handle;

// Verifies the consensus messages and forwards them to the corresponding components - done by having a
// dedicated thread pool for signature verification so that we don't block the consensus runtime.
pub struct ConsensusMsgVerifier {
    my_peer_id: Author,
    epoch_state: Option<Arc<EpochState>>,
    quorum_store_enabled: bool,
    quorum_store_msg_tx: Option<aptos_channel::Sender<AccountAddress, VerifiedEvent>>,
    buffered_proposal_tx: Option<aptos_channel::Sender<Author, VerifiedEvent>>,
    round_manager_tx: Option<
        aptos_channel::Sender<(Author, Discriminant<VerifiedEvent>), (Author, VerifiedEvent)>,
    >,
    payload_manager: Arc<PayloadManager>,
    /// Dedicated async runtime used to verify signatures.
    executor: Handle,
    max_qs_batch: usize,
}

impl ConsensusMsgVerifier {
    pub fn new(
        my_peer_id: Author,
        executor: Handle,
        epoch_state: Option<Arc<EpochState>>,
        quorum_store_enabled: bool,
        quorum_store_msg_tx: Option<aptos_channel::Sender<AccountAddress, VerifiedEvent>>,
        buffered_proposal_tx: Option<aptos_channel::Sender<Author, VerifiedEvent>>,
        round_manager_tx: Option<
            aptos_channel::Sender<(Author, Discriminant<VerifiedEvent>), (Author, VerifiedEvent)>,
        >,
        payload_manager: Arc<PayloadManager>,
        max_qs_batch: usize,
    ) -> Self {
        Self {
            my_peer_id,
            epoch_state,
            quorum_store_enabled,
            quorum_store_msg_tx,
            buffered_proposal_tx,
            round_manager_tx,
            payload_manager,
            executor,
            max_qs_batch,
        }
    }

    pub fn verify_and_forward(&self, unverified_event: UnverifiedEvent, peer_id: AccountAddress) {
        let epoch_state = self.epoch_state.clone().unwrap();
        let quorum_store_enabled = self.quorum_store_enabled;
        let quorum_store_msg_tx = self.quorum_store_msg_tx.clone();
        let buffered_proposal_tx = self.buffered_proposal_tx.clone();
        let round_manager_tx = self.round_manager_tx.clone();
        let my_peer_id = self.my_peer_id;
        let max_num_batches = self.max_qs_batch;
        let payload_manager = self.payload_manager.clone();

        self.executor.spawn(async move {
            match monitor!(
                "verify_message",
                unverified_event.clone().verify(
                    peer_id,
                    &epoch_state.verifier,
                    quorum_store_enabled,
                    peer_id == my_peer_id,
                    max_num_batches,
                )
            ) {
                Ok(verified_event) => {
                    Self::forward_event(
                        quorum_store_msg_tx,
                        round_manager_tx,
                        buffered_proposal_tx,
                        peer_id,
                        verified_event,
                        payload_manager,
                    );
                },
                Err(e) => {
                    error!(
                        SecurityEvent::ConsensusInvalidMessage,
                        remote_peer = peer_id,
                        error = ?e,
                        unverified_event = unverified_event
                    );
                },
            }
        });
    }

    fn forward_event(
        quorum_store_msg_tx: Option<aptos_channel::Sender<AccountAddress, VerifiedEvent>>,
        round_manager_tx: Option<
            aptos_channel::Sender<(Author, Discriminant<VerifiedEvent>), (Author, VerifiedEvent)>,
        >,
        buffered_proposal_tx: Option<aptos_channel::Sender<Author, VerifiedEvent>>,
        peer_id: AccountAddress,
        event: VerifiedEvent,
        payload_manager: Arc<PayloadManager>,
    ) {
        if let VerifiedEvent::ProposalMsg(proposal) = &event {
            observe_block(
                proposal.proposal().timestamp_usecs(),
                BlockStage::EPOCH_MANAGER_VERIFIED,
            );
        }
        if let Err(e) = match event {
            quorum_store_event @ (VerifiedEvent::SignedBatchInfo(_)
            | VerifiedEvent::ProofOfStoreMsg(_)
            | VerifiedEvent::BatchMsg(_)) => {
                Self::forward_event_to(quorum_store_msg_tx, peer_id, quorum_store_event)
                    .context("quorum store sender")
            },
            proposal_event @ VerifiedEvent::ProposalMsg(_) => {
                if let VerifiedEvent::ProposalMsg(p) = &proposal_event {
                    if let Some(payload) = p.proposal().payload() {
                        payload_manager
                            .prefetch_payload_data(payload, p.proposal().timestamp_usecs());
                    }
                }
                Self::forward_event_to(buffered_proposal_tx, peer_id, proposal_event)
                    .context("proposal precheck sender")
            },
            round_manager_event => Self::forward_event_to(
                round_manager_tx,
                (peer_id, discriminant(&round_manager_event)),
                (peer_id, round_manager_event),
            )
            .context("round manager sender"),
        } {
            warn!("Failed to forward event: {}", e);
        }
    }

    fn forward_event_to<K: Eq + Hash + Clone, V>(
        mut maybe_tx: Option<aptos_channel::Sender<K, V>>,
        key: K,
        value: V,
    ) -> anyhow::Result<()> {
        if let Some(tx) = &mut maybe_tx {
            tx.push(key, value)
        } else {
            bail!("channel not initialized");
        }
    }
}
