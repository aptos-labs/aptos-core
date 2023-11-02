// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{network::NetworkSender, network_interface::ConsensusMsg};
use anyhow::bail;
pub use aptos_consensus_types::common::Author;
use aptos_consensus_types::common::Round;
use aptos_crypto::HashValue;
use aptos_dkg::weighted_vuf::traits::WeightedVUF;
use aptos_enum_conversion_derive::EnumConversion;
use aptos_reliable_broadcast::{BroadcastStatus, RBMessage, RBNetworkSender};
use aptos_types::{randomness::{RandConfig, RandDecision, Mode, ProofShare, Delta, RandMetadata, WVUF}};
use async_trait::async_trait;
use futures_channel::mpsc::UnboundedSender;
use serde::{Deserialize, Serialize};
use std::{time::Duration, collections::HashSet};

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct RandShare {
    author: Author,
    mode: Mode,
    metadata: RandMetadata,
    share: ProofShare,
    apk_delta: Option<Delta>,
}

impl RandShare {
    pub fn new(author: Author, mode: Mode, metadata: RandMetadata, share: ProofShare, apk_delta: Option<Delta>) -> Self {
        Self { author, mode, metadata, share, apk_delta }
    }

    pub fn author(&self) -> &Author {
        &self.author
    }

    pub fn mode(&self) -> &Mode {
        &self.mode
    }

    pub fn metadata(&self) -> &RandMetadata {
        &self.metadata
    }

    pub fn id(&self) -> HashValue {
        self.metadata.block_id
    }

    pub fn round(&self) -> Round {
        self.metadata.round
    }

    pub fn epoch(&self) -> u64 {
        self.metadata.epoch
    }

    pub fn timestamp(&self) -> u64 {
        self.metadata.timestamp
    }

    pub fn share(&self) -> &ProofShare {
        &self.share
    }

    pub fn apk_delta(&self) -> &Option<Delta> {
        &self.apk_delta
    }
}

impl RandShare {
    pub fn verify(&self, mode: Mode, rand_config: &RandConfig) -> anyhow::Result<()> {
        assert_eq!(self.mode, mode, "[RandShare] Invalid mode");
        let index = *rand_config.validator.address_to_validator_index().get(&self.author).unwrap();
        let maybe_apk = match self.mode {
            Mode::Optimistic => &rand_config.keys_o.apks[index],
            Mode::Fallback => &rand_config.keys_f.apks[index],
        };
        if let Some(apk) = maybe_apk {
            <WVUF as WeightedVUF>::verify_share(&rand_config.vuf_pp, apk, self.metadata.to_bytes().as_slice(), &self.share)?;
        } else {
            bail!("[RandShare] No augmented public key for validator id {}, {}", index, self.author);
        }
        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ShareAck {
    pub maybe_decision: Option<RandDecision>,
    pub missing_apk: bool,
}

impl ShareAck {
    pub fn new(maybe_decision: Option<RandDecision>, missing_apk: bool) -> Self {
        Self { maybe_decision, missing_apk }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DeltaMsg {
    pub author: Author,
    pub delta: Delta,
}

impl DeltaMsg {
    pub fn new(author: Author, delta: Delta) -> Self {
        Self { author, delta }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, EnumConversion)]
pub enum RandMessage {
    Share(RandShare),
    ShareAck(ShareAck),
    Delta(DeltaMsg),
    DeltaAck(()),
}

impl RandMessage {
    // pub fn epoch(&self) -> anyhow::Result<u64>{
    //     match self {
    //         RandMessage::Share(share) => Ok(share.epoch()),
    //         _ => bail!("[RandMessage] Unexpected ack in incoming randomness message"),
    //     }
    // }

    pub fn name(&self) -> &'static str {
        match self {
            RandMessage::Share(_) => "RandMessage::Share",
            RandMessage::ShareAck(_) => "RandMessage::ShareAck",
            RandMessage::Delta(_) => "RandMessage::Delta",
            RandMessage::DeltaAck(_) => "RandMessage::DeltaAck",
        }
    }
}

impl RBMessage for RandMessage {}

pub struct ShareAckState {
    validators: HashSet<Author>,
    rand_config: RandConfig,
    rand_decision_tx: UnboundedSender<RandDecision>,
    send_delta_request_tx: UnboundedSender<Author>,
}

impl ShareAckState {
    pub fn new(validators: impl Iterator<Item = Author>, rand_config: RandConfig, rand_decision_tx: UnboundedSender<RandDecision>, send_delta_request_tx: UnboundedSender<Author>) -> Self {
        Self { 
            validators: validators.collect(),
            rand_config,
            rand_decision_tx,
            send_delta_request_tx,
        }
    }
}

impl BroadcastStatus<RandMessage> for ShareAckState {
    type Ack = ShareAck;
    type Aggregated = ();
    type Message = RandShare;

    fn add(&mut self, peer: Author, ack: Self::Ack) -> anyhow::Result<Option<Self::Aggregated>> { 
        if self.validators.remove(&peer) {
            if ack.missing_apk {
                // send delta to peer if missing
                let _ = self.send_delta_request_tx.unbounded_send(peer);
            }
            // If receive a decision, verify it and send it to the randomness manager and stop the reliable broadcast
            if let Some(decision) = ack.maybe_decision {
                match decision.verify(&self.rand_config) {
                    Ok(()) => {
                        let _ = self.rand_decision_tx.unbounded_send(decision);
                        return Ok(Some(()));
                    },
                    Err(e) => {
                        bail!("[RandMessage] Invalid decision from {}: {}", peer, e);
                    },
                }
            }
            // If receive from all validators, stop the reliable broadcast
            if self.validators.is_empty() {
                Ok(Some(()))
            } else {
                Ok(None)
            }
        } else {
            bail!("[RandMessage] Unknown author: {}", peer);
        }
    }
}

#[async_trait]
impl RBNetworkSender<RandMessage> for NetworkSender {
    async fn send_rb_rpc(
        &self,
        receiver: Author,
        message: RandMessage,
        timeout_duration: Duration,
    ) -> anyhow::Result<RandMessage> {
        let msg = ConsensusMsg::RandMessage(message.into());
        let response = match self.send_rpc(receiver, msg, timeout_duration).await? {
            ConsensusMsg::RandMessage(resp) if matches!(*resp, RandMessage::ShareAck(_) | RandMessage::DeltaAck(_)) => *resp,
            _ => bail!("[RandMessage] Invalid response to request"),
        };

        Ok(response)
    }
}