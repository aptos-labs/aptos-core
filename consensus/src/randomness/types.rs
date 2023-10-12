// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{network::NetworkSender, network_interface::ConsensusMsg};
use anyhow::bail;
pub use aptos_consensus_types::common::Author;
use aptos_consensus_types::common::Round;
use aptos_crypto::HashValue;
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use aptos_enum_conversion_derive::EnumConversion;
use aptos_reliable_broadcast::{BroadcastStatus, RBMessage, RBNetworkSender};
use aptos_types::{block_info::BlockInfo, randomness::{RandConfig, RandDecision, SHARE_SIZE, NUM_SHARES_PER_VALIDATOR}};
use async_trait::async_trait;
use futures_channel::mpsc::UnboundedSender;
use serde::{Deserialize, Serialize};
use std::{time::Duration, collections::HashSet};


#[derive(Clone, Serialize, Deserialize, CryptoHasher, Debug, PartialEq)]
pub struct ShareWrapper {
    // rand todo: fill
    bytes: Vec<u8>,
}

impl ShareWrapper {
    // only for testing
    pub fn new_for_test() -> Self {
        Self { bytes: vec![u8::MAX; SHARE_SIZE] }
    }

    pub fn verify(&self, _rand_config: &RandConfig) -> anyhow::Result<()> {
        // rand todo: fill
        Ok(())
    }

    // rand todo: more, aggregation
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, CryptoHasher, BCSCryptoHash)]
pub struct RandShareMetadata {
    author: Author,
    block_info: BlockInfo,
    weight: u64,
}

impl RandShareMetadata {
    pub fn new(author: Author, block_info: BlockInfo, weight: u64) -> Self {
        Self { author, block_info, weight }
    }
}

#[derive(Clone, Serialize, Deserialize, CryptoHasher, Debug, PartialEq)]
pub struct RandShare {
    metadata: RandShareMetadata,
    share: ShareWrapper,
}

impl RandShare {
    pub fn new(author: Author, block_info: BlockInfo, weight: u64, share: ShareWrapper) -> Self {
        Self {
            metadata: RandShareMetadata { author, block_info, weight },
            share,
        }
    }

    pub fn new_for_test(author: Author, block_info: BlockInfo) -> Self {
        Self {
            metadata: RandShareMetadata { author, block_info, weight: NUM_SHARES_PER_VALIDATOR as u64 },
            share: ShareWrapper::new_for_test(),
        }
    }

    pub fn metadata(&self) -> &RandShareMetadata {
        &self.metadata
    }

    pub fn author(&self) -> &Author {
        &self.metadata.author
    }

    pub fn block_info(&self) -> &BlockInfo {
        &self.metadata.block_info
    }

    pub fn weight(&self) -> u64 {
        self.metadata.weight
    }

    pub fn id(&self) -> HashValue {
        self.metadata.block_info.id()
    }

    pub fn round(&self) -> Round {
        self.metadata.block_info.round()
    }

    pub fn epoch(&self) -> u64 {
        self.metadata.block_info.epoch()
    }

    pub fn timestamp(&self) -> u64 {
        self.metadata.block_info.timestamp_usecs()
    }

    pub fn share(&self) -> &ShareWrapper {
        &self.share
    }
}

impl RandShare {
    pub fn verify(&self, rand_config: &RandConfig) -> anyhow::Result<()> {
        self.share.verify(rand_config)?;

        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, EnumConversion)]
pub enum RandMessage {
    Share(RandShare),
    ShareAck(Option<RandDecision>),
}

impl RandMessage {
    pub fn epoch(&self) -> anyhow::Result<u64>{
        match self {
            RandMessage::Share(share) => Ok(share.block_info().epoch()),
            _ => bail!("[Randomness] Unexpected ack in incoming randomness message"),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            RandMessage::Share(_) => "RandMessage::Share",
            RandMessage::ShareAck(_) => "RandMessage::ShareAck",
        }
    }
}

impl RBMessage for RandMessage {}

pub struct ShareAckState {
    validators: HashSet<Author>,
    rand_config: RandConfig,
    rand_decision_tx: UnboundedSender<RandDecision>,
}

impl ShareAckState {
    pub fn new(validators: impl Iterator<Item = Author>, rand_config: RandConfig, rand_decision_tx: UnboundedSender<RandDecision>) -> Self {
        Self { 
            validators: validators.collect(),
            rand_config,
            rand_decision_tx,
        }
    }
}

impl BroadcastStatus<RandMessage> for ShareAckState {
    type Ack = Option<RandDecision>;
    type Aggregated = ();
    type Message = RandShare;

    fn add(&mut self, peer: Author, ack: Self::Ack) -> anyhow::Result<Option<Self::Aggregated>> { 
        if self.validators.remove(&peer) {
            // If receive a decision, verify it and send it to the randomness manager and stop the reliable broadcast
            if let Some(decision) = ack {
                match decision.verify(&self.rand_config) {
                    Ok(()) => {
                        let _ = self.rand_decision_tx.unbounded_send(decision);
                        return Ok(Some(()));
                    },
                    Err(e) => {
                        bail!("[Randomness] Invalid decision from {}: {}", peer, e);
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
            bail!("[Randomness] Unknown author: {}", peer);
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
            ConsensusMsg::RandMessage(resp) if matches!(*resp, RandMessage::ShareAck(_)) => *resp,
            _ => bail!("[Randomness] Invalid response to request"),
        };

        Ok(response)
    }
}