// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, ensure};
pub use aptos_consensus_types::common::Author;
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use aptos_enum_conversion_derive::EnumConversion;
use aptos_infallible::Mutex;
use aptos_reliable_broadcast::{BroadcastStatus, RBMessage};
pub use aptos_types::dkg::DKGAggNode;
use aptos_types::{
    dkg::{DKGPvssConfig, DKGTranscriptWrapper},
    epoch_state::EpochState,
    validator_verifier::{ValidatorVerifier, VerifyError},
};
use move_core_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, sync::Arc};

pub trait TDKGMessage: Into<DKGMessage> + TryFrom<DKGMessage> {
    fn verify(
        &self,
        dkg_pvss_config: &DKGPvssConfig,
        verifier: &ValidatorVerifier,
    ) -> anyhow::Result<()>;
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
pub struct DKGNodeRequest {
    dealer_epoch: u64,
}

impl DKGNodeRequest {
    pub fn new(epoch: u64) -> Self {
        Self {
            dealer_epoch: epoch,
        }
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

#[derive(Default)]
pub struct TrxAggregator {
    pub contributors: HashSet<AccountAddress>,
    pub trx: Option<DKGTranscriptWrapper>,
}

pub struct DKGNodeAggState {
    trx_aggregator: Mutex<TrxAggregator>,
    pvss_config: DKGPvssConfig,
    epoch_state: EpochState,
    my_addr: AccountAddress,
}

impl DKGNodeAggState {
    pub fn new(
        pvss_config: DKGPvssConfig,
        epoch_state: EpochState,
        my_addr: AccountAddress,
    ) -> Self {
        Self {
            trx_aggregator: Mutex::new(TrxAggregator::default()),
            pvss_config,
            epoch_state,
            my_addr,
        }
    }
}

impl BroadcastStatus<DKGMessage> for Arc<DKGNodeAggState> {
    type Aggregated = DKGAggNode;
    type Message = DKGNodeRequest;
    type Response = DKGNode;

    fn add(&self, sender: Author, dkg_node: DKGNode) -> anyhow::Result<Option<Self::Aggregated>> {
        let DKGNode { metadata, trx } = dkg_node;
        ensure!(
            metadata.author == sender,
            "dkg node author should match sender"
        );

        let mut trx_aggregator = self.trx_aggregator.lock();
        if trx_aggregator.contributors.contains(&metadata.author) {
            return Ok(None);
        }

        trx.verify(&self.pvss_config, &self.epoch_state.verifier)?;

        // All checks passed. Aggregating.
        trx_aggregator.contributors.insert(metadata.author);
        if let Some(agg_trx) = trx_aggregator.trx.as_mut() {
            agg_trx.aggregate_with(&self.pvss_config, &trx);
        } else {
            trx_aggregator.trx = Some(trx);
        }
        let maybe_aggregated = self
            .epoch_state
            .verifier
            .check_voting_power(trx_aggregator.contributors.iter(), true)
            .ok()
            .map(|x| {
                DKGAggNode::new(
                    self.epoch_state.epoch,
                    self.my_addr,
                    trx_aggregator.trx.clone().unwrap(),
                )
            });
        Ok(maybe_aggregated)
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, EnumConversion, PartialEq)]
pub enum DKGMessage {
    NodeRequest(DKGNodeRequest),
    NodeResponse(DKGNode),
}

impl DKGMessage {
    pub fn name(&self) -> &str {
        match self {
            DKGMessage::NodeRequest(_) => "DKGNodeRequest",
            DKGMessage::NodeResponse(_) => "DKGNodeResponse",
        }
    }

    pub fn author(&self) -> anyhow::Result<Author> {
        match self {
            DKGMessage::NodeResponse(node) => Ok(node.metadata.author),
            _ => bail!("message does not support author field"),
        }
    }
}

impl RBMessage for DKGMessage {}

impl DKGMessage {
    pub fn epoch(&self) -> u64 {
        match self {
            DKGMessage::NodeRequest(req) => req.dealer_epoch,
            DKGMessage::NodeResponse(node) => node.epoch(),
        }
    }
}
