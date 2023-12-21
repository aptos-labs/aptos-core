// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::network_interface::DKGMsg;
use anyhow::bail;
pub use aptos_consensus_types::common::Author;
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use aptos_enum_conversion_derive::EnumConversion;
use aptos_infallible::Mutex;
use aptos_reliable_broadcast::{BroadcastStatus, RBMessage};
pub use aptos_types::dkg::DKGAggNode;
use aptos_types::{
    dkg::{DKGPvssConfig, DKGTranscriptWrapper},
    validator_verifier::ValidatorVerifier,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, sync::Arc};

pub trait TDKGMessage: Into<DKGMessage> + TryFrom<DKGMessage> {
    fn verify(
        &self,
        dkg_pvss_config: &DKGPvssConfig,
        verifier: &ValidatorVerifier,
    ) -> anyhow::Result<()>;
}

impl TDKGMessage for DKGNodeAck {
    fn verify(
        &self,
        _dkg_pvss_config: &DKGPvssConfig,
        _verifier: &ValidatorVerifier,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

impl TDKGMessage for DKGAggNodeAck {
    fn verify(
        &self,
        _dkg_pvss_config: &DKGPvssConfig,
        _verifier: &ValidatorVerifier,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

/// Represents the metadata about the node, without payload and parents from Node
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, CryptoHasher, BCSCryptoHash)]
pub struct DKGNodeMetadata {
    epoch: u64,
    author: Author,
}

impl DKGNodeMetadata {
    #[cfg(test)]
    pub fn new_for_test(epoch: u64, author: Author) -> Self {
        Self { epoch, author }
    }

    pub fn author(&self) -> &Author {
        &self.author
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }
}

#[derive(Clone, Serialize, Deserialize, CryptoHasher, Debug, PartialEq)]
pub struct DKGNode {
    metadata: DKGNodeMetadata,
    trx: DKGTranscriptWrapper,
}

impl DKGNode {
    pub fn new(epoch: u64, author: Author, trx: DKGTranscriptWrapper) -> Self {
        Self {
            metadata: DKGNodeMetadata { epoch, author },
            trx,
        }
    }

    pub fn metadata(&self) -> &DKGNodeMetadata {
        &self.metadata
    }

    pub fn author(&self) -> &Author {
        self.metadata.author()
    }

    pub fn epoch(&self) -> u64 {
        self.metadata.epoch
    }

    pub fn transcript(&self) -> &DKGTranscriptWrapper {
        &self.trx
    }
}

impl TDKGMessage for DKGNode {
    fn verify(
        &self,
        dkg_pvss_config: &DKGPvssConfig,
        verifier: &ValidatorVerifier,
    ) -> anyhow::Result<()> {
        self.trx.verify(dkg_pvss_config, verifier)?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DKGNodeAck {
    epoch: u64,
}

impl DKGNodeAck {
    pub fn new(epoch: u64) -> Self {
        Self { epoch }
    }
}

pub struct DKGNodeAckState {
    num_validators: usize,
    received: Mutex<HashSet<Author>>,
}

impl DKGNodeAckState {
    pub fn new(num_validators: usize) -> Self {
        Self {
            num_validators,
            received: Mutex::new(HashSet::new()),
        }
    }
}

impl BroadcastStatus<DKGMessage> for Arc<DKGNodeAckState> {
    type Aggregated = ();
    type Message = DKGNode;
    type Response = DKGNodeAck;

    fn add(&self, peer: Author, _ack: Self::Response) -> anyhow::Result<Option<Self::Aggregated>> {
        let mut validators = self.received.lock();
        validators.insert(peer);
        if validators.len() == self.num_validators {
            Ok(Some(()))
        } else {
            Ok(None)
        }
    }
}

impl TDKGMessage for DKGAggNode {
    fn verify(
        &self,
        dkg_pvss_config: &DKGPvssConfig,
        verifier: &ValidatorVerifier,
    ) -> anyhow::Result<()> {
        let dealers = self.agg_trx.verify_dealers(verifier.len())?;
        let addresses = verifier.get_ordered_account_addresses();
        let dealers_addresses = dealers
            .iter()
            .filter_map(|&pos| addresses.get(pos))
            .cloned()
            .collect::<Vec<_>>();
        // Ensure aggregated transcript has enough stakes
        verifier.check_voting_power(dealers_addresses.iter(), false)?;

        self.agg_trx.verify(dkg_pvss_config, verifier)?;

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DKGAggNodeAck {
    epoch: u64,
}

impl DKGAggNodeAck {
    pub fn new(epoch: u64) -> Self {
        Self { epoch }
    }
}

pub struct DKGAggNodeAckState {
    num_validators: usize,
    received: Mutex<HashSet<Author>>,
}

impl DKGAggNodeAckState {
    pub fn new(num_validators: usize) -> Self {
        Self {
            num_validators,
            received: Mutex::new(HashSet::new()),
        }
    }
}

impl BroadcastStatus<DKGMessage> for Arc<DKGAggNodeAckState> {
    type Aggregated = ();
    type Message = DKGAggNode;
    type Response = DKGAggNodeAck;

    fn add(&self, peer: Author, _ack: Self::Response) -> anyhow::Result<Option<Self::Aggregated>> {
        let mut validators = self.received.lock();
        validators.insert(peer);
        if validators.len() == self.num_validators {
            Ok(Some(()))
        } else {
            Ok(None)
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DKGNetworkMessage {
    pub epoch: u64,
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
}

#[derive(Clone, Serialize, Deserialize, Debug, EnumConversion)]
pub enum DKGMessage {
    DKGNodeMsg(DKGNode),
    DKGNodeAckMsg(DKGNodeAck),
    DKGAggNodeMsg(DKGAggNode),
    DKGAggNodeAckMsg(DKGAggNodeAck),
}

impl DKGMessage {
    pub fn name(&self) -> &str {
        match self {
            DKGMessage::DKGNodeMsg(_) => "DKGNodeMsg",
            DKGMessage::DKGNodeAckMsg(_) => "DKGNodeAckMsg",
            DKGMessage::DKGAggNodeMsg(_) => "DKGAggNodeMsg",
            DKGMessage::DKGAggNodeAckMsg(_) => "DKGAggNodeAckMsg",
        }
    }

    pub fn author(&self) -> anyhow::Result<Author> {
        match self {
            DKGMessage::DKGNodeMsg(node) => Ok(node.metadata.author),
            DKGMessage::DKGAggNodeMsg(node) => Ok(node.metadata.author),
            _ => bail!("message does not support author field"),
        }
    }
}

impl RBMessage for DKGMessage {}

impl DKGMessage {
    pub fn epoch(&self) -> u64 {
        match self {
            DKGMessage::DKGNodeMsg(node) => node.metadata.epoch,
            DKGMessage::DKGNodeAckMsg(ack) => ack.epoch,
            DKGMessage::DKGAggNodeMsg(node) => node.metadata.epoch,
            DKGMessage::DKGAggNodeAckMsg(ack) => ack.epoch,
        }
    }
}

impl TryFrom<DKGNetworkMessage> for DKGMessage {
    type Error = anyhow::Error;

    fn try_from(msg: DKGNetworkMessage) -> Result<Self, Self::Error> {
        Ok(bcs::from_bytes(&msg.data)?)
    }
}

impl From<DKGMessage> for DKGMsg {
    fn from(value: DKGMessage) -> Self {
        Self::DKGMessage(Box::new(DKGNetworkMessage {
            epoch: value.epoch(),
            data: bcs::to_bytes(&value).unwrap(),
        }))
    }
}

impl TryFrom<DKGMsg> for DKGMessage {
    type Error = anyhow::Error;

    fn try_from(value: DKGMsg) -> Result<Self, Self::Error> {
        match value {
            DKGMsg::DKGMessage(obj) => Ok(bcs::from_bytes(&obj.data)?),
        }
    }
}
