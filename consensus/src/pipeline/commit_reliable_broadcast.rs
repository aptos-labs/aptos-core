// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{counters, network::NetworkSender, network_interface::ConsensusMsg};
use anyhow::bail;
use velor_consensus_types::{
    common::Author,
    pipeline::{commit_decision::CommitDecision, commit_vote::CommitVote},
};
use velor_infallible::Mutex;
use velor_reliable_broadcast::{BroadcastStatus, RBMessage, RBNetworkSender};
use velor_types::{validator_verifier::ValidatorVerifier, PeerId};
use async_trait::async_trait;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Network message for the pipeline phase
pub enum CommitMessage {
    /// Vote on execution result
    Vote(CommitVote),
    /// Quorum proof on execution result
    Decision(CommitDecision),
    /// Ack on either vote or decision
    Ack(()),
    /// Nack is non-acknowledgement, we got your message, but it was bad/we were bad
    Nack,
}

impl CommitMessage {
    /// Verify the signatures on the message
    pub fn verify(&self, sender: Author, verifier: &ValidatorVerifier) -> anyhow::Result<()> {
        match self {
            CommitMessage::Vote(vote) => {
                let _timer = counters::VERIFY_MSG
                    .with_label_values(&["commit_vote"])
                    .start_timer();
                vote.verify(sender, verifier)
            },
            CommitMessage::Decision(decision) => {
                let _timer = counters::VERIFY_MSG
                    .with_label_values(&["commit_decision"])
                    .start_timer();
                decision.verify(verifier)
            },
            CommitMessage::Ack(_) => bail!("Unexpected ack in incoming commit message"),
            CommitMessage::Nack => bail!("Unexpected NACK in incoming commit message"),
        }
    }

    pub fn epoch(&self) -> Option<u64> {
        match self {
            CommitMessage::Vote(vote) => Some(vote.epoch()),
            CommitMessage::Decision(decision) => Some(decision.epoch()),
            _ => None,
        }
    }
}

impl RBMessage for CommitMessage {}

pub struct AckState {
    validators: Mutex<HashSet<Author>>,
}

impl AckState {
    pub fn new(validators: impl Iterator<Item = Author>) -> Arc<Self> {
        Arc::new(Self {
            validators: Mutex::new(validators.collect()),
        })
    }
}

impl BroadcastStatus<CommitMessage> for Arc<AckState> {
    type Aggregated = ();
    type Message = CommitMessage;
    type Response = CommitMessage;

    fn add(&self, peer: Author, ack: Self::Response) -> anyhow::Result<Option<Self::Aggregated>> {
        match ack {
            CommitMessage::Vote(_) => {
                bail!("unexected Vote reply to broadcast");
            },
            CommitMessage::Decision(_) => {
                bail!("unexected Decision reply to broadcast");
            },
            CommitMessage::Ack(_) => {
                // okay! continue
            },
            CommitMessage::Nack => {
                bail!("unexected Nack reply to broadcast");
            },
        }
        let mut validators = self.validators.lock();
        if validators.remove(&peer) {
            if validators.is_empty() {
                Ok(Some(()))
            } else {
                Ok(None)
            }
        } else {
            bail!("Unknown author: {}", peer);
        }
    }
}

#[async_trait]
impl RBNetworkSender<CommitMessage> for NetworkSender {
    async fn send_rb_rpc_raw(
        &self,
        receiver: Author,
        raw_message: Bytes,
        timeout_duration: Duration,
    ) -> anyhow::Result<CommitMessage> {
        let response = match self
            .consensus_network_client
            .send_rpc_raw(receiver, raw_message, timeout_duration)
            .await?
        {
            ConsensusMsg::CommitMessage(resp) if matches!(*resp, CommitMessage::Ack(_)) => *resp,
            ConsensusMsg::CommitMessage(resp) if matches!(*resp, CommitMessage::Nack) => {
                bail!("Received nack, will retry")
            },
            _ => bail!("Invalid response to request"),
        };

        Ok(response)
    }

    async fn send_rb_rpc(
        &self,
        receiver: Author,
        message: CommitMessage,
        timeout: Duration,
    ) -> anyhow::Result<CommitMessage> {
        let req = ConsensusMsg::CommitMessage(Box::new(message));
        let response = match self.send_rpc(receiver, req, timeout).await? {
            ConsensusMsg::CommitMessage(resp) if matches!(*resp, CommitMessage::Ack(_)) => *resp,
            ConsensusMsg::CommitMessage(resp) if matches!(*resp, CommitMessage::Nack) => {
                bail!("Received nack, will retry")
            },
            _ => bail!("Invalid response to request"),
        };

        Ok(response)
    }

    fn to_bytes_by_protocol(
        &self,
        peers: Vec<Author>,
        message: CommitMessage,
    ) -> Result<HashMap<Author, bytes::Bytes>, anyhow::Error> {
        let msg = ConsensusMsg::CommitMessage(Box::new(message));
        self.consensus_network_client
            .to_bytes_by_protocol(peers, msg)
    }

    fn sort_peers_by_latency(&self, peers: &mut [PeerId]) {
        self.sort_peers_by_latency(peers);
    }
}
